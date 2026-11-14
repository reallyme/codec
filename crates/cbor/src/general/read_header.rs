// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use zeroize::{Zeroize, ZeroizeOnDrop};

use super::error::{error, CborError, ErrorReason};
use super::float::Float;

pub(super) const INLINE_ARGUMENT_LIMIT: u64 = 24;
const SIMPLE_EXTENDED_MINIMUM: u64 = 32;
const ADDITIONAL_INFORMATION_MASK: u8 = 0x1f;
const MAJOR_TYPE_SHIFT: u32 = 5;

/// Schema-generated CBOR major type and received argument width.
pub use codec_proto::generated::proto::reallyme::codec::v1::{
    CodecGeneralCborArgumentWidth as ArgumentWidth, CodecGeneralCborMajorType as MajorType,
};

fn decode_major_type(code: u8) -> Result<MajorType, CborError> {
    match code {
        0 => Ok(MajorType::Unsigned),
        1 => Ok(MajorType::Negative),
        2 => Ok(MajorType::Bytes),
        3 => Ok(MajorType::Text),
        4 => Ok(MajorType::Array),
        5 => Ok(MajorType::Map),
        6 => Ok(MajorType::Tag),
        7 => Ok(MajorType::Simple),
        _ => Err(error(ErrorReason::ReservedAdditionalInformation)),
    }
}

pub(super) trait MajorTypeCode {
    fn prefix(self) -> Result<u8, CborError>;
}

impl MajorTypeCode for MajorType {
    fn prefix(self) -> Result<u8, CborError> {
        match self {
            Self::Unsigned => Ok(0x00),
            Self::Negative => Ok(0x20),
            Self::Bytes => Ok(0x40),
            Self::Text => Ok(0x60),
            Self::Array => Ok(0x80),
            Self::Map => Ok(0xa0),
            Self::Tag => Ok(0xc0),
            Self::Simple => Ok(0xe0),
            _ => Err(error(ErrorReason::ReservedAdditionalInformation)),
        }
    }
}

pub(super) fn preferred_width(value: u64) -> ArgumentWidth {
    if value < INLINE_ARGUMENT_LIMIT {
        ArgumentWidth::Inline
    } else if value <= u64::from(u8::MAX) {
        ArgumentWidth::One
    } else if value <= u64::from(u16::MAX) {
        ArgumentWidth::Two
    } else if value <= u64::from(u32::MAX) {
        ArgumentWidth::Four
    } else {
        ArgumentWidth::Eight
    }
}

/// Borrowed received header. Successful header decoding makes no payload claim.
///
/// Numeric argument copies are wiped on drop. The received bytes remain owned
/// by the caller, which must provide a zeroizing owner for sensitive input.
pub struct Header<'a> {
    encoded: &'a [u8],
    major: MajorType,
    width: ArgumentWidth,
    argument: Option<u64>,
}

