// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

use super::error::{error, CborError, ErrorReason};
use super::float::{Float, FloatWidth};
use super::limits::{Budget, Limits};
use super::profile::{Profile, Serialization};
use super::read_header::{preferred_width, ArgumentWidth, MajorType, MajorTypeCode};
use super::receipt::Frame;
use super::scan::{add, scan};

/// Borrowed encoding input. Arrays and maps retain caller order. Scalar integer
/// copies zeroize on drop; borrowed storage remains the caller's responsibility.
/// No payload-printing or cloning traits are implemented.
pub enum ValueRef<'a> {
    /// Full unsigned 64-bit integer.
    Unsigned(u64),
    /// Full unsigned argument `n` for the negative integer `-1-n`.
    Negative(u64),
    /// Borrowed byte contents.
    Bytes(&'a [u8]),
    /// Borrowed UTF-8 text.
    Text(&'a str),
    /// Ordered array elements.
    Array(&'a [ValueRef<'a>]),
    /// Ordered unique map entries; equivalence is checked before returning output.
    Map(&'a [MapEntry<'a>]),
    /// Arbitrary tag and its child; application-specific semantics remain opaque.
    Tag(u64, &'a ValueRef<'a>),
    /// Borrowed protected floating-point value.
    Float(&'a Float),
    /// Boolean.
    Bool(bool),
    /// Null.
    Null,
    /// Undefined.
    Undefined,
    /// Simple value (0..23 or 32..255); codes 24..31 are reserved.
    Simple(u8),
    /// Embed the complete received encoding verbatim, subject to the selected
    /// profile and remaining budgets. This variant never normalizes input.
    Received(&'a Frame<'a>),
}

impl Drop for ValueRef<'_> {
    fn drop(&mut self) {
        match self {
            Self::Unsigned(value) | Self::Negative(value) | Self::Tag(value, _) => value.zeroize(),
            Self::Bool(value) => value.zeroize(),
            Self::Simple(value) => value.zeroize(),
            _ => {}
        }
    }
}
impl ZeroizeOnDrop for ValueRef<'_> {}

/// One ordered pair of borrowed encoding inputs, without hidden key sorting.
pub struct MapEntry<'a> {
    key: ValueRef<'a>,
    value: ValueRef<'a>,
}

impl<'a> MapEntry<'a> {
    /// Construct a pair; the encoder validates key uniqueness across the map.
    #[must_use]
    pub const fn new(key: ValueRef<'a>, value: ValueRef<'a>) -> Self {
        Self { key, value }
    }
}
impl ZeroizeOnDrop for MapEntry<'_> {}

/// Protected general CBOR output. The initialized allocation is wiped on drop,
/// including partial output on every encoding or validation failure.
pub struct EncodedCbor {
    bytes: Zeroizing<Vec<u8>>,
}

impl EncodedCbor {
    /// Borrow the protected encoding. Copies made by the caller need an owner
    /// that provides the same memory hygiene as the original allocation.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
}
impl ZeroizeOnDrop for EncodedCbor {}

struct Encoder {
    bytes: Zeroizing<Vec<u8>>,
    length: usize,
    measure: bool,
    budget: Budget,
    profile: Profile,
}

impl Encoder {
    fn append(&mut self, bytes: &[u8]) -> Result<(), CborError> {
        let end = add(self.length, bytes.len())?;
        if end > self.budget.limits.output {
            return Err(error(ErrorReason::OutputLimit));
        }
        self.budget.charge(bytes.len())?;
        if !self.measure {
            // Preflight reserves once before any payload is copied. Reallocating
            // a populated Vec could leave sensitive bytes in the old allocation.
            if end > self.bytes.capacity() {
                return Err(error(ErrorReason::OutputLimit));
            }
            self.bytes.extend_from_slice(bytes);
        }
        self.length = end;
        Ok(())
    }

    fn argument(&mut self, major: MajorType, value: u64) -> Result<(), CborError> {
        let octets = Zeroizing::new(value.to_be_bytes());
        let (info, start) = match preferred_width(value) {
            ArgumentWidth::Inline => {
                let inline =
                    u8::try_from(value).map_err(|_| error(ErrorReason::ArithmeticOverflow))?;
                return self.append(&[major.prefix()? | inline]);
            }
            ArgumentWidth::One => (24, 7),
            ArgumentWidth::Two => (25, 6),
            ArgumentWidth::Four => (26, 4),
            ArgumentWidth::Eight => (27, 0),
            _ => return Err(error(ErrorReason::ArithmeticOverflow)),
        };
        self.append(&[major.prefix()? | info])?;
        self.append(
            octets
                .get(start..)
                .ok_or(error(ErrorReason::ArithmeticOverflow))?,
        )
    }

