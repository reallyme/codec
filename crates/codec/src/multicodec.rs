// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Semantic layer for multicodec lookup operations.
//!
//! The public protobuf, FFI, JNI, WASM, and SDK adapters are responsible for
//! transport validation and representation conversion. This module owns the
//! operation meaning so adapters do not independently classify multicodec
//! lookup failures or table metadata.

use codec_multicodec::{
    lookup_codec_prefix, CodecSpec, CodecTag as PrimitiveCodecTag,
    KeyMaterialKind as PrimitiveKeyMaterialKind, MULTICODEC_TABLE, VARIABLE_KEY_LENGTH,
};
use std::sync::OnceLock;

const MAX_U64_VARINT_BYTES: usize = 10;

static REGISTRY_VALIDITY: OnceLock<Result<(), MulticodecOperationError>> = OnceLock::new();

/// Semantic failure reasons for multicodec lookup operations.
///
/// The display strings are fixed and do not include caller input. Boundary
/// adapters map these reasons into their public typed error contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MulticodecOperationError {
    /// The requested name is not present in the supported registry.
    #[error("unknown multicodec name")]
    UnknownName,
    /// The input does not start with a supported multicodec prefix.
    #[error("invalid multicodec prefix")]
    InvalidPrefix,
    /// Primitive registry metadata violated an executor invariant.
    #[error("multicodec registry invariant violation")]
    RegistryInvariant,
    /// The bounded semantic result could not reserve its required storage.
    #[error("multicodec result allocation failed")]
    AllocationFailure,
}

/// Semantic class of a supported multicodec entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum CodecTag {
    /// Encryption-scheme identifier.
    Encryption,
    /// Hash-function identifier.
    Hash,
    /// Raw key-material identifier.
    Key,
    /// Multihash identifier.
    Multihash,
    /// Multikey-related identifier.
    Multikey,
}

/// Semantic key-material classification for a supported multicodec entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum KeyMaterialKind {
    /// The entry does not identify raw key material.
    NotKey,
    /// Public key material.
    PublicKey,
    /// Private key material.
    PrivateKey,
    /// Symmetric key material.
    SymmetricKey,
}

/// Semantic length rule for the value described by a multicodec entry.
///
/// The registry uses zero for both variable-length key material and entries
/// where a key length does not apply. Keeping those meanings distinct here
/// prevents boundary adapters from treating an algorithm identifier as a
/// variable-length key codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum MulticodecLength {
    /// Values have the exact byte length carried by the variant.
    Fixed(usize),
    /// Valid value lengths vary within limits defined by the owning codec.
    Variable,
    /// A value length is not meaningful for this registry entry.
    NotApplicable,
}

/// Borrowed semantic metadata for one supported multicodec entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MulticodecSpec<'a> {
    name: &'a str,
    tag: CodecTag,
    key_material: KeyMaterialKind,
    algorithm_name: &'a str,
    code: &'a [u8],
    prefix: &'a [u8],
    length: MulticodecLength,
}

/// Semantic result for prefix lookup by payload bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MulticodecLookup<'a> {
    name: &'a str,
    prefix_length: usize,
    metadata: MulticodecSpec<'a>,
}

/// Semantic result for the supported multicodec table.
///
/// This owner deliberately does not implement `Clone`: duplicating its vector
/// would allocate outside the fallible, typed construction path.
#[derive(Debug, PartialEq, Eq)]
pub struct MulticodecTable<'a> {
    entries: Vec<MulticodecSpec<'a>>,
}

impl<'a> MulticodecSpec<'a> {
    /// Return the canonical registry name.
    pub const fn name(&self) -> &'a str {
        self.name
    }

    /// Return the semantic registry class.
    pub const fn tag(&self) -> CodecTag {
        self.tag
    }

    /// Return the key-material classification.
    pub const fn key_material(&self) -> KeyMaterialKind {
        self.key_material
    }

    /// Return the public algorithm name.
    pub const fn algorithm_name(&self) -> &'a str {
        self.algorithm_name
    }

    /// Return the encoded multicodec varint bytes.
    pub const fn code(&self) -> &'a [u8] {
        self.code
    }

    /// Return the prefix bytes matched or prepended by codec operations.
    pub const fn prefix(&self) -> &'a [u8] {
        self.prefix
    }

    /// Return the semantic length rule for values described by this entry.
    pub const fn length(&self) -> MulticodecLength {
        self.length
    }
}

