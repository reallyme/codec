// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use codec_proto::generated::proto::reallyme::codec::v1 as proto;

/// Borrowed header bytes and a generated receipt. This does not prove a value
/// is present. No Debug, cloning, or serialization of borrowed bytes is exposed.
pub struct Header<'a> {
    pub(super) identifier: &'a [u8],
    pub(super) length_octets: &'a [u8],
    pub(super) received: &'a [u8],
    pub(super) remainder: &'a [u8],
    pub(super) receipt: proto::CodecTlvHeader,
}

impl<'a> Header<'a> {
    /// Complete wire identifier, including all received continuation bits.
    #[must_use]
    pub const fn identifier_bytes(&self) -> &'a [u8] {
        self.identifier
    }
    /// Encoded length field, including the initial discriminator octet.
    #[must_use]
    pub const fn length_octets(&self) -> &'a [u8] {
        self.length_octets
    }
    /// Complete original header, without copying or reconstruction.
    #[must_use]
    pub const fn received_bytes(&self) -> &'a [u8] {
        self.received
    }
    /// All supplied bytes after the header, even when the value is incomplete.
    #[must_use]
    pub const fn supplied_remainder(&self) -> &'a [u8] {
        self.remainder
    }
    /// Declared length; it need not fit usize or the supplied input.
    #[must_use]
    pub const fn declared_value_length(&self) -> u64 {
        self.receipt.declared_value_length
    }
    /// Actual header bytes consumed from the source.
    #[must_use]
    pub const fn consumed_bytes(&self) -> usize {
        self.received.len()
    }
    /// Inspect the schema-defined metadata without copying input bytes.
    #[must_use]
    pub const fn contract(&self) -> &proto::CodecTlvHeader {
        &self.receipt
    }
    /// Move the generated receipt to operation dispatch.
    #[must_use]
    pub fn into_contract(self) -> proto::CodecTlvHeader {
        self.receipt
    }
}

/// A complete outer value borrowing the exact original encoding. Its generated
/// source spans never extend beyond supplied bytes; contents remain opaque.
pub struct Frame<'a> {
    pub(super) header: Header<'a>,
    pub(super) received: &'a [u8],
    pub(super) contents: &'a [u8],
    pub(super) received_span: proto::CodecByteSpan,
    pub(super) contents_span: proto::CodecByteSpan,
    pub(super) consumed: u64,
}

impl<'a> Frame<'a> {
    /// Header receipt with the exact received identifier and length encoding.
    #[must_use]
    pub const fn header(&self) -> &Header<'a> {
        &self.header
    }
    /// Complete original TLV, without copying or normalizing its bytes.
    #[must_use]
    pub const fn received_bytes(&self) -> &'a [u8] {
        self.received
    }
    /// Original value contents. Constructed contents are not recursively read.
    #[must_use]
    pub const fn value_bytes(&self) -> &'a [u8] {
        self.contents
    }
    /// Actual complete-value bytes consumed from the source.
    #[must_use]
    pub const fn consumed_bytes(&self) -> usize {
        self.received.len()
    }
    /// Move the generated source receipt to operation dispatch.
    #[must_use]
    pub fn into_contract(self) -> proto::CodecTlvFrame {
        proto::CodecTlvFrame {
            header: Some(self.header.into_contract()).into(),
            received: Some(self.received_span).into(),
            contents: Some(self.contents_span).into(),
            consumed_bytes: self.consumed,
        }
    }
}