    fn count(&mut self, count: usize, map: bool) -> Result<u64, CborError> {
        if count > self.budget.limits.entries {
            return Err(error(ErrorReason::EntryLimit));
        }
        let children = if map {
            count
                .checked_mul(2)
                .ok_or(error(ErrorReason::ArithmeticOverflow))?
        } else {
            count
        };
        self.budget.declared_nodes(children)?;
        u64::try_from(count).map_err(|_| error(ErrorReason::ArithmeticOverflow))
    }

    fn value(&mut self, value: &ValueRef<'_>, depth: usize) -> Result<(), CborError> {
        self.budget.charge(1)?;
        if let ValueRef::Received(frame) = value {
            // Account the original structure, not a second semantic tree. Final
            // profile validation rejects incompatible widths rather than silently
            // rebuilding bytes that may already be authenticated.
            scan(
                frame.received_bytes(),
                &mut self.budget,
                depth,
                None,
                true,
                self.profile.lengths,
            )?;
            return self.append(frame.received_bytes());
        }
        self.budget.node()?;
        match value {
            ValueRef::Unsigned(value) => self.argument(MajorType::Unsigned, *value),
            ValueRef::Negative(value) => self.argument(MajorType::Negative, *value),
            ValueRef::Bytes(bytes) => self.bytes_value(bytes, false),
            ValueRef::Text(text) => self.bytes_value(text.as_bytes(), true),
            ValueRef::Array(values) => {
                let depth = self.budget.enter(depth)?;
                let count = self.count(values.len(), false)?;
                self.argument(MajorType::Array, count)?;
                for value in *values {
                    self.value(value, depth)?;
                }
                Ok(())
            }
            ValueRef::Map(entries) => {
                let depth = self.budget.enter(depth)?;
                let count = self.count(entries.len(), true)?;
                self.argument(MajorType::Map, count)?;
                for entry in *entries {
                    self.value(&entry.key, depth)?;
                    self.value(&entry.value, depth)?;
                }
                Ok(())
            }
            ValueRef::Tag(tag, child) => {
                let depth = self.budget.enter(depth)?;
                self.argument(MajorType::Tag, *tag)?;
                self.value(child, depth)
            }
            ValueRef::Float(value) => self.float_value(value),
            ValueRef::Bool(value) => self.append(&[if *value { 0xf5 } else { 0xf4 }]),
            ValueRef::Null => self.append(&[0xf6]),
            ValueRef::Undefined => self.append(&[0xf7]),
            ValueRef::Simple(value) => self.simple_value(*value),
            ValueRef::Received(_) => Err(error(ErrorReason::WrongType)),
        }
    }
    fn contract_value(
        &mut self,
        node: &codec_proto::generated::proto::reallyme::codec::v1::CodecGeneralCborValue,
        depth: usize,
    ) -> Result<(), CborError> {
        use codec_proto::generated::proto::reallyme::codec::v1::codec_general_cbor_value::Value;

        let value = node.value.as_ref().ok_or(error(ErrorReason::WrongType))?;
        self.budget.charge(1)?;
        if let Value::ReceivedValue(value) = value {
            // Native Received holds a Frame, which proves exact framing and
            // the hard input ceiling. Re-establish that proof for wire bytes
            // before they can be embedded in a constructed container.
            if value.value.is_empty() {
                return Err(error(ErrorReason::EmptyInput));
            }
            if value.value.len() > super::limits::MAX_BYTES {
                return Err(error(ErrorReason::InputLimit));
            }
            let received = scan(
                &value.value,
                &mut self.budget,
                depth,
                None,
                true,
                self.profile.lengths,
            )?;
            if received.received_bytes().len() != value.value.len() {
                return Err(error(ErrorReason::TrailingData));
            }
            return self.append(&value.value);
        }
        self.budget.node()?;
        match value {
            Value::ArrayValue(array) => {
                let depth = self.budget.enter(depth)?;
                let count = self.count(array.values.len(), false)?;
                self.argument(MajorType::Array, count)?;
                for child in &array.values {
                    self.contract_value(child, depth)?;
                }
                Ok(())
            }
            Value::MapValue(map) => {
                let depth = self.budget.enter(depth)?;
                let count = self.count(map.entries.len(), true)?;
                self.argument(MajorType::Map, count)?;
                for entry in &map.entries {
                    self.contract_value(
                        entry.key.as_option().ok_or(error(ErrorReason::WrongType))?,
                        depth,
                    )?;
                    self.contract_value(
                        entry
                            .value
                            .as_option()
                            .ok_or(error(ErrorReason::WrongType))?,
                        depth,
                    )?;
                }
                Ok(())
            }
            Value::TagValue(tag) => {
                let depth = self.budget.enter(depth)?;
                self.argument(MajorType::Tag, tag.tag)?;
                self.contract_value(
                    tag.value.as_option().ok_or(error(ErrorReason::WrongType))?,
                    depth,
                )
            }
            // Scalar transport nodes enter the native scalar writer without
            // charging a second structural node or introducing payload copies.
            value => self.contract_scalar(value),
        }
    }

