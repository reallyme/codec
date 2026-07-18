// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

#![allow(clippy::unwrap_used)]

use super::{
    is_canonical_u64_varint, lookup_prefix, prefix_for_name, semantic_codec_tag,
    semantic_key_material_kind, strip_prefix, supported_table, table_entries_with_capacity,
    validate_registry_entries, CodecSpec, CodecTag, KeyMaterialKind, MulticodecLength,
    MulticodecOperationError, PrimitiveCodecTag, PrimitiveKeyMaterialKind, MULTICODEC_TABLE,
    VARIABLE_KEY_LENGTH,
};

#[test]
fn prefix_for_name_returns_canonical_metadata() {
    let spec = prefix_for_name("ed25519-pub").unwrap();

    assert_eq!(spec.name(), "ed25519-pub");
    assert_eq!(spec.tag(), CodecTag::Key);
    assert_eq!(spec.key_material(), KeyMaterialKind::PublicKey);
    assert_eq!(spec.algorithm_name(), "Ed25519");
    assert_eq!(spec.code(), &[0xed, 0x01]);
    assert_eq!(spec.prefix(), &[0xed, 0x01]);
    assert_eq!(spec.length(), MulticodecLength::Fixed(32));
}

#[test]
fn prefix_for_name_rejects_unknown_name() {
    let error = prefix_for_name("not-a-codec").unwrap_err();

    assert_eq!(error, MulticodecOperationError::UnknownName);
}

#[test]
fn lookup_prefix_returns_matched_prefix_length_and_metadata() {
    let lookup = lookup_prefix(&[0xed, 0x01, 0xaa]).unwrap();

    assert_eq!(lookup.name(), "ed25519-pub");
    assert_eq!(lookup.prefix_length(), 2);
    assert_eq!(lookup.metadata().name(), "ed25519-pub");
    assert_eq!(lookup.metadata().length(), MulticodecLength::Fixed(32));
}

#[test]
fn lookup_prefix_rejects_unknown_prefix() {
    let error = lookup_prefix(&[0, 0, 7]).unwrap_err();

    assert_eq!(error, MulticodecOperationError::InvalidPrefix);
}

#[test]
fn strip_prefix_removes_known_prefix() {
    let stripped = strip_prefix(&[0xed, 0x01, 0xaa, 0xbb]).unwrap();

    assert_eq!(stripped, &[0xaa, 0xbb]);
}

#[test]
fn strip_prefix_preserves_unknown_prefix() {
    let input = [0, 0, 7];
    let stripped = strip_prefix(&input).unwrap();

    assert_eq!(stripped, input);
}

#[test]
fn table_returns_supported_entries_without_reclassification() {
    let table = supported_table().unwrap();

    assert_eq!(table.entries().len(), MULTICODEC_TABLE.len());
    assert!(table.entries().iter().any(|entry| {
        entry.name() == "ed25519-pub" && entry.length() == MulticodecLength::Fixed(32)
    }));
    assert!(table.entries().iter().any(|entry| {
        entry.name() == "rsa-pub" && entry.length() == MulticodecLength::Variable
    }));
    assert!(table.entries().iter().any(|entry| {
        entry.name() == "aes-gcm-256" && entry.length() == MulticodecLength::NotApplicable
    }));
}

#[test]
fn table_allocation_failure_is_typed() {
    assert_eq!(
        table_entries_with_capacity(usize::MAX),
        Err(MulticodecOperationError::AllocationFailure)
    );
}

#[test]
fn every_semantic_entry_preserves_primitive_registry_metadata() {
    let table = supported_table().unwrap();
    assert_eq!(table.entries().len(), MULTICODEC_TABLE.len());

    for ((primitive_name, primitive), semantic) in MULTICODEC_TABLE.iter().zip(table.entries()) {
        let expected_length = if primitive.key_length != VARIABLE_KEY_LENGTH {
            MulticodecLength::Fixed(primitive.key_length)
        } else if primitive.key_material == PrimitiveKeyMaterialKind::NotKey {
            MulticodecLength::NotApplicable
        } else {
            MulticodecLength::Variable
        };

        assert_eq!(semantic.name(), *primitive_name);
        assert_eq!(semantic.tag(), semantic_codec_tag(primitive.tag).unwrap());
        assert_eq!(
            semantic.key_material(),
            semantic_key_material_kind(primitive.key_material).unwrap()
        );
        assert_eq!(semantic.algorithm_name(), primitive.alg);
        assert_eq!(semantic.code(), primitive.codec);
        assert_eq!(semantic.prefix(), primitive.codec);
        assert_eq!(semantic.length(), expected_length);

        let by_name = prefix_for_name(primitive_name).unwrap();
        assert_eq!(by_name, *semantic);

        let mut prefixed_value = primitive.codec.to_vec();
        prefixed_value.push(0xa5);
        let lookup = lookup_prefix(&prefixed_value).unwrap();
        assert_eq!(lookup.name(), *primitive_name);
        assert_eq!(lookup.prefix_length(), primitive.codec.len());
        assert_eq!(lookup.metadata(), semantic);
        assert_eq!(strip_prefix(&prefixed_value).unwrap(), &[0xa5]);
    }
}

