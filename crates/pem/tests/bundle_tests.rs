// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(clippy::unwrap_used, missing_docs)]
use base64::{engine::general_purpose::STANDARD, Engine as _};
use codec_pem::{
    decode_pem_bundle, CodecPemBundleDecodeOptions, CodecPemDecodeScope, CodecPemSyntaxProfile,
    PemError,
};

fn compatible() -> CodecPemBundleDecodeOptions {
    CodecPemBundleDecodeOptions::default()
        .with_syntax_profile(CodecPemSyntaxProfile::CODEC_PEM_SYNTAX_PROFILE_COMPATIBLE)
}
fn parity(input: &[u8]) {
    let original = pem::parse_many(input);
    let projected = decode_pem_bundle(input, &compatible());
    assert_eq!(original.is_ok(), projected.is_ok(), "{input:?}");
    if let (Ok(original), Ok(projected)) = (original, projected) {
        assert_eq!(original.len(), projected.blocks.len());
        for (old, new) in original.iter().zip(projected.blocks.iter()) {
            assert_eq!(old.tag(), new.label.as_ref());
            assert_eq!(old.contents(), new.der.as_ref());
        }
    }
}

#[test]
fn compatibility_matches_established_pem_admission_and_exact_der() {
    for size in 0_u8..64 {
        let der: Vec<u8> = (0..size).collect();
        let encoded = STANDARD.encode(&der);
        for label in [
            "CERTIFICATE",
            "PRIVATE KEY",
            "UNKNOWN LABEL",
            "custom label",
        ] {
            for newline in ["\n", "\r\n", "\r"] {
                let block = format!("-----BEGIN {label}-----{newline}{encoded}{newline}-----END {label}-----{newline}");
                parity(block.as_bytes());
                parity(format!("ignored prose{newline}{block}{block}unfinished -----BEGIN CERTIFICATE-----").as_bytes());
                parity(format!("-----BEGIN {label}-----{encoded}-----END {label}-----").as_bytes());
                for space in [" ", "\t", "\u{2003}"] {
                    let spaced = encoded
                        .chars()
                        .map(|c| format!("{c}{space}"))
                        .collect::<String>();
                    parity(format!("-----BEGIN {label}-----{newline}{spaced}{newline}-----END {label}-----").as_bytes());
                }
            }
            for headers in [
                "Name: Value\n\n",
                "Name: Value\r\n\r\n",
                "Broken header\n\n",
            ] {
                parity(
                    format!("-----BEGIN {label}-----\n{headers}{encoded}\n-----END {label}-----")
                        .as_bytes(),
                );
            }
        }
    }
    for input in [
        b"not armor".as_slice(),
        b"-----BEGIN A-----!-----END A-----",
        b"-----BEGIN A-----AA==-----END B-----",
        b"-----BEGIN -----AA==-----END -----",
        b"-----BEGIN A-----AA==-----END A----",
        b"-----BEGIN A-----AA==-----END A-----\xff",
        b"------BEGIN A-----\nAA==\n-----END A-----\n",
        b"-----BEGIN A-----\nAA==\n------END A-----\n",
    ] {
        parity(input);
    }
}

#[test]
fn compatibility_preserves_partial_marker_admission() {
    for extra in 0..16 {
        let hyphens = "-".repeat(extra);
        for begin in ["-----BEGIN ", "-----BEGIN A-----"] {
            for end in ["-----END ", "-----END A-----"] {
                for prefix in ["", "prose", "\n"] {
                    let block =
                        format!("{prefix}{hyphens}{begin}A-----\nAQID\n{hyphens}{end}A-----\n");
                    parity(block.as_bytes());
                }
            }
        }
    }
}

