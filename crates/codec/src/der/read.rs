// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use der_core::asn1::{AnyRef, BitStringRef, IntRef, Null, OctetStringRef, UintRef, Utf8StringRef};
use der_core::{Decode, EncodingRules, FixedTag, Header, Reader, SliceReader, Tag};

use super::error::map_backend;
use super::validate::validate_oid;
use super::{DerBitStringRef, DerError, DerErrorReason, DerIntegerRef, DerLimits, DerOidRef};

#[derive(Clone, Copy, Eq, PartialEq)]
pub(super) enum Container {
    Document,
    Sequence,
    SetOf,
}

/// Schema-directed, nonallocating DER reader with a sticky failure state.
///
/// Construct through [`super::decode_der`]. Reader state and borrowed payloads
/// deliberately have no Debug, Clone, or serialization implementation. Shared
/// element accounting is propagated through every nested container.
pub struct DerReader<'a> {
    input: &'a [u8],
    inner: SliceReader<'a>,
    limits: DerLimits,
    container: Container,
    depth: usize,
    elements: usize,
    values: usize,
    previous: Option<&'a [u8]>,
    failed: Option<DerErrorReason>,
}

impl<'a> DerReader<'a> {
    pub(super) fn new(
        input: &'a [u8],
        limits: DerLimits,
        container: Container,
        depth: usize,
        elements: usize,
    ) -> Result<Self, DerError> {
        Ok(Self {
            input,
            // Select DER explicitly even if another consumer enables BER on the
            // shared dependency through Cargo feature unification.
            inner: SliceReader::new_with_encoding_rules(input, EncodingRules::Der)
                .map_err(map_backend)?,
            limits,
            container,
            depth,
            elements,
            values: 0,
            previous: None,
            failed: None,
        })
    }

