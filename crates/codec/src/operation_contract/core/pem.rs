// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Operation-specific semantic layer for structured PEM operations.
//!
//! Boundary adapters own protobuf, FFI, JNI, WASM, and SDK representation.
//! This module owns primitive PEM error classification for structured decode
//! operations, while preserving the primitive parser's zeroizing DER owner.

use codec_pem::{
    decode_pem as decode_primitive_pem, encode_pem as encode_primitive_pem, PemDecodePolicy,
    PemEncodeOptions, PemError, PemLabel,
};
use zeroize::Zeroizing;

/// Semantic failure reasons for structured PEM decode.
///
/// The display strings are fixed and do not include PEM text, DER bytes, label
/// text from untrusted input, or backend exception text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PemOperationError {
    /// The PEM input exceeded the configured input size.
    #[error("pem input too large")]
    InputTooLarge,
    /// The decoded DER body exceeded the configured size.
    #[error("pem der too large")]
    DerTooLarge,
    /// The PEM boundaries were malformed or incomplete.
    #[error("invalid pem boundary")]
    InvalidBoundary,
    /// The BEGIN and END labels did not match.
    #[error("pem label mismatch")]
    LabelMismatch,
    /// The label is not allowed by the decode policy.
    #[error("unsupported pem label")]
    UnsupportedLabel,
    /// The PEM body is malformed.
    #[error("invalid pem body")]
    InvalidBody,
    /// The adapter supplied an invalid decode policy.
    #[error("invalid pem policy")]
    InvalidPolicy,
    /// The primitive parser returned a failure outside the structured contract.
    #[error("pem operation invariant violation")]
    OperationInvariant,
}

/// Decoded PEM data for structured operation adapters.
pub struct DecodedPem {
    label: PemLabel,
    der: Zeroizing<Vec<u8>>,
}

/// Encoded PEM armor owned by the semantic operation layer.
pub struct EncodedPem {
    pem: Zeroizing<String>,
}

impl EncodedPem {
    /// Consume the semantic owner and transfer its allocation as mutable bytes.
    ///
    /// The generated protobuf result becomes the next zeroizing owner. The
    /// conversion does not copy or reallocate the private-key armor.
    pub(crate) fn into_bytes(mut self) -> Vec<u8> {
        core::mem::take(&mut *self.pem).into_bytes()
    }
}

impl DecodedPem {
    /// Return the accepted PEM boundary label.
    pub const fn label(&self) -> PemLabel {
        self.label
    }

    /// Return the decoded DER payload.
    pub fn der(&self) -> &[u8] {
        self.der.as_slice()
    }

    /// Consume the semantic owner and transfer its DER allocation.
    ///
    /// The generated protobuf result becomes the next zeroizing owner. Moving
    /// the allocation avoids an additional secret-bearing copy and guarantees
    /// the semantic owner still wipes DER if result construction fails before
    /// this transfer occurs.
    pub(crate) fn into_der(mut self) -> Vec<u8> {
        core::mem::take(&mut *self.der)
    }
}

/// Decode PEM text armor according to an adapter-selected policy.
pub fn decode_pem(
    input: &str,
    policy: PemDecodePolicy<'_>,
) -> Result<DecodedPem, PemOperationError> {
    let decoded = decode_primitive_pem(input, policy).map_err(pem_operation_error)?;
    Ok(DecodedPem {
        label: decoded.label,
        der: decoded.der,
    })
}

/// Encode DER bytes using the typed, documented PEM policy.
pub fn encode_pem(
    label: PemLabel,
    der: &[u8],
    options: PemEncodeOptions,
) -> Result<EncodedPem, PemOperationError> {
    let pem = encode_primitive_pem(label, der, options).map_err(pem_operation_error)?;
    Ok(EncodedPem { pem })
}

fn pem_operation_error(error: PemError) -> PemOperationError {
    match error {
        PemError::InputTooLarge => PemOperationError::InputTooLarge,
        PemError::DerTooLarge => PemOperationError::DerTooLarge,
        PemError::MissingBegin | PemError::MissingEnd | PemError::InvalidBoundary => {
            PemOperationError::InvalidBoundary
        }
        PemError::LabelMismatch => PemOperationError::LabelMismatch,
        PemError::UnsupportedLabel => PemOperationError::UnsupportedLabel,
        PemError::InvalidBase64 | PemError::InvalidBody => PemOperationError::InvalidBody,
        PemError::InvalidOptions => PemOperationError::InvalidPolicy,
        _ => PemOperationError::OperationInvariant,
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use codec_pem::{encode_pem, PemDecodePolicy, PemEncodeOptions, PemLabel};

    use super::{decode_pem, decode_primitive_pem, PemOperationError};

    #[test]
    fn decode_pem_preserves_primitive_decode_semantics() {
        let der = b"not real der";
        let pem = encode_pem(PemLabel::PublicKey, der, PemEncodeOptions::default()).unwrap();
        let policy = PemDecodePolicy {
            allowed_labels: &[PemLabel::PublicKey],
            ..PemDecodePolicy::default()
        };
        let primitive = decode_primitive_pem(&pem, policy).unwrap();
        let decoded = decode_pem(&pem, policy).unwrap();

        assert_eq!(decoded.label(), primitive.label);
        assert_eq!(decoded.der(), primitive.der.as_slice());
    }

    #[test]
    fn decode_pem_maps_label_mismatch_without_input_context() {
        let pem = "-----BEGIN PUBLIC KEY-----\nAA==\n-----END PRIVATE KEY-----\n";
        let error = match decode_pem(pem, PemDecodePolicy::default()) {
            Ok(_) => PemOperationError::OperationInvariant,
            Err(error) => error,
        };

        assert_eq!(error, PemOperationError::LabelMismatch);
    }

    #[test]
    fn decode_pem_maps_unsupported_label_without_input_context() {
        let der = b"not real der";
        let pem = encode_pem(PemLabel::PublicKey, der, PemEncodeOptions::default()).unwrap();
        let policy = PemDecodePolicy {
            allowed_labels: &[PemLabel::PrivateKey],
            ..PemDecodePolicy::default()
        };
        let error = match decode_pem(&pem, policy) {
            Ok(_) => PemOperationError::OperationInvariant,
            Err(error) => error,
        };

        assert_eq!(error, PemOperationError::UnsupportedLabel);
    }
}
