// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Multicodec prefix table and lookup for tagging key and hash material with its algorithm.

mod lookup;
mod table;
mod validate_public_key_payload;

pub use lookup::{lookup_codec_prefix, strip_codec_prefix, CodecLookupResult, CodecPrefixError};
pub use table::{CodecSpec, CodecTag, KeyLength, KeyMaterialKind, MULTICODEC_TABLE};
pub use validate_public_key_payload::{validate_public_key_payload, MAX_RSA_PUBLIC_KEY_DER_LEN};
