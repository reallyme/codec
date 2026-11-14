// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::error::{overflow, size};
use super::read_header::{identifier_length, measure_length, span};
use super::validate_options::ValidatedOptions;
use super::{Options, TlvError};
use codec_proto::generated::proto::reallyme::codec::v1::{CodecTlvIdentifier, CodecTlvLength};

/// Read a complete wire identifier without reading a subsequent length.
pub fn read_identifier<'a>(input: &'a [u8], options: &Options) -> Result<Identifier<'a>, TlvError> {
    let options = ValidatedOptions::for_input(input, options)?;
    let end = identifier_length(input, &options)?;
    Ok(Identifier {
        bytes: input.get(..end).ok_or_else(overflow)?,
        remainder: input.get(end..).ok_or_else(overflow)?,
        receipt: CodecTlvIdentifier {
            identifier: Some(span(0, end)?).into(),
            consumed_bytes: size(end)?,
        },
    })
}

/// Read a complete definite length without requiring any value bytes.
pub fn read_length<'a>(input: &'a [u8], options: &Options) -> Result<Length<'a>, TlvError> {
    let options = ValidatedOptions::for_input(input, options)?;
    let (end, declared) = measure_length(input, &options)?;
    Ok(Length {
        bytes: input.get(..end).ok_or_else(overflow)?,
        remainder: input.get(end..).ok_or_else(overflow)?,
        receipt: CodecTlvLength {
            length_octets: Some(span(0, end)?).into(),
            declared_value_length: *declared,
            consumed_bytes: size(end)?,
        },
    })
}

/// Complete encoded identifier and its generated source receipt.
pub struct Identifier<'a> {
    bytes: &'a [u8],
    remainder: &'a [u8],
    receipt: CodecTlvIdentifier,
}

impl<'a> Identifier<'a> {
    /// Exact source identifier, including received continuation bits.
    #[must_use]
    pub const fn identifier_bytes(&self) -> &'a [u8] {
        self.bytes
    }
    /// Supplied bytes after the identifier.
    #[must_use]
    pub const fn supplied_remainder(&self) -> &'a [u8] {
        self.remainder
    }
    /// Actual identifier bytes consumed.
    #[must_use]
    pub const fn consumed_bytes(&self) -> usize {
        self.bytes.len()
    }
    /// Borrow the generated receipt.
    #[must_use]
    pub const fn contract(&self) -> &CodecTlvIdentifier {
        &self.receipt
    }
    /// Move the generated receipt without copying the borrowed input.
    #[must_use]
    pub fn into_contract(self) -> CodecTlvIdentifier {
        self.receipt
    }
}

/// A definite length field and its generated source receipt.
pub struct Length<'a> {
    bytes: &'a [u8],
    remainder: &'a [u8],
    receipt: CodecTlvLength,
}

impl<'a> Length<'a> {
    /// Exact source length field, including its discriminator.
    #[must_use]
    pub const fn length_octets(&self) -> &'a [u8] {
        self.bytes
    }
    /// Supplied bytes after the length, even if the value is incomplete.
    #[must_use]
    pub const fn supplied_remainder(&self) -> &'a [u8] {
        self.remainder
    }
    /// Actual length field bytes consumed.
    #[must_use]
    pub const fn consumed_bytes(&self) -> usize {
        self.bytes.len()
    }
    /// Declared value length, independent of available value bytes.
    #[must_use]
    pub const fn declared_value_length(&self) -> u64 {
        self.receipt.declared_value_length
    }
    /// Borrow the generated receipt.
    #[must_use]
    pub const fn contract(&self) -> &CodecTlvLength {
        &self.receipt
    }
    /// Move the generated receipt without copying the borrowed input.
    #[must_use]
    pub fn into_contract(self) -> CodecTlvLength {
        self.receipt
    }
}
