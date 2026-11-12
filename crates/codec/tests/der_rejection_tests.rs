// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Stable rejection behavior for malformed and noncanonical DER inputs.

#![cfg(feature = "der")]

#[path = "der/support/mod.rs"]
mod support;

use reallyme_codec::der::{
    decode_der, decode_unsigned_integer_pair, DerError, DerErrorReason, DerLimits,
};
use support::{pair, tlv};

#[test]
fn pair_rejects_noncanonical_and_malformed_integer_encodings() {
    for (index, (integer, reason)) in [
        (&[0x02, 0x00][..], DerErrorReason::NonCanonicalEncoding),
        (&[0x02, 1, 0x80], DerErrorReason::InvalidInteger),
        (&[0x02, 1, 0xff], DerErrorReason::InvalidInteger),
        (&[0x02, 2, 0, 0x7f], DerErrorReason::NonCanonicalEncoding),
        (&[0x02, 2, 0, 0], DerErrorReason::NonCanonicalEncoding),
        (&[0x02, 3, 0, 0, 0x80], DerErrorReason::NonCanonicalEncoding),
        (&[0x02, 0x81, 1, 1], DerErrorReason::InvalidLength),
        (&[0x02, 0x80, 1, 0, 0], DerErrorReason::IndefiniteLength),
        (&[0x22, 1, 1], DerErrorReason::InvalidTag),
        (&[0x04, 1, 1], DerErrorReason::UnexpectedTag),
    ]
    .into_iter()
    .enumerate()
    {
        let mut contents = integer.to_vec();
        contents.extend_from_slice(&[0x02, 1, 2]);
        let encoded = tlv(0x30, &contents);
        assert_eq!(
            decode_unsigned_integer_pair(&encoded, DerLimits::default())
                .err()
                .map(DerError::reason),
            Some(reason),
            "fixture {index}"
        );
    }
}

#[test]
fn pair_rejects_wrong_shape_trailing_values_and_partial_input() {
    for input in [
        &[][..],
        &[0x30, 0],
        &[0x30, 3, 0x02, 1, 1],
        &[0x31, 6, 0x02, 1, 1, 0x02, 1, 2],
        &[0x30, 9, 0x02, 1, 1, 0x02, 1, 2, 0x02, 1, 3],
        &[0x30, 6, 0x02, 1, 1, 0x02, 1, 2, 0],
        &[0x30, 0x80, 0x02, 1, 1, 0x02, 1, 2, 0, 0],
        &[0x30, 0x81, 6, 0x02, 1, 1, 0x02, 1, 2],
        &[0x30, 0x82, 0, 6, 0x02, 1, 1, 0x02, 1, 2],
        &[0x30, 0x84, 0xff, 0xff, 0xff, 0xff],
        &[0x30, 0xff],
    ] {
        assert!(decode_unsigned_integer_pair(input, DerLimits::default()).is_err());
    }
    let canonical = pair(&[0x80; 66], &[1; 66]);
    for cut in 0..canonical.len() {
        assert!(decode_unsigned_integer_pair(&canonical[..cut], DerLimits::default()).is_err());
    }
}

#[test]
fn all_noncanonical_boolean_true_octets_fail() {
    for octet in 1_u8..=254 {
        let input = [0x01, 1, octet];
        assert_eq!(
            decode_der(&input, DerLimits::default(), |r| r.read_boolean()),
            Err(DerError::new(DerErrorReason::NonCanonicalEncoding))
        );
    }
    for input in [&[0x01, 0][..], &[0x01, 2, 0, 0]] {
        assert!(decode_der(input, DerLimits::default(), |r| r.read_boolean()).is_err());
    }
}

#[test]
fn bit_strings_reject_nonzero_padding_and_invalid_counts() {
    for unused in 1_u8..=7 {
        for last in 1_u8..=255 {
            if last & (u8::MAX >> (8 - unused)) != 0 {
                let encoded = tlv(0x03, &[unused, last]);
                assert!(
                    matches!(decode_der(&encoded, DerLimits::default(), |r| r.read_bit_string()),
                    Err(error) if error.reason() == DerErrorReason::NonCanonicalEncoding)
                );
            }
        }
    }
    assert!(matches!(
        decode_der(&[0x03, 0], DerLimits::default(), |r| r.read_bit_string()),
        Err(error) if error.reason() == DerErrorReason::InvalidLength
    ));
    for input in [
        &[0x03, 0][..],
        &[0x03, 1, 1],
        &[0x03, 2, 8, 0],
        &[0x23, 1, 0],
    ] {
        assert!(decode_der(input, DerLimits::default(), |r| r.read_bit_string()).is_err());
    }
}

#[test]
fn signed_integers_reject_redundant_sign_octets() {
    for contents in [&[][..], &[0, 0], &[0, 0x7f], &[0xff, 0xff], &[0xff, 0x80]] {
        let input = tlv(0x02, contents);
        assert!(
            matches!(decode_der(&input, DerLimits::default(), |r| r.read_integer()),
            Err(error) if error.reason() == DerErrorReason::NonCanonicalEncoding)
        );
    }
}

#[test]
fn invalid_oid_utf8_null_and_constructed_string_encodings_fail() {
    for contents in [
        &[][..],
        &[0x80, 0x2a],
        &[0x2a, 0x80, 0x01],
        &[0x2a, 0x80],
        &[0xff; 40],
    ] {
        let input = tlv(0x06, contents);
        assert!(decode_der(&input, DerLimits::default(), |r| r.read_object_identifier()).is_err());
    }
    for contents in [&[0xff][..], &[0xc0, 0x80], &[0xed, 0xa0, 0x80]] {
        let input = tlv(0x0c, contents);
        assert!(decode_der(&input, DerLimits::default(), |r| r.read_utf8_string()).is_err());
    }
    assert!(decode_der(&[0x05, 1, 0], DerLimits::default(), |r| r.read_null()).is_err());
    assert!(decode_der(&[0x24, 0], DerLimits::default(), |r| r.read_octet_string()).is_err());
}

#[test]
fn set_of_rejects_unsorted_primitive_and_nested_values() {
    let encoded = tlv(0x31, &[0x02, 1, 2, 0x02, 1, 1]);
    let result = decode_der(&encoded, DerLimits::default(), |r| {
        r.read_set_of(|set| {
            set.read_unsigned_integer()?;
            set.read_unsigned_integer()?;
            Ok(())
        })
    });
    assert_eq!(result, Err(DerError::new(DerErrorReason::SetOrdering)));
    let mut contents = pair(&[2], &[1]);
    contents.extend_from_slice(&pair(&[1], &[1]));
    let encoded = tlv(0x31, &contents);
    assert_eq!(
        decode_der(&encoded, DerLimits::default(), |r| r.read_set_of(|set| {
            for _ in 0..2 {
                set.read_sequence(|sequence| {
                    sequence.read_unsigned_integer()?;
                    sequence.read_unsigned_integer()?;
                    Ok(())
                })?;
            }
            Ok(())
        })),
        Err(DerError::new(DerErrorReason::SetOrdering))
    );
}