    fn contract_scalar(
        &mut self,
        value: &codec_proto::generated::proto::reallyme::codec::v1::codec_general_cbor_value::Value,
    ) -> Result<(), CborError> {
        use super::encode_contract::float;
        use codec_proto::generated::proto::reallyme::codec::v1::codec_general_cbor_value::Value;
        match value {
            Value::UnsignedValue(v) => self.argument(MajorType::Unsigned, v.argument),
            Value::NegativeValue(v) => self.argument(MajorType::Negative, v.argument),
            Value::BytesValue(v) => self.bytes_value(&v.value, false),
            Value::TextValue(v) => self.bytes_value(v.value.as_bytes(), true),
            Value::FloatValue(v) => self.float_value(&float(v)?),
            Value::BoolValue(v) => self.append(&[if v.value { 0xf5 } else { 0xf4 }]),
            Value::NullValue(_) => self.append(&[0xf6]),
            Value::UndefinedValue(_) => self.append(&[0xf7]),
            Value::SimpleValue(v) => self.simple_value(
                u8::try_from(v.value).map_err(|_| error(ErrorReason::InvalidSimpleValue))?,
            ),
            _ => Err(error(ErrorReason::WrongType)),
        }
    }

    fn bytes_value(&mut self, bytes: &[u8], text: bool) -> Result<(), CborError> {
        self.budget.contents(bytes.len(), text)?;
        self.argument(
            if text {
                MajorType::Text
            } else {
                MajorType::Bytes
            },
            u64::try_from(bytes.len()).map_err(|_| error(ErrorReason::ArithmeticOverflow))?,
        )?;
        self.append(bytes)
    }

    fn simple_value(&mut self, value: u8) -> Result<(), CborError> {
        match value {
            0..=23 => self.append(&[0xe0 | value]),
            32..=255 => self.append(&[0xf8, value]),
            _ => Err(error(ErrorReason::InvalidSimpleValue)),
        }
    }

    fn float_value(&mut self, value: &Float) -> Result<(), CborError> {
        value.check(self.profile.floats)?;
        let width = if self.profile.serialization == Serialization::Preferred {
            value.shortest()
        } else {
            value.width()
        };
        let bits = Zeroizing::new(value.at_width(width)?.to_be_bytes());
        let (header, start) = match width {
            FloatWidth::Binary16 => (0xf9, 6),
            FloatWidth::Binary32 => (0xfa, 4),
            FloatWidth::Binary64 => (0xfb, 0),
            _ => return Err(error(ErrorReason::UnsupportedPolicy)),
        };
        self.append(&[header])?;
        self.append(
            bits.get(start..)
                .ok_or(error(ErrorReason::ArithmeticOverflow))?,
        )
    }
}

/// Encode general CBOR into protected output, preserving map order and received
/// items. Constructed arguments use shortest widths; the selected profile
/// controls float shortening, validity, and optional key-order enforcement.
pub fn encode(
    value: &ValueRef<'_>,
    profile: Profile,
    limits: Limits,
) -> Result<EncodedCbor, CborError> {
    encode_with(profile, limits, |encoder| encoder.value(value, 0))
}

pub(super) fn encode_generated(
    value: &codec_proto::generated::proto::reallyme::codec::v1::CodecGeneralCborValue,
    profile: Profile,
    limits: Limits,
) -> Result<EncodedCbor, CborError> {
    encode_with(profile, limits, |encoder| encoder.contract_value(value, 0))
}

fn encode_with(
    profile: Profile,
    limits: Limits,
    visit: impl Fn(&mut Encoder) -> Result<(), CborError>,
) -> Result<EncodedCbor, CborError> {
    profile.ensure_supported()?;
    let mut measure = Encoder {
        bytes: Zeroizing::new(Vec::new()),
        length: 0,
        measure: true,
        budget: Budget::new(limits),
        profile,
    };
    visit(&mut measure)?;
    let mut bytes = Zeroizing::new(Vec::new());
    bytes
        .try_reserve_exact(measure.length)
        .map_err(|_| error(ErrorReason::AllocationFailure))?;
    let mut budget = Budget::new(limits);
    budget.work = measure.budget.work;
    let mut encoder = Encoder {
        bytes,
        length: 0,
        measure: false,
        budget,
        profile,
    };
    visit(&mut encoder)?;
    // All passes share work credits. Structural totals are checked separately
    // per pass so the published node/content limits describe the actual output.
    let mut validation = Budget::new(limits);
    validation.work = encoder.budget.work;
    {
        let validated = scan(
            &encoder.bytes,
            &mut validation,
            0,
            Some(profile),
            true,
            profile.lengths,
        )?;
        if validated.received_bytes().len() != encoder.bytes.len() {
            return Err(error(ErrorReason::TrailingData));
        }
    }
    Ok(EncodedCbor {
        bytes: encoder.bytes,
    })
}
