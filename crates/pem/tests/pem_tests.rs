// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use codec_pem::{
    decode_pem, encode_pem, PemDecodePolicy, PemEncodeOptions, PemError, PemLabel, PemLineEnding,
};

const DER: &[u8] = b"not really der, only armor bytes";

#[test]
fn empty_inputs_and_invalid_options_have_distinct_reasons() {
    assert!(matches!(
        decode_pem("", PemDecodePolicy::default()),
        Err(PemError::EmptyInput)
    ));
    assert_eq!(
        encode_pem(PemLabel::PublicKey, &[], PemEncodeOptions::default()).unwrap_err(),
        PemError::EmptyDer
    );
    assert_eq!(
        encode_pem(
            PemLabel::PublicKey,
            DER,
            PemEncodeOptions {
                line_width: 77,
                ..Default::default()
            },
        )
        .unwrap_err(),
        PemError::InvalidOptions
    );
}

#[test]
fn encode_emits_exact_label_boundaries_and_line_width() -> Result<(), PemError> {
    let pem = encode_pem(
        PemLabel::PublicKey,
        b"abcdef",
        PemEncodeOptions {
            line_width: 4,
            ..Default::default()
        },
    )?;

    assert_eq!(
        pem.as_str(),
        "-----BEGIN PUBLIC KEY-----\nYWJj\nZGVm\n-----END PUBLIC KEY-----\n"
    );
    Ok(())
}

#[test]
fn pem_round_trips_known_labels() -> Result<(), PemError> {
    for label in [
        PemLabel::PrivateKey,
        PemLabel::EcPrivateKey,
        PemLabel::PublicKey,
    ] {
        let pem = encode_pem(label, DER, PemEncodeOptions::default())?;
        let allowed_labels = [label];
        let decoded = decode_pem(
            pem.as_str(),
            PemDecodePolicy {
                allowed_labels: &allowed_labels,
                ..PemDecodePolicy::default()
            },
        )?;
        assert_eq!(decoded.label, label);
        assert_eq!(decoded.der.as_slice(), DER);
    }
    Ok(())
}

#[test]
fn default_policy_rejects_private_key_labels() -> Result<(), PemError> {
    for label in [PemLabel::PrivateKey, PemLabel::EcPrivateKey] {
        let pem = encode_pem(label, DER, PemEncodeOptions::default())?;
        assert!(matches!(
            decode_pem(pem.as_str(), PemDecodePolicy::default()),
            Err(PemError::UnsupportedLabel)
        ));
    }
    Ok(())
}

#[test]
fn decode_accepts_crlf_line_endings() -> Result<(), PemError> {
    let pem = encode_pem(
        PemLabel::PublicKey,
        DER,
        PemEncodeOptions {
            line_ending: PemLineEnding::Crlf,
            ..Default::default()
        },
    )?;

    let decoded = decode_pem(pem.as_str(), PemDecodePolicy::default())?;
    assert_eq!(decoded.label, PemLabel::PublicKey);
    assert_eq!(decoded.der.as_slice(), DER);
    Ok(())
}

#[test]
fn decode_accepts_blank_lines_around_armor() -> Result<(), PemError> {
    let input = "\n\n-----BEGIN PUBLIC KEY-----\nYWJj\n-----END PUBLIC KEY-----\n\n";

    let decoded = decode_pem(input, PemDecodePolicy::default())?;

    assert_eq!(decoded.label, PemLabel::PublicKey);
    assert_eq!(decoded.der.as_slice(), b"abc");
    Ok(())
}

#[test]
fn decode_rejects_mismatched_label() {
    let input = "-----BEGIN PUBLIC KEY-----\nAA==\n-----END PRIVATE KEY-----\n";
    assert!(matches!(
        decode_pem(input, PemDecodePolicy::default()),
        Err(PemError::LabelMismatch)
    ));
}

#[test]
fn decode_rejects_missing_begin_and_missing_end() {
    assert!(matches!(
        decode_pem(
            "YWJj\n-----END PUBLIC KEY-----\n",
            PemDecodePolicy::default()
        ),
        Err(PemError::MissingBegin)
    ));
    assert!(matches!(
        decode_pem(
            "-----BEGIN PUBLIC KEY-----\nYWJj\n",
            PemDecodePolicy::default()
        ),
        Err(PemError::MissingEnd)
    ));
}

#[test]
fn decode_rejects_nested_begin_and_trailing_material() {
    assert!(matches!(
        decode_pem(
            "-----BEGIN PUBLIC KEY-----\n-----BEGIN PUBLIC KEY-----\nYWJj\n-----END PUBLIC KEY-----\n",
            PemDecodePolicy::default()
        ),
        Err(PemError::InvalidBoundary)
    ));
    assert!(matches!(
        decode_pem(
            "-----BEGIN PUBLIC KEY-----\nYWJj\n-----END PUBLIC KEY-----\nYWJj\n",
            PemDecodePolicy::default()
        ),
        Err(PemError::InvalidBoundary)
    ));
    assert!(matches!(
        decode_pem(
            "-----BEGIN PUBLIC KEY-----\nYWJj\n-----END PUBLIC KEY-----\n-----END PUBLIC KEY-----\n",
            PemDecodePolicy::default()
        ),
        Err(PemError::InvalidBoundary)
    ));
}