#[test]
fn registry_varint_validation_rejects_ambiguous_or_out_of_range_encodings() {
    assert!(is_canonical_u64_varint(&[0]));
    assert!(is_canonical_u64_varint(&[0x80, 0x01]));
    assert!(is_canonical_u64_varint(&[
        0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01,
    ]));

    assert!(!is_canonical_u64_varint(&[]));
    assert!(!is_canonical_u64_varint(&[0x80]));
    assert!(!is_canonical_u64_varint(&[0x80, 0]));
    assert!(!is_canonical_u64_varint(&[0, 0]));
    assert!(!is_canonical_u64_varint(&[
        0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x02,
    ]));
    assert!(!is_canonical_u64_varint(&[
        0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x80, 0x01,
    ]));
}

#[test]
fn registry_validation_fails_closed_for_malformed_metadata() {
    fn spec(
        tag: PrimitiveCodecTag,
        key_material: PrimitiveKeyMaterialKind,
        algorithm: &'static str,
        prefix: &'static [u8],
    ) -> CodecSpec {
        CodecSpec {
            tag,
            key_material,
            alg: algorithm,
            codec: prefix,
            key_length: VARIABLE_KEY_LENGTH,
        }
    }

    assert_eq!(
        validate_registry_entries(&[]),
        Err(MulticodecOperationError::RegistryInvariant)
    );

    let empty_name = [(
        "",
        spec(
            PrimitiveCodecTag::Multihash,
            PrimitiveKeyMaterialKind::NotKey,
            "SHA2-256",
            &[0x12],
        ),
    )];
    assert_eq!(
        validate_registry_entries(&empty_name),
        Err(MulticodecOperationError::RegistryInvariant)
    );

    let empty_algorithm = [(
        "sha2-256",
        spec(
            PrimitiveCodecTag::Multihash,
            PrimitiveKeyMaterialKind::NotKey,
            "",
            &[0x12],
        ),
    )];
    assert_eq!(
        validate_registry_entries(&empty_algorithm),
        Err(MulticodecOperationError::RegistryInvariant)
    );

    let noncanonical_prefix = [(
        "sha2-256",
        spec(
            PrimitiveCodecTag::Multihash,
            PrimitiveKeyMaterialKind::NotKey,
            "SHA2-256",
            &[0x92, 0x00],
        ),
    )];
    assert_eq!(
        validate_registry_entries(&noncanonical_prefix),
        Err(MulticodecOperationError::RegistryInvariant)
    );

    let key_without_key_material = [(
        "bad-key",
        spec(
            PrimitiveCodecTag::Key,
            PrimitiveKeyMaterialKind::NotKey,
            "BadKey",
            &[0x21],
        ),
    )];
    assert_eq!(
        validate_registry_entries(&key_without_key_material),
        Err(MulticodecOperationError::RegistryInvariant)
    );

    let non_key_with_key_material = [(
        "bad-hash",
        spec(
            PrimitiveCodecTag::Hash,
            PrimitiveKeyMaterialKind::PublicKey,
            "BadHash",
            &[0x22],
        ),
    )];
    assert_eq!(
        validate_registry_entries(&non_key_with_key_material),
        Err(MulticodecOperationError::RegistryInvariant)
    );

    let duplicate_name = [
        (
            "duplicate",
            spec(
                PrimitiveCodecTag::Multihash,
                PrimitiveKeyMaterialKind::NotKey,
                "First",
                &[0x23],
            ),
        ),
        (
            "duplicate",
            spec(
                PrimitiveCodecTag::Multihash,
                PrimitiveKeyMaterialKind::NotKey,
                "Second",
                &[0x24],
            ),
        ),
    ];
    assert_eq!(
        validate_registry_entries(&duplicate_name),
        Err(MulticodecOperationError::RegistryInvariant)
    );

    let ambiguous_prefix = [
        (
            "short",
            spec(
                PrimitiveCodecTag::Multihash,
                PrimitiveKeyMaterialKind::NotKey,
                "Short",
                &[0x25],
            ),
        ),
        (
            "other",
            spec(
                PrimitiveCodecTag::Multihash,
                PrimitiveKeyMaterialKind::NotKey,
                "Other",
                &[0x25],
            ),
        ),
    ];
    assert_eq!(
        validate_registry_entries(&ambiguous_prefix),
        Err(MulticodecOperationError::RegistryInvariant)
    );
}
