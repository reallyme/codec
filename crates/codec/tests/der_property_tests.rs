// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Property-based DER canonicality, bounded-input, and exact-consumption checks.

#![cfg(feature = "der")]

#[path = "der/support/mod.rs"]
mod support;

use proptest::collection::vec;
use proptest::prelude::{any, prop_assert, prop_assert_eq, proptest, ProptestConfig};
use reallyme_codec::der::{decode_der, decode_unsigned_integer_pair, DerLimits};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(512))]

    #[test]
    fn canonical_integer_pairs_round_trip_without_normalization(
        mut first in vec(any::<u8>(), 1..=66),
        mut second in vec(any::<u8>(), 1..=66),
    ) {
        if first.len() > 1 && first[0] == 0 { first[0] = 1; }
        if second.len() > 1 && second[0] == 0 { second[0] = 1; }
        let encoded = support::pair(&first, &second);
        let parsed = decode_unsigned_integer_pair(&encoded, DerLimits::default())?;
        prop_assert_eq!(parsed.first(), first.as_slice());
        prop_assert_eq!(parsed.second(), second.as_slice());
        prop_assert_eq!(support::pair(parsed.first(), parsed.second()), &encoded[..]);
    }

    #[test]
    fn trailing_bytes_never_extend_a_valid_document(
        first in 0_u8..=127,
        second in 0_u8..=127,
        suffix in vec(any::<u8>(), 1..=128),
    ) {
        let mut encoded = support::pair(&[first], &[second]);
        encoded.extend_from_slice(&suffix);
        prop_assert!(decode_unsigned_integer_pair(&encoded, DerLimits::default()).is_err());
    }

    #[test]
    fn arbitrary_inputs_never_yield_a_noncanonical_integer_pair(input in vec(any::<u8>(), 0..=1_024)) {
        if let Ok(parsed) = decode_unsigned_integer_pair(&input, DerLimits::default()) {
            prop_assert!(!parsed.first().is_empty());
            prop_assert!(!parsed.second().is_empty());
            prop_assert_eq!(support::pair(parsed.first(), parsed.second()), input);
        }
    }

    #[test]
    fn octet_strings_preserve_every_byte(payload in vec(any::<u8>(), 0..=4_096)) {
        let input = support::tlv(0x04, &payload);
        let parsed = decode_der(&input, DerLimits::default(), |r| r.read_octet_string())?;
        prop_assert_eq!(parsed, payload.as_slice());
    }

    #[test]
    fn redundant_integer_sign_octets_never_become_valid(value in 0_u8..=127) {
        let input = [0x30, 7, 0x02, 2, 0, value, 0x02, 1, 1];
        prop_assert!(decode_unsigned_integer_pair(&input, DerLimits::default()).is_err());
    }
}