impl<'a> MulticodecLookup<'a> {
    /// Return the canonical name of the matched entry.
    pub const fn name(&self) -> &'a str {
        self.name
    }

    /// Return the number of matched prefix bytes.
    pub const fn prefix_length(&self) -> usize {
        self.prefix_length
    }

    /// Return the complete metadata for the matched entry.
    pub const fn metadata(&self) -> &MulticodecSpec<'a> {
        &self.metadata
    }
}

impl<'a> MulticodecTable<'a> {
    /// Return the supported entries in stable registry order.
    pub fn entries(&self) -> &[MulticodecSpec<'a>] {
        self.entries.as_slice()
    }
}

/// Resolve a canonical multicodec name to semantic registry metadata.
pub fn prefix_for_name(name: &str) -> Result<MulticodecSpec<'static>, MulticodecOperationError> {
    validate_registry()?;
    let Some((canonical_name, spec)) = find_codec_spec(name) else {
        return Err(MulticodecOperationError::UnknownName);
    };
    multicodec_spec(
        canonical_name,
        spec.tag,
        spec.key_material,
        spec.alg,
        spec.codec,
        spec.key_length,
    )
}

/// Resolve the known multicodec prefix at the start of `value`.
pub fn lookup_prefix(value: &[u8]) -> Result<MulticodecLookup<'static>, MulticodecOperationError> {
    validate_registry()?;
    let Some(found) = lookup_codec_prefix(value) else {
        return Err(MulticodecOperationError::InvalidPrefix);
    };
    Ok(MulticodecLookup {
        name: found.name,
        prefix_length: found.codec.len(),
        metadata: multicodec_spec(
            found.name,
            found.tag,
            found.key_material,
            found.alg,
            found.codec,
            found.key_length,
        )?,
    })
}

/// Strip a known multicodec prefix, preserving `value` when no prefix matches.
pub fn strip_prefix(value: &[u8]) -> Result<&[u8], MulticodecOperationError> {
    match lookup_prefix(value) {
        Ok(found) => value
            .get(found.prefix_length()..)
            .ok_or(MulticodecOperationError::RegistryInvariant),
        Err(MulticodecOperationError::InvalidPrefix) => Ok(value),
        Err(error) => Err(error),
    }
}

/// Return all supported multicodec entries in stable registry order.
pub fn supported_table() -> Result<MulticodecTable<'static>, MulticodecOperationError> {
    validate_registry()?;
    let mut entries = table_entries_with_capacity(MULTICODEC_TABLE.len())?;
    for (name, spec) in MULTICODEC_TABLE {
        entries.push(multicodec_spec(
            name,
            spec.tag,
            spec.key_material,
            spec.alg,
            spec.codec,
            spec.key_length,
        )?);
    }
    Ok(MulticodecTable { entries })
}

fn table_entries_with_capacity(
    capacity: usize,
) -> Result<Vec<MulticodecSpec<'static>>, MulticodecOperationError> {
    let mut entries = Vec::new();
    entries
        .try_reserve(capacity)
        .map_err(|_| MulticodecOperationError::AllocationFailure)?;
    Ok(entries)
}

fn find_codec_spec(codec_name: &str) -> Option<(&'static str, &'static CodecSpec)> {
    MULTICODEC_TABLE
        .iter()
        .find(|(name, _)| *name == codec_name)
        .map(|(name, spec)| (*name, spec))
}

fn validate_registry() -> Result<(), MulticodecOperationError> {
    *REGISTRY_VALIDITY.get_or_init(validate_registry_uncached)
}

fn validate_registry_uncached() -> Result<(), MulticodecOperationError> {
    validate_registry_entries(MULTICODEC_TABLE)
}

