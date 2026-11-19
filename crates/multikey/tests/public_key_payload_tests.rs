// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(
    missing_docs,
    clippy::unwrap_used,
    reason = "bounded public-key fixtures"
)]

use codec_multibase::bytes_to_multibase58btc;
use codec_multicodec::{strip_codec_prefix, MAX_RSA_PUBLIC_KEY_DER_LEN};
use codec_multikey::{encode_multikey, parse_multikey, MultikeyError};

#[test]
fn multikey_and_prefix_stripping_agree_for_all_supported_ec_curves() {
    for (name, prefix, length) in [
        ("p256-pub", [0x80, 0x24], 33),
        ("p384-pub", [0x81, 0x24], 49),
        ("p521-pub", [0x82, 0x24], 67),
        ("secp256k1-pub", [0xe7, 0x01], 33),
    ] {
        for tag in [0x00, 0x01, 0x02, 0x03, 0x04, 0x06, 0x07, 0xff] {
            let mut key = vec![0; length];
            key[0] = tag;
            let mut prefixed = prefix.to_vec();
            prefixed.extend_from_slice(&key);
            // Bypass the key encoder to exercise parser admission independently.
            let multibase = bytes_to_multibase58btc(&prefixed).unwrap();
            if matches!(tag, 0x02 | 0x03) {
                assert_eq!(encode_multikey(name, &key).unwrap(), multibase);
                assert_eq!(parse_multikey(&multibase).unwrap().public_key(), key);
                assert_eq!(strip_codec_prefix(&prefixed).unwrap(), key);
            } else {
                assert!(matches!(
                    encode_multikey(name, &key),
                    Err(MultikeyError::InvalidCompressedPoint)
                ));
                assert!(matches!(
                    parse_multikey(&multibase),
                    Err(MultikeyError::InvalidCompressedPoint)
                ));
                assert!(strip_codec_prefix(&prefixed).is_err());
            }
        }
        let short = vec![0x04; length.checked_sub(1).unwrap()];
        let mut prefixed = prefix.to_vec();
        prefixed.extend_from_slice(&short);
        let encoded = bytes_to_multibase58btc(&prefixed).unwrap();
        for error in [
            encode_multikey(name, &short).err().unwrap(),
            parse_multikey(&encoded).err().unwrap(),
        ] {
            assert!(
                matches!(error, MultikeyError::KeyLengthMismatch { expected, actual, .. }
                if expected == length && actual == short.len())
            );
        }
    }
}

#[test]
fn multikey_rsa_boundaries_keep_the_existing_detailed_errors() {
    let key = vec![0x30; MAX_RSA_PUBLIC_KEY_DER_LEN];
    let encoded = encode_multikey("rsa-pub", &key).unwrap();
    assert_eq!(parse_multikey(&encoded).unwrap().public_key(), key);
    for length in [4_097, 600_000] {
        assert!(matches!(
            encode_multikey("rsa-pub", &vec![0x30; length]),
            Err(MultikeyError::KeyTooLarge { max: 4_096, actual, .. }) if actual == length
        ));
    }
    assert!(matches!(
        encode_multikey("rsa-pub", &[]),
        Err(MultikeyError::EmptyKey)
    ));
    let empty = bytes_to_multibase58btc(&[0x85, 0x24]).unwrap();
    assert!(matches!(
        parse_multikey(&empty),
        Err(MultikeyError::EmptyKey)
    ));
}
