// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Multikey public-key encoding: a multicodec algorithm prefix plus multibase,
//! with strict length and binding validation on decode.
//!
//! RSA public-key bytes are an opaque payload in this codec profile, bounded
//! to 4096 bytes. Parsing an RSA multikey does not validate ASN.1 DER or the
//! cryptographic key; the consuming crypto layer must do that before use.

mod binding;
mod encode;
mod error;
mod parse;

#[allow(deprecated)]
pub use binding::{
    binding_type_matches_codec, validate_key_binding, validate_multikey_binding, KeyBindingInput,
};
pub use encode::encode_multikey;
pub use error::{
    BindingAlgorithmKind, BindingTypeKind, CodecNameReason, MultikeyCodecKind, MultikeyError,
};
pub use parse::{parse_multikey, ParsedMultikey};
