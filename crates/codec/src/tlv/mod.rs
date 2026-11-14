// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Bounded borrowed views of definite BER/ISO 7816 and DER headers.
//!
//! A header receipt needs no value bytes. A frame receipt proves one complete
//! outer value fits the source; its contents remain opaque, even for constructed
//! identifiers. The DER framing profile checks only the outer header. Use
//! the `der` module for schema-directed DER value and nested canonicality checks.
//!
//! Options, limits, profiles, receipts, and error reasons come from protobuf.
//! Native views borrow the caller's original bytes and share the implementation
//! used by generated operation dispatch. The caller retains ownership and must
//! use a zeroizing owner for secret input; these views cannot wipe borrowed data.

mod error;
mod frame;
mod read_header;
mod read_prefix;
mod validate_options;
mod view;

pub use codec_proto::generated::proto::reallyme::codec::v1::{
    CodecTlvLimits as Limits, CodecTlvOptions as Options, CodecTlvProfile as Profile,
};
pub use error::{ErrorReason, TlvError};
pub use frame::{frame_exact, frame_prefix};
pub use read_header::read_header;
pub use read_prefix::{read_identifier, read_length, Identifier, Length};
pub use validate_options::{
    DEFAULT_IDENTIFIER_OCTETS, DEFAULT_INPUT_BYTES, DEFAULT_LENGTH_OCTETS, MAX_IDENTIFIER_OCTETS,
    MAX_INPUT_BYTES, MAX_LENGTH_OCTETS,
};
pub use view::{Frame, Header};