#[test]
fn decode_rejects_empty_body() {
    assert!(matches!(
        decode_pem(
            "-----BEGIN PUBLIC KEY-----\n-----END PUBLIC KEY-----\n",
            PemDecodePolicy::default()
        ),
        Err(PemError::InvalidBody)
    ));
}

#[test]
fn decode_rejects_disallowed_label() {
    let pem = encode_pem(PemLabel::PrivateKey, DER, PemEncodeOptions::default()).unwrap();
    let policy = PemDecodePolicy {
        allowed_labels: &[PemLabel::PublicKey],
        ..Default::default()
    };
    assert!(matches!(
        decode_pem(pem.as_str(), policy),
        Err(PemError::UnsupportedLabel)
    ));
}

#[test]
fn decode_rejects_oversized_input_and_der() {
    let pem = encode_pem(PemLabel::PublicKey, DER, PemEncodeOptions::default()).unwrap();
    let input_policy = PemDecodePolicy {
        max_input_len: 8,
        ..Default::default()
    };
    assert!(matches!(
        decode_pem(pem.as_str(), input_policy),
        Err(PemError::InputTooLarge)
    ));

    let der_policy = PemDecodePolicy {
        max_der_len: 1,
        ..Default::default()
    };
    assert!(matches!(
        decode_pem(pem.as_str(), der_policy),
        Err(PemError::DerTooLarge)
    ));
}

#[test]
fn decode_input_cap_accepts_exact_length_and_rejects_one_more() -> Result<(), PemError> {
    let pem = encode_pem(PemLabel::PublicKey, DER, PemEncodeOptions::default())?;
    let exact_policy = PemDecodePolicy {
        max_input_len: pem.len(),
        ..Default::default()
    };
    assert!(decode_pem(&pem, exact_policy).is_ok());

    let short_policy = PemDecodePolicy {
        max_input_len: pem.len().checked_sub(1).ok_or(PemError::InvalidOptions)?,
        ..Default::default()
    };
    assert!(matches!(
        decode_pem(&pem, short_policy),
        Err(PemError::InputTooLarge)
    ));
    Ok(())
}

#[test]
fn decode_rejects_each_invalid_policy_field_independently() {
    let input = "-----BEGIN PUBLIC KEY-----\nYWJj\n-----END PUBLIC KEY-----\n";
    assert!(matches!(
        decode_pem(
            input,
            PemDecodePolicy {
                max_der_len: 0,
                ..Default::default()
            }
        ),
        Err(PemError::InvalidOptions)
    ));
    assert!(matches!(
        decode_pem(
            input,
            PemDecodePolicy {
                allowed_labels: &[],
                ..Default::default()
            }
        ),
        Err(PemError::InvalidOptions)
    ));
}

#[test]
fn decode_encoded_and_der_caps_have_distinct_exact_boundaries() -> Result<(), PemError> {
    let policy = PemDecodePolicy {
        max_der_len: 3,
        ..Default::default()
    };
    let exact = "-----BEGIN PUBLIC KEY-----\nYWJj\n-----END PUBLIC KEY-----\n";
    assert_eq!(decode_pem(exact, policy)?.der.as_slice(), b"abc");

    // Invalid base64 beyond the encoded-length cap must be rejected by the
    // length guard before the base64 parser examines the body.
    let over_encoded = "-----BEGIN PUBLIC KEY-----\n========\n-----END PUBLIC KEY-----\n";
    assert!(matches!(
        decode_pem(over_encoded, policy),
        Err(PemError::DerTooLarge)
    ));
    Ok(())
}

#[test]
fn decode_checks_exact_der_size_after_base64_decoding() -> Result<(), PemError> {
    // Base64 rounds multiple DER lengths into the same encoded width. These
    // cases reach the post-decode bound instead of the earlier text estimate.
    for maximum in [2_usize, 4, 5] {
        let der = vec![0x30; maximum + 1];
        let pem = encode_pem(PemLabel::PublicKey, &der, PemEncodeOptions::default())?;
        let policy = PemDecodePolicy {
            max_der_len: maximum,
            ..Default::default()
        };
        assert!(matches!(
            decode_pem(pem.as_str(), policy),
            Err(PemError::DerTooLarge)
        ));
    }
    Ok(())
}

#[test]
fn decode_rejects_invalid_body_bytes() {
    let input = "-----BEGIN PUBLIC KEY-----\n@@@\n-----END PUBLIC KEY-----\n";
    assert!(matches!(
        decode_pem(input, PemDecodePolicy::default()),
        Err(PemError::InvalidBody)
    ));
}

#[test]
fn encode_rejects_invalid_options() {
    assert!(matches!(
        encode_pem(
            PemLabel::PublicKey,
            DER,
            PemEncodeOptions {
                line_width: 0,
                ..Default::default()
            }
        ),
        Err(PemError::InvalidOptions)
    ));
}
