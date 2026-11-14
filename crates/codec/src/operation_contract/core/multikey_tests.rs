// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(clippy::unwrap_used)]

use codec_multicodec::KeyLength;
use codec_multikey::encode_multikey;

use super::{parse_multikey, parse_primitive_multikey, MultikeyOperationError};

#[test]
fn parse_multikey_preserves_primitive_parse_semantics() {
    let public_key = [7_u8; 32];
    let multikey = encode_multikey("ed25519-pub", &public_key).unwrap();
    let primitive = parse_primitive_multikey(&multikey).unwrap();
    let parsed = parse_multikey(&multikey).unwrap();

    assert_eq!(parsed.codec_name(), primitive.codec_name());
    assert_eq!(parsed.algorithm_name(), primitive.algorithm_name());
    assert_eq!(parsed.public_key(), primitive.public_key());
    assert_eq!(
        parsed.expected_public_key_length(),
        Some(match primitive.key_length() {
            KeyLength::Fixed(length) => length,
            KeyLength::Variable | KeyLength::NotApplicable => 0,
        })
    );
    assert!(!parsed.variable_public_key_length());
}

#[test]
fn parse_multikey_represents_variable_public_key_length_explicitly() {
    let public_key = [9_u8; 80];
    let multikey = encode_multikey("rsa-pub", &public_key).unwrap();
    let parsed = parse_multikey(&multikey).unwrap();

    assert_eq!(parsed.codec_name(), "rsa-pub");
    assert_eq!(parsed.algorithm_name(), "RSA");
    assert_eq!(parsed.public_key(), public_key);
    assert_eq!(parsed.expected_public_key_length(), None);
    assert!(parsed.variable_public_key_length());
}

#[test]
fn parse_multikey_maps_noncanonical_multibase_to_invalid_multikey() {
    let error = match parse_multikey("not-a-key") {
        Ok(_) => MultikeyOperationError::OperationInvariant,
        Err(error) => error,
    };

    assert_eq!(error, MultikeyOperationError::InvalidMultikey);
}

#[test]
fn parse_multikey_maps_unknown_prefix_to_unknown_codec() {
    let multikey = codec_multibase::bytes_to_multibase58btc(&[0, 0, 7]).unwrap();
    let error = match parse_multikey(&multikey) {
        Ok(_) => MultikeyOperationError::OperationInvariant,
        Err(error) => error,
    };

    assert_eq!(error, MultikeyOperationError::UnknownCodec);
}

impl super::ParsedMultikey {
    /// Return the public key material with the multicodec prefix stripped.
    pub fn public_key(&self) -> &[u8] {
        self.public_key.as_slice()
    }
}
