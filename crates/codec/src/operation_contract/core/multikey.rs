// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Operation-specific semantic layer for structured multikey parsing.
//!
//! Transport adapters own protobuf, FFI, JNI, WASM, and SDK validation. This
//! module owns the structured parse operation meaning so those adapters do not
//! independently classify primitive multikey failures or expose primitive
//! representation details.

use codec_multicodec::VARIABLE_KEY_LENGTH;
use codec_multikey::{parse_multikey as parse_primitive_multikey, MultikeyError};

/// Semantic failure reasons for structured multikey parsing.
///
/// The display strings are fixed and do not include caller input. Boundary
/// adapters map these reasons into their public typed error contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MultikeyOperationError {
    /// The multikey payload used a multicodec prefix outside the supported set.
    #[error("unknown multikey codec")]
    UnknownCodec,
    /// The multikey input is malformed, non-canonical, or violates key limits.
    #[error("invalid multikey")]
    InvalidMultikey,
    /// The primitive parser returned a failure outside the structured contract.
    #[error("multikey operation invariant violation")]
    OperationInvariant,
}

/// Parsed multikey data for structured operation adapters.
///
/// The public key bytes are caller-provided public material, not secret key
/// material. The type still avoids derived `Debug` so future key families do
/// not accidentally expose bytes through formatting if sensitivity changes.
pub struct ParsedMultikey {
    codec_name: &'static str,
    algorithm_name: &'static str,
    public_key: Vec<u8>,
    expected_public_key_length: Option<usize>,
}

impl ParsedMultikey {
    /// Return the canonical multicodec name for the parsed public key.
    pub const fn codec_name(&self) -> &'static str {
        self.codec_name
    }

    /// Return the public algorithm name implied by the multicodec.
    pub const fn algorithm_name(&self) -> &'static str {
        self.algorithm_name
    }

    /// Return the public key material with the multicodec prefix stripped.
    #[cfg(test)]
    pub fn public_key(&self) -> &[u8] {
        self.public_key.as_slice()
    }

    /// Consume the parsed value and return the public key material.
    pub fn into_public_key(self) -> Vec<u8> {
        self.public_key
    }

    /// Return the fixed public-key length, or `None` for variable-length keys.
    pub const fn expected_public_key_length(&self) -> Option<usize> {
        self.expected_public_key_length
    }

    /// Return whether the key family accepts variable-length public material.
    pub const fn variable_public_key_length(&self) -> bool {
        self.expected_public_key_length.is_none()
    }
}

/// Parse a canonical multibase multikey into structured semantic data.
pub fn parse_multikey(multikey: &str) -> Result<ParsedMultikey, MultikeyOperationError> {
    let parsed = parse_primitive_multikey(multikey).map_err(multikey_operation_error)?;
    let expected_public_key_length = if parsed.key_length == VARIABLE_KEY_LENGTH {
        None
    } else {
        Some(parsed.key_length)
    };

    Ok(ParsedMultikey {
        codec_name: parsed.codec_name,
        algorithm_name: parsed.alg,
        public_key: parsed.public_key,
        expected_public_key_length,
    })
}

fn multikey_operation_error(error: MultikeyError) -> MultikeyOperationError {
    match error {
        MultikeyError::UnknownCodecPrefix | MultikeyError::UnknownCodecName { .. } => {
            MultikeyOperationError::UnknownCodec
        }
        MultikeyError::InvalidMultibase
        | MultikeyError::DecodedTooShort(_)
        | MultikeyError::KeyLengthMismatch { .. }
        | MultikeyError::KeyTooLarge { .. }
        | MultikeyError::EncodedPayloadTooLarge
        | MultikeyError::BindingTypeCodecMismatch { .. }
        | MultikeyError::BindingAlgorithmMismatch { .. }
        | MultikeyError::BindingAlgorithmMissing { .. } => MultikeyOperationError::InvalidMultikey,
        _ => MultikeyOperationError::OperationInvariant,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use codec_multikey::encode_multikey;

    use super::{parse_multikey, parse_primitive_multikey, MultikeyOperationError};

    #[test]
    fn parse_multikey_preserves_primitive_parse_semantics() {
        let public_key = [7_u8; 32];
        let multikey = encode_multikey("ed25519-pub", &public_key).unwrap();
        let primitive = parse_primitive_multikey(&multikey).unwrap();
        let parsed = parse_multikey(&multikey).unwrap();

        assert_eq!(parsed.codec_name(), primitive.codec_name);
        assert_eq!(parsed.algorithm_name(), primitive.alg);
        assert_eq!(parsed.public_key(), primitive.public_key.as_slice());
        assert_eq!(
            parsed.expected_public_key_length(),
            Some(primitive.key_length)
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
}
