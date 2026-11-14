// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

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

impl super::DecodedPem {
    /// Return the decoded DER payload.
    pub fn der(&self) -> &[u8] {
        self.der.as_slice()
    }
}
