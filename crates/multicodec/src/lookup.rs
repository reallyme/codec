// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use crate::table::{CodecTag, KeyMaterialKind, MULTICODEC_TABLE};

/// Codec metadata resolved from a multicodec prefix.
#[derive(Debug, Clone)]
pub struct CodecLookupResult {
    /// Canonical multicodec name (e.g. `ed25519-pub`).
    pub name: &'static str,
    /// Multicodec table tag.
    pub tag: CodecTag,
    /// Key-material class for `key` codecs.
    pub key_material: KeyMaterialKind,
    /// Human-readable algorithm name (e.g. `Ed25519`).
    pub alg: &'static str,
    /// The multicodec varint prefix bytes.
    pub codec: &'static [u8],
    /// Expected raw public key length after the prefix.
    pub key_length: usize,
}

/// Lookup multicodec prefix → codec metadata
pub fn lookup_codec_prefix(bytes: &[u8]) -> Option<CodecLookupResult> {
    for (name, spec) in MULTICODEC_TABLE {
        let prefix = spec.codec;
        if bytes.len() >= prefix.len() && bytes.starts_with(prefix) {
            return Some(CodecLookupResult {
                name,
                tag: spec.tag,
                key_material: spec.key_material,
                alg: spec.alg,
                codec: spec.codec,
                key_length: spec.key_length,
            });
        }
    }
    None
}

/// Strip multicodec prefix → raw key bytes
///
/// If no known prefix is found, returns the input unchanged.
pub fn strip_codec_prefix(bytes: &[u8]) -> &[u8] {
    if let Some(found) = lookup_codec_prefix(bytes) {
        &bytes[found.codec.len()..]
    } else {
        bytes
    }
}
