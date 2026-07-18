// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Multicodec prefix table and lookup for tagging key and hash material with its algorithm.

mod lookup;
mod table;

pub use lookup::{lookup_codec_prefix, strip_codec_prefix, CodecLookupResult};
pub use table::{
    CodecSpec, CodecTag, KeyMaterialKind, FIXED_LENGTH_NOT_APPLICABLE, MULTICODEC_TABLE,
    VARIABLE_KEY_LENGTH,
};