fn validate_registry_entries(
    entries: &[(&str, CodecSpec)],
) -> Result<(), MulticodecOperationError> {
    if entries.is_empty() {
        return Err(MulticodecOperationError::RegistryInvariant);
    }
    for (index, (name, spec)) in entries.iter().enumerate() {
        if name.is_empty() || spec.alg.is_empty() || !is_canonical_u64_varint(spec.codec) {
            return Err(MulticodecOperationError::RegistryInvariant);
        }

        let tag = semantic_codec_tag(spec.tag)?;
        let key_material = semantic_key_material_kind(spec.key_material)?;
        let is_key_tag = tag == CodecTag::Key;
        let carries_key_material = key_material != KeyMaterialKind::NotKey;
        if is_key_tag != carries_key_material {
            return Err(MulticodecOperationError::RegistryInvariant);
        }

        let next_index = index
            .checked_add(1)
            .ok_or(MulticodecOperationError::RegistryInvariant)?;
        let remaining = entries
            .get(next_index..)
            .ok_or(MulticodecOperationError::RegistryInvariant)?;
        for (other_name, other_spec) in remaining {
            let ambiguous_prefix = spec.codec.starts_with(other_spec.codec)
                || other_spec.codec.starts_with(spec.codec);
            if name == other_name || ambiguous_prefix {
                return Err(MulticodecOperationError::RegistryInvariant);
            }
        }
    }
    Ok(())
}

fn is_canonical_u64_varint(value: &[u8]) -> bool {
    if value.len() > MAX_U64_VARINT_BYTES {
        return false;
    }
    let Some((&last, leading)) = value.split_last() else {
        return false;
    };
    if last & 0x80 != 0 || leading.iter().any(|byte| byte & 0x80 == 0) {
        return false;
    }
    if !leading.is_empty() && last & 0x7f == 0 {
        return false;
    }
    value.len() < MAX_U64_VARINT_BYTES || last <= 1
}

fn multicodec_spec(
    name: &'static str,
    tag: PrimitiveCodecTag,
    key_material: PrimitiveKeyMaterialKind,
    algorithm_name: &'static str,
    prefix: &'static [u8],
    key_length: usize,
) -> Result<MulticodecSpec<'static>, MulticodecOperationError> {
    let tag = semantic_codec_tag(tag)?;
    let key_material = semantic_key_material_kind(key_material)?;
    let length = if key_length != VARIABLE_KEY_LENGTH {
        MulticodecLength::Fixed(key_length)
    } else if key_material == KeyMaterialKind::NotKey {
        MulticodecLength::NotApplicable
    } else {
        MulticodecLength::Variable
    };
    Ok(MulticodecSpec {
        name,
        tag,
        key_material,
        algorithm_name,
        code: prefix,
        prefix,
        length,
    })
}

fn semantic_codec_tag(tag: PrimitiveCodecTag) -> Result<CodecTag, MulticodecOperationError> {
    match tag {
        PrimitiveCodecTag::Encryption => Ok(CodecTag::Encryption),
        PrimitiveCodecTag::Hash => Ok(CodecTag::Hash),
        PrimitiveCodecTag::Key => Ok(CodecTag::Key),
        PrimitiveCodecTag::Multihash => Ok(CodecTag::Multihash),
        PrimitiveCodecTag::Multikey => Ok(CodecTag::Multikey),
        _ => Err(MulticodecOperationError::RegistryInvariant),
    }
}

fn semantic_key_material_kind(
    kind: PrimitiveKeyMaterialKind,
) -> Result<KeyMaterialKind, MulticodecOperationError> {
    match kind {
        PrimitiveKeyMaterialKind::NotKey => Ok(KeyMaterialKind::NotKey),
        PrimitiveKeyMaterialKind::PublicKey => Ok(KeyMaterialKind::PublicKey),
        PrimitiveKeyMaterialKind::PrivateKey => Ok(KeyMaterialKind::PrivateKey),
        PrimitiveKeyMaterialKind::SymmetricKey => Ok(KeyMaterialKind::SymmetricKey),
        _ => Err(MulticodecOperationError::RegistryInvariant),
    }
}

#[cfg(test)]
mod tests;
