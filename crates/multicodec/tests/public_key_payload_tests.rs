// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(
    missing_docs,
    clippy::unwrap_used,
    reason = "bounded public-key fixtures"
)]

use codec_multicodec::{
    strip_codec_prefix, validate_public_key_payload, CodecPrefixError, KeyLength, KeyMaterialKind,
    MAX_RSA_PUBLIC_KEY_DER_LEN, MULTICODEC_TABLE,
};

#[test]
fn stripping_and_shared_validation_require_compressed_tags_for_every_ec_codec() {
    for (name, prefix, length) in [
        ("p256-pub", [0x80, 0x24], 33),
        ("p384-pub", [0x81, 0x24], 49),
        ("p521-pub", [0x82, 0x24], 67),
        ("secp256k1-pub", [0xe7, 0x01], 33),
    ] {
        for tag in 0..=u8::MAX {
            let mut key = vec![0; length];
            key[0] = tag;
            let mut prefixed = prefix.to_vec();
            prefixed.extend_from_slice(&key);
            let validation = validate_public_key_payload(name, &key);
            let stripped = strip_codec_prefix(&prefixed);
            if matches!(tag, 0x02 | 0x03) {
                assert_eq!(validation, Ok(()), "{name}");
                assert_eq!(stripped, Ok(key.as_slice()), "{name}");
                assert_eq!(stripped.unwrap().as_ptr(), prefixed[2..].as_ptr());
            } else {
                assert_eq!(
                    validation,
                    Err(CodecPrefixError::InvalidCompressedPoint),
                    "{name}"
                );
                assert_eq!(
                    stripped,
                    Err(CodecPrefixError::InvalidCompressedPoint),
                    "{name}"
                );
            }
        }
        for wrong_length in [
            0,
            length.checked_sub(1).unwrap(),
            length.checked_add(1).unwrap(),
        ] {
            assert_eq!(
                validate_public_key_payload(name, &vec![0x02; wrong_length]),
                Err(CodecPrefixError::InvalidLength),
                "{name}"
            );
        }
    }
}

#[test]
fn rsa_payloads_are_nonempty_and_bounded_before_stripping() {
    assert_eq!(MAX_RSA_PUBLIC_KEY_DER_LEN, 4_096);
    for length in [0, 1, 4_096, 4_097, 600_000] {
        let key = vec![0x30; length];
        let mut prefixed = vec![0x85, 0x24];
        prefixed.extend_from_slice(&key);
        let expected = match length {
            0 => Err(CodecPrefixError::InvalidLength),
            1..=4_096 => Ok(()),
            actual => Err(CodecPrefixError::KeyTooLarge { max: 4_096, actual }),
        };
        assert_eq!(validate_public_key_payload("rsa-pub", &key), expected);
        assert_eq!(strip_codec_prefix(&prefixed).map(|_| ()), expected);
    }
}

#[test]
fn all_other_registered_public_key_lengths_remain_accepted() {
    for (name, spec) in MULTICODEC_TABLE {
        if spec.key_material != KeyMaterialKind::PublicKey
            || matches!(
                *name,
                "p256-pub" | "p384-pub" | "p521-pub" | "secp256k1-pub" | "rsa-pub"
            )
        {
            continue;
        }
        let KeyLength::Fixed(length) = spec.key_length else {
            continue;
        };
        // SEC1 tagging does not apply to Edwards, Montgomery, or PQ keys.
        let key = vec![0x04; length];
        assert_eq!(validate_public_key_payload(name, &key), Ok(()), "{name}");
        let mut prefixed = spec.codec.to_vec();
        prefixed.extend_from_slice(&key);
        assert_eq!(strip_codec_prefix(&prefixed), Ok(key.as_slice()), "{name}");
        assert_eq!(
            validate_public_key_payload(name, &key[..length.checked_sub(1).unwrap()]),
            Err(CodecPrefixError::InvalidLength),
            "{name}"
        );
    }
}

#[test]
fn shared_validation_rejects_unknown_and_nonpublic_codecs() {
    assert_eq!(
        validate_public_key_payload("unknown", &[0; 32]),
        Err(CodecPrefixError::InvalidPrefix)
    );
    for (name, spec) in MULTICODEC_TABLE {
        if spec.key_material != KeyMaterialKind::PublicKey {
            assert_eq!(
                validate_public_key_payload(name, &[]),
                Err(CodecPrefixError::NonPublicKeyMaterial)
            );
        }
    }
}