impl<'a> Header<'a> {
    /// Exact received header bytes, including any non-preferred width.
    #[must_use]
    pub const fn received_bytes(&self) -> &'a [u8] {
        self.encoded
    }
    /// Inspect the major type.
    #[must_use]
    pub const fn major_type(&self) -> MajorType {
        self.major
    }
    /// Inspect the received argument or float-payload width.
    #[must_use]
    pub const fn argument_width(&self) -> ArgumentWidth {
        self.width
    }
    /// Return an integer argument; floats, indefinite lengths, and breaks have none.
    #[must_use]
    pub fn argument(&self) -> Option<u64> {
        if self.is_float() {
            None
        } else {
            self.argument
        }
    }
    /// Observe shortest argument encoding independently of payload validation.
    /// Floats, indefinite lengths, and breaks return `None`.
    #[must_use]
    pub fn has_preferred_argument(&self) -> Option<bool> {
        self.argument()
            .map(|value| preferred_width(value) == self.width)
    }
    /// Observe whether a float uses the shortest width that holds its value
    /// exactly, including NaN significands. Non-float headers return `None`.
    pub fn has_preferred_float(&self) -> Result<Option<bool>, CborError> {
        Ok(self.float()?.map(|value| value.width() == value.shortest()))
    }
    /// Inspect whether this header carries IEEE floating-point bits.
    #[must_use]
    pub const fn is_float(&self) -> bool {
        matches!(self.major, MajorType::Simple)
            && matches!(
                self.width,
                ArgumentWidth::Two | ArgumentWidth::Four | ArgumentWidth::Eight
            )
    }
    /// Extract owned protected floating-point bits, if present.
    pub fn float(&self) -> Result<Option<Float>, CborError> {
        if !self.is_float() {
            return Ok(None);
        }
        let value = self.argument.ok_or(error(ErrorReason::Truncated))?;
        match self.width {
            ArgumentWidth::Two => Ok(Some(Float::binary16(
                u16::try_from(value).map_err(|_| error(ErrorReason::ArithmeticOverflow))?,
            ))),
            ArgumentWidth::Four => Ok(Some(Float::binary32(
                u32::try_from(value).map_err(|_| error(ErrorReason::ArithmeticOverflow))?,
            ))),
            ArgumentWidth::Eight => Ok(Some(Float::binary64(value))),
            _ => Err(error(ErrorReason::WrongType)),
        }
    }
    /// Extract a simple-value code; floats and breaks return `None`.
    pub fn simple_value(&self) -> Result<Option<u8>, CborError> {
        if self.major != MajorType::Simple || self.is_float() || self.width == ArgumentWidth::Break
        {
            return Ok(None);
        }
        self.argument
            .map(|value| u8::try_from(value).map_err(|_| error(ErrorReason::ArithmeticOverflow)))
            .transpose()
    }
}

impl Drop for Header<'_> {
    fn drop(&mut self) {
        self.argument.zeroize();
    }
}
impl ZeroizeOnDrop for Header<'_> {}

fn argument_bytes<const N: usize>(input: &[u8]) -> Result<[u8; N], CborError> {
    let end = 1_usize
        .checked_add(N)
        .ok_or(error(ErrorReason::ArithmeticOverflow))?;
    input
        .get(1..end)
        .ok_or(error(ErrorReason::Truncated))?
        .try_into()
        .map_err(|_| error(ErrorReason::Truncated))
}

/// Decode at most nine header bytes, without inspecting or normalizing the payload.
pub fn read_header_prefix(input: &[u8]) -> Result<Header<'_>, CborError> {
    let initial = *input.first().ok_or(error(ErrorReason::EmptyInput))?;
    let major = decode_major_type(initial >> MAJOR_TYPE_SHIFT)?;
    let info = initial & ADDITIONAL_INFORMATION_MASK;
    let (width, argument, length) = match info {
        0..=23 => (ArgumentWidth::Inline, Some(u64::from(info)), 1),
        24 => (
            ArgumentWidth::One,
            Some(u64::from(u8::from_be_bytes(argument_bytes::<1>(input)?))),
            2,
        ),
        25 => (
            ArgumentWidth::Two,
            Some(u64::from(u16::from_be_bytes(argument_bytes::<2>(input)?))),
            3,
        ),
        26 => (
            ArgumentWidth::Four,
            Some(u64::from(u32::from_be_bytes(argument_bytes::<4>(input)?))),
            5,
        ),
        27 => (
            ArgumentWidth::Eight,
            Some(u64::from_be_bytes(argument_bytes::<8>(input)?)),
            9,
        ),
        31 if matches!(
            major,
            MajorType::Bytes | MajorType::Text | MajorType::Array | MajorType::Map
        ) =>
        {
            (ArgumentWidth::Indefinite, None, 1)
        }
        31 if major == MajorType::Simple => (ArgumentWidth::Break, None, 1),
        31 => return Err(error(ErrorReason::InvalidIndefiniteArgument)),
        _ => return Err(error(ErrorReason::ReservedAdditionalInformation)),
    };
    if major == MajorType::Simple
        && width == ArgumentWidth::One
        && argument.is_some_and(|value| value < SIMPLE_EXTENDED_MINIMUM)
    {
        return Err(error(ErrorReason::InvalidSimpleValue));
    }
    let encoded = input.get(..length).ok_or(error(ErrorReason::Truncated))?;
    Ok(Header {
        encoded,
        major,
        width,
        argument,
    })
}
