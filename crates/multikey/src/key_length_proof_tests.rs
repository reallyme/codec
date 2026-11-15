// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{validate_key_length, KeyLength, KeyMaterialKind, MultikeyError};
use codec_multicodec::{CodecLookupResult, CodecTag};

#[kani::proof]
#[kani::unwind(24)]
fn fixed_public_key_lengths_and_the_rsa_cap_are_enforced() {
    const RSA_LIMIT: usize = 4_096;
    let rsa: bool = kani::any();
    let expected: usize = kani::any();
    let length: usize = kani::any();
    let (name, algorithm, prefix, policy) = if rsa {
        ("rsa-pub", "RSA", &[0x85, 0x24][..], KeyLength::Variable)
    } else {
        (
            "ed25519-pub",
            "Ed25519",
            &[0xed, 0x01][..],
            KeyLength::Fixed(expected),
        )
    };
    let found = CodecLookupResult {
        name,
        tag: CodecTag::Key,
        key_material: KeyMaterialKind::PublicKey,
        alg: algorithm,
        codec: prefix,
        key_length: policy,
    };

    let result = validate_key_length(&found, length);
    match policy {
        KeyLength::Fixed(expected) if length != expected => {
            assert!(matches!(
                result,
                Err(MultikeyError::KeyLengthMismatch { expected: min, actual, .. })
                    if min == expected && actual == length
            ));
        }
        KeyLength::Fixed(_) => {
            assert!(result.is_ok());
        }
        KeyLength::Variable => {
            // RSA is the registered variable-length public key.
            if length == 0 {
                assert!(matches!(result, Err(MultikeyError::EmptyKey)));
            } else if length > RSA_LIMIT {
                assert!(matches!(
                    result, Err(MultikeyError::KeyTooLarge { max: RSA_LIMIT, actual, .. }) if actual == length
                ));
            } else {
                assert!(result.is_ok());
            }
        }
        KeyLength::NotApplicable => {
            kani::assert(false, "public-key metadata declares a length policy")
        }
    }
    kani::cover!(
        matches!(policy, KeyLength::Fixed(expected) if length == expected),
        "exact fixed size"
    );
    kani::cover!(
        matches!(policy, KeyLength::Fixed(expected) if length > expected),
        "excess fixed-size bytes"
    );
    kani::cover!(
        policy == KeyLength::Variable && length == 0,
        "empty RSA key"
    );
    kani::cover!(
        policy == KeyLength::Variable && length == RSA_LIMIT,
        "RSA cap"
    );
    kani::cover!(
        policy == KeyLength::Variable && length > RSA_LIMIT,
        "RSA exceeds cap"
    );
}
