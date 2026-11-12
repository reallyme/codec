// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Canonical DER primitives, boundary encodings, and borrowed output contracts.

#![cfg(feature = "der")]

#[path = "der/support/mod.rs"]
mod support;

use reallyme_codec::der::{
    decode_der, decode_unsigned_integer_pair, DerError, DerErrorReason, DerLimits,
};
use support::{pair, tlv};

#[test]
fn exact_pair_borrows_minimal_magnitudes_and_keeps_zero() -> Result<(), DerError> {
    let encoded = [0x30, 0x07, 0x02, 0x02, 0x00, 0x80, 0x02, 0x01, 0x00];
    let decoded = decode_unsigned_integer_pair(&encoded, DerLimits::new(9, 1, 3, 1)?)?;
    assert_eq!(decoded.first(), &[0x80]);
    assert_eq!(decoded.second(), &[0]);
    assert!(core::ptr::eq(
        decoded.first().as_ptr(),
        encoded[5..].as_ptr()
    ));
    assert!(core::ptr::eq(
        decoded.second().as_ptr(),
        encoded[8..].as_ptr()
    ));
    Ok(())
}

#[test]
fn curve_independent_widths_include_40_and_66_and_sign_octets() -> Result<(), DerError> {
    for width in [1, 32, 40, 48, 66, 128, 256, 4_096] {
        for high_bit in [0x01, 0x80, 0xff] {
            let mut magnitude = vec![0x34; width];
            magnitude[0] = high_bit;
            let encoded = pair(&magnitude, &magnitude);
            let limits = DerLimits::new(encoded.len(), 1, 3, width)?;
            let parsed = decode_unsigned_integer_pair(&encoded, limits)?;
            assert_eq!(parsed.first(), magnitude);
            assert_eq!(parsed.second(), magnitude);
            let short = DerLimits::new(encoded.len(), 1, 3, width.saturating_sub(1).max(1))?;
            if width > 1 {
                assert!(matches!(decode_unsigned_integer_pair(&encoded, short),
                    Err(error) if error.reason() == DerErrorReason::IntegerTooLarge));
            }
        }
    }
    Ok(())
}

#[test]
fn integer_syntax_does_not_invent_scalar_range_or_low_s_policy() -> Result<(), DerError> {
    for magnitude in [&[0][..], &[1], &[0xff; 40], &[0xff; 66]] {
        let encoded = pair(magnitude, magnitude);
        let result = decode_unsigned_integer_pair(&encoded, DerLimits::default())?;
        assert_eq!(result.first(), magnitude);
        assert_eq!(result.second(), magnitude);
    }
    Ok(())
}

#[test]
fn signed_integers_preserve_canonical_twos_complement() -> Result<(), DerError> {
    for contents in [
        &[0][..],
        &[0x7f],
        &[0, 0x80],
        &[0xff],
        &[0x80],
        &[0xff, 0x7f],
    ] {
        let encoded = tlv(0x02, contents);
        let value = decode_der(&encoded, DerLimits::default(), |r| r.read_integer())?;
        assert_eq!(value.as_bytes(), contents);
    }
    Ok(())
}

#[test]
fn canonical_length_transitions_are_supported_without_payload_copies() -> Result<(), DerError> {
    for length in [0, 1, 127, 128, 255, 256, 65_535, 65_536] {
        let contents = vec![0x5a; length];
        let encoded = tlv(0x04, &contents);
        let result = decode_der(&encoded, DerLimits::default(), |r| r.read_octet_string())?;
        assert_eq!(result, contents);
    }
    Ok(())
}