#[test]
fn strict_profile_scope_limits_and_unknown_policy_values_fail_closed() {
    let strict = CodecPemBundleDecodeOptions::default();
    let block = b"-----BEGIN CERTIFICATE-----\nAQID\n-----END CERTIFICATE-----\n";
    assert_eq!(
        decode_pem_bundle(block, &strict).unwrap().blocks[0]
            .der
            .as_ref(),
        &[1, 2, 3]
    );
    for input in [
        b"prose\n".as_slice(),
        b"-----BEGIN CERTIFICATE-----AQID-----END CERTIFICATE-----",
        b"-----BEGIN CERTIFICATE-----\nX: Y\n\nAQID\n-----END CERTIFICATE-----",
        b"-----BEGIN CERTIFICATE-----\nAQ ID\n-----END CERTIFICATE-----",
    ] {
        assert!(decode_pem_bundle(input, &strict).is_err());
    }
    let pair = [block.as_slice(), block.as_slice()].concat();
    assert_eq!(
        decode_pem_bundle(&pair, &compatible().with_max_blocks(1)).err(),
        Some(PemError::ResourceLimitExceeded)
    );
    assert_eq!(
        decode_pem_bundle(block, &compatible().with_max_input_len(1)).err(),
        Some(PemError::InputTooLarge)
    );
    assert_eq!(
        decode_pem_bundle(block, &compatible().with_max_der_len(2)).err(),
        Some(PemError::DerTooLarge)
    );
    assert!(decode_pem_bundle(block, &compatible().with_max_der_len(3)).is_ok());
    let malformed = [
        block.as_slice(),
        b"-----BEGIN CERTIFICATE-----!-----END CERTIFICATE-----",
    ]
    .concat();
    assert_eq!(
        decode_pem_bundle(&malformed, &compatible()).err(),
        Some(PemError::InvalidBase64)
    );
    assert_eq!(
        decode_pem_bundle(
            &malformed,
            &compatible().with_scope(CodecPemDecodeScope::CODEC_PEM_DECODE_SCOPE_FIRST)
        )
        .unwrap()
        .blocks
        .len(),
        1
    );
    for options in [
        compatible().with_scope(999),
        compatible().with_syntax_profile(999),
    ] {
        assert_eq!(
            decode_pem_bundle(block, &options).err(),
            Some(PemError::InvalidOptions)
        );
    }
}

#[test]
fn strict_labels_and_line_endings_follow_the_declared_profile() {
    let options = CodecPemBundleDecodeOptions::default();
    for label in ["CERTIFICATE", "X509 CRL", "X.509-CERTIFICATE"] {
        for newline in ["\n", "\r\n", "\r"] {
            let block = format!(
                "-----BEGIN {label}-----{newline}AQID{newline}-----END {label}-----{newline}"
            );
            assert_eq!(
                decode_pem_bundle(block.as_bytes(), &options)
                    .unwrap()
                    .blocks[0]
                    .der
                    .as_ref(),
                &[1, 2, 3]
            );
        }
    }
    for label in [" A", "A ", "A--B", "A  B", "A -B", "lower"] {
        let block = format!("-----BEGIN {label}-----\nAQID\n-----END {label}-----\n");
        assert_eq!(
            decode_pem_bundle(block.as_bytes(), &options).err(),
            Some(PemError::InvalidBoundary)
        );
    }
}

#[test]
fn strict_armor_boundaries_occupy_separate_lines() {
    let options = CodecPemBundleDecodeOptions::default();
    let block = "-----BEGIN A-----\nAQID\n-----END A-----";
    for input in [
        format!(" {block}\n"),
        format!("\n\t{block}\n"),
        format!("{block}{block}\n"),
        format!("{block} {block}\n"),
        format!("{block} \t{block}\n"),
        format!("{block}\n {block}\n"),
        format!("{block} \n"),
        format!("\u{000c}{block}\n"),
        format!("\u{000c}\n{block}\n"),
        format!("\u{000b}\n{block}\n"),
    ] {
        assert_eq!(
            decode_pem_bundle(input.as_bytes(), &options).err(),
            Some(PemError::InvalidBoundary)
        );
    }
    for newline in ["\n", "\r", "\r\n"] {
        let input = format!("{newline}{block}{newline}{block}");
        assert_eq!(
            decode_pem_bundle(input.as_bytes(), &options)
                .unwrap()
                .blocks
                .len(),
            2
        );
    }
}