    /// Read one canonical nonnegative INTEGER as a minimal big-endian magnitude.
    ///
    /// A necessary DER sign octet is removed; zero is the nonempty slice `[0]`.
    /// This operation performs no curve-specific validation or normalization.
    pub fn read_unsigned_integer(&mut self) -> Result<&'a [u8], DerError> {
        let Some(contents_limit) = self.limits.integer_len.checked_add(1) else {
            return self.fail(DerErrorReason::ArithmeticOverflow);
        };
        let value = self.decode::<UintRef<'a>>(Some(contents_limit))?;
        let bytes = value.as_bytes();
        if bytes.len() > self.limits.integer_len {
            return self.fail(DerErrorReason::IntegerTooLarge);
        }
        Ok(bytes)
    }

    /// Read one canonical signed INTEGER, preserving its two's-complement bytes.
    pub fn read_integer(&mut self) -> Result<DerIntegerRef<'a>, DerError> {
        let value = self.decode::<IntRef<'a>>(Some(self.limits.integer_len))?;
        Ok(DerIntegerRef {
            bytes: value.as_bytes(),
        })
    }

    /// Read a canonical BOOLEAN: only `00` and `ff` are accepted value octets.
    pub fn read_boolean(&mut self) -> Result<bool, DerError> {
        self.decode::<bool>(None)
    }

    /// Read one primitive OCTET STRING without interpreting or copying its bytes.
    pub fn read_octet_string(&mut self) -> Result<&'a [u8], DerError> {
        Ok(self.decode::<&'a OctetStringRef>(None)?.as_bytes())
    }

    /// Read a primitive BIT STRING, validating its count and unused padding bits.
    ///
    /// Named-bit schemas must additionally enforce DER trailing-zero removal.
    pub fn read_bit_string(&mut self) -> Result<DerBitStringRef<'a>, DerError> {
        let value = self.decode::<BitStringRef<'a>>(None)?;
        // The foundation checks the unused-bit count, but does not reject
        // nonzero padding. X.690 11.2.1 requires those unused bits to be zero.
        if let Some(last) = value.raw_bytes().last() {
            let Some(mask) = 1_u8
                .checked_shl(u32::from(value.unused_bits()))
                .and_then(|shifted| shifted.checked_sub(1))
            else {
                return self.fail(DerErrorReason::InvalidValue);
            };
            if last & mask != 0 {
                return self.fail(DerErrorReason::NonCanonicalEncoding);
            }
        }
        Ok(DerBitStringRef {
            bytes: value.raw_bytes(),
            unused_bits: value.unused_bits(),
        })
    }

    /// Read NULL; its content length must be zero.
    pub fn read_null(&mut self) -> Result<(), DerError> {
        self.decode::<Null>(None).map(|_| ())
    }

    /// Read canonical OBJECT IDENTIFIER contents without interpreting its arcs.
    pub fn read_object_identifier(&mut self) -> Result<DerOidRef<'a>, DerError> {
        let start = self.begin(Tag::ObjectIdentifier, None)?;
        let value = match self.inner.decode::<AnyRef<'a>>() {
            Ok(value) => value,
            Err(error) => return self.record(map_backend(error)),
        };
        if let Err(error) = validate_oid(value.value()) {
            return self.record(error);
        }
        self.check_order(start)?;
        Ok(DerOidRef {
            bytes: value.value(),
        })
    }

    /// Read canonical UTF8String contents as a borrowed UTF-8 string.
    pub fn read_utf8_string(&mut self) -> Result<&'a str, DerError> {
        Ok(self.decode::<Utf8StringRef<'a>>(None)?.as_str())
    }

    /// Decode exactly the contents of one SEQUENCE using its expected schema.
    pub fn read_sequence<T>(
        &mut self,
        decode: impl FnOnce(&mut Self) -> Result<T, DerError>,
    ) -> Result<T, DerError> {
        self.read_container(Tag::Sequence, Container::Sequence, decode)
    }

    /// Decode SET OF contents, requiring lexicographically ordered DER elements.
    ///
    /// Equal encodings are permitted; the caller selects the repeated element's
    /// schema. This method does not implement ASN.1 SET component tag ordering.
    pub fn read_set_of<T>(
        &mut self,
        decode: impl FnOnce(&mut Self) -> Result<T, DerError>,
    ) -> Result<T, DerError> {
        self.read_container(Tag::Set, Container::SetOf, decode)
    }

    /// Return true only when this container is exhausted and has not failed.
    #[must_use]
    pub fn is_finished(&self) -> bool {
        self.failed.is_none() && self.inner.is_finished()
    }

    /// Require exact consumption; an ignored earlier error remains an error.
    pub fn finish(&mut self) -> Result<(), DerError> {
        if let Some(reason) = self.failed {
            return Err(DerError::new(reason));
        }
        if !self.inner.is_finished() {
            return self.fail(DerErrorReason::TrailingData);
        }
        Ok(())
    }

    fn begin(&mut self, tag: Tag, contents_limit: Option<usize>) -> Result<usize, DerError> {
        if let Some(reason) = self.failed {
            return Err(DerError::new(reason));
        }
        if self.container == Container::Document && self.values != 0 {
            return self.fail(DerErrorReason::TrailingData);
        }
        let Some(elements) = self.elements.checked_add(1) else {
            return self.fail(DerErrorReason::ArithmeticOverflow);
        };
        if elements > self.limits.elements {
            return self.fail(DerErrorReason::TooManyElements);
        }
        let header = match Header::peek(&self.inner) {
            Ok(header) => header,
            Err(error) => return self.record(map_backend(error)),
        };
        if header.tag() != tag {
            return self.fail(DerErrorReason::UnexpectedTag);
        }
        if header.is_constructed() != tag.is_constructed() {
            return self.fail(DerErrorReason::InvalidTag);
        }
        let length = match usize::try_from(header.length()) {
            Ok(length) => length,
            Err(_) => return self.fail(DerErrorReason::ArithmeticOverflow),
        };
        if contents_limit.is_some_and(|maximum| length > maximum) {
            return self.fail(DerErrorReason::IntegerTooLarge);
        }
        self.elements = elements;
        let Some(values) = self.values.checked_add(1) else {
            return self.fail(DerErrorReason::ArithmeticOverflow);
        };
        self.values = values;
        match usize::try_from(self.inner.position()) {
            Ok(position) => Ok(position),
            Err(_) => self.fail(DerErrorReason::ArithmeticOverflow),
        }
    }

    fn decode<T>(&mut self, maximum: Option<usize>) -> Result<T, DerError>
    where
        T: Decode<'a, Error = der_core::Error> + FixedTag,
    {
        let start = self.begin(T::TAG, maximum)?;
        let value = match self.inner.decode::<T>() {
            Ok(value) => value,
            Err(error) => return self.record(map_backend(error)),
        };
        self.check_order(start)?;
        Ok(value)
    }

    fn read_container<T>(
        &mut self,
        tag: Tag,
        container: Container,
        decode: impl FnOnce(&mut Self) -> Result<T, DerError>,
    ) -> Result<T, DerError> {
        let start = self.begin(tag, None)?;
        let Some(depth) = self.depth.checked_add(1) else {
            return self.fail(DerErrorReason::ArithmeticOverflow);
        };
        if depth > self.limits.nesting_depth {
            return self.fail(DerErrorReason::NestingTooDeep);
        }
        let header = match Header::decode(&mut self.inner) {
            Ok(header) => header,
            Err(error) => return self.record(map_backend(error)),
        };
        let bytes = match self.inner.read_slice(header.length()) {
            Ok(bytes) => bytes,
            Err(error) => return self.record(map_backend(error)),
        };
        let mut nested = match Self::new(bytes, self.limits, container, depth, self.elements) {
            Ok(nested) => nested,
            Err(error) => return self.record(error),
        };
        let result = decode(&mut nested);
        self.elements = nested.elements;
        if let Some(reason) = nested.failed {
            return self.fail(reason);
        }
        let value = match result {
            Ok(value) => value,
            Err(error) => return self.record(error),
        };
        if let Err(error) = nested.finish() {
            return self.record(error);
        }
        self.check_order(start)?;
        Ok(value)
    }

    fn check_order(&mut self, start: usize) -> Result<(), DerError> {
        if self.container != Container::SetOf {
            return Ok(());
        }
        let end = match usize::try_from(self.inner.position()) {
            Ok(end) => end,
            Err(_) => return self.fail(DerErrorReason::ArithmeticOverflow),
        };
        let Some(encoded) = self.input.get(start..end) else {
            return self.fail(DerErrorReason::ArithmeticOverflow);
        };
        if self.previous.is_some_and(|previous| previous > encoded) {
            return self.fail(DerErrorReason::SetOrdering);
        }
        self.previous = Some(encoded);
        Ok(())
    }

    fn record<T>(&mut self, error: DerError) -> Result<T, DerError> {
        self.fail(error.reason())
    }

    fn fail<T>(&mut self, reason: DerErrorReason) -> Result<T, DerError> {
        let first = *self.failed.get_or_insert(reason);
        Err(DerError::new(first))
    }
}