#[test]
fn typed_primitives_and_nested_sequence_are_exact() -> Result<(), DerError> {
    let encoded = [
        0x30, 0x0f, 0x01, 0x01, 0xff, 0x05, 0x00, 0x0c, 0x02, 0xc3, 0xa9, 0x30, 0x04, 0x04, 0x02,
        0x61, 0x62,
    ];
    decode_der(&encoded, DerLimits::default(), |r| {
        r.read_sequence(|s| {
            assert!(s.read_boolean()?);
            s.read_null()?;
            assert_eq!(s.read_utf8_string()?, "é");
            s.read_sequence(|nested| {
                assert_eq!(nested.read_octet_string()?, b"ab");
                nested.finish()
            })?;
            assert!(s.is_finished());
            s.finish()
        })
    })
}

#[test]
fn bit_strings_validate_padding_and_allow_empty_values() -> Result<(), DerError> {
    for unused in 0_u8..=7 {
        let contents = [unused, 0x80];
        let encoded = tlv(0x03, &contents);
        let value = decode_der(&encoded, DerLimits::default(), |r| r.read_bit_string())?;
        assert_eq!(value.unused_bits(), unused);
        assert_eq!(value.as_bytes(), &[0x80]);
    }
    let value = decode_der(&[0x03, 0x01, 0x00], DerLimits::default(), |r| {
        r.read_bit_string()
    })?;
    assert!(value.as_bytes().is_empty());
    Ok(())
}

#[test]
fn oid_profile_accepts_canonical_values() -> Result<(), DerError> {
    for contents in [
        &[0x2a, 0x86, 0x48, 0xce, 0x3d, 0x04, 0x03, 0x02][..],
        &[0x55, 0x04, 0x03],
        &[0x88, 0x37, 0x03],
    ] {
        let encoded = tlv(0x06, contents);
        let result = decode_der(&encoded, DerLimits::default(), |r| {
            r.read_object_identifier()
        })?;
        assert_eq!(result.as_bytes(), contents);
    }
    Ok(())
}

#[test]
fn set_of_requires_full_encoding_order_and_permits_duplicates() -> Result<(), DerError> {
    for contents in [
        &[0x02, 1, 1, 0x02, 1, 2][..],
        &[0x02, 1, 1, 0x02, 1, 1],
        &[],
    ] {
        let encoded = tlv(0x31, contents);
        decode_der(&encoded, DerLimits::default(), |r| {
            r.read_set_of(|set| {
                while !set.is_finished() {
                    set.read_unsigned_integer()?;
                }
                Ok(())
            })
        })?;
    }
    Ok(())
}

#[test]
fn boolean_values_and_empty_utf8_string_are_preserved() -> Result<(), DerError> {
    for (octet, expected) in [(0, false), (0xff, true)] {
        let input = [0x01, 1, octet];
        assert_eq!(
            decode_der(&input, DerLimits::default(), |r| r.read_boolean())?,
            expected
        );
    }
    assert_eq!(
        decode_der(&[0x0c, 0], DerLimits::default(), |r| r.read_utf8_string())?,
        ""
    );
    Ok(())
}

#[test]
fn oid_arcs_are_not_narrowed_to_machine_integers() -> Result<(), DerError> {
    let mut large_arc = vec![0x2a, 0x81];
    large_arc.extend_from_slice(&[0x80; 64]);
    large_arc.push(0);
    for contents in [&[0][..], &[0x2a], large_arc.as_slice()] {
        let input = tlv(0x06, contents);
        let oid = decode_der(&input, DerLimits::default(), |r| r.read_object_identifier())?;
        assert_eq!(oid.as_bytes(), contents);
    }
    Ok(())
}

#[test]
fn set_of_orders_complete_tlvs_instead_of_payloads() -> Result<(), DerError> {
    let mut contents = tlv(0x04, b"z");
    contents.extend_from_slice(&tlv(0x04, b"aa"));
    let input = tlv(0x31, &contents);
    decode_der(&input, DerLimits::default(), |r| {
        r.read_set_of(|set| {
            assert_eq!(set.read_octet_string()?, b"z");
            assert_eq!(set.read_octet_string()?, b"aa");
            Ok(())
        })
    })
}
