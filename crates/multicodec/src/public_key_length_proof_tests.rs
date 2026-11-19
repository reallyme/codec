// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{validate_key_length, CodecPrefixError, KeyLength, MAX_RSA_PUBLIC_KEY_DER_LEN};

#[kani::proof]
#[kani::unwind(24)]
fn fixed_public_key_lengths_and_the_rsa_cap_are_enforced() {
    let rsa: bool = kani::any();
    let expected: usize = kani::any();
    let length: usize = kani::any();
    let (name, policy) = if rsa {
        ("rsa-pub", KeyLength::Variable)
    } else {
        ("ed25519-pub", KeyLength::Fixed(expected))
    };
    let result = validate_key_length(name, policy, length);
    match policy {
        KeyLength::Fixed(expected) if length != expected => {
            assert_eq!(result, Err(CodecPrefixError::InvalidLength));
        }
        KeyLength::Fixed(_) => assert!(result.is_ok()),
        KeyLength::Variable if length == 0 => {
            assert_eq!(result, Err(CodecPrefixError::InvalidLength));
        }
        KeyLength::Variable if length > MAX_RSA_PUBLIC_KEY_DER_LEN => assert_eq!(
            result,
            Err(CodecPrefixError::KeyTooLarge {
                max: MAX_RSA_PUBLIC_KEY_DER_LEN,
                actual: length,
            })
        ),
        KeyLength::Variable => assert!(result.is_ok()),
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
        policy == KeyLength::Variable && length == MAX_RSA_PUBLIC_KEY_DER_LEN,
        "RSA cap"
    );
    kani::cover!(
        policy == KeyLength::Variable && length > MAX_RSA_PUBLIC_KEY_DER_LEN,
        "RSA exceeds cap"
    );
}
