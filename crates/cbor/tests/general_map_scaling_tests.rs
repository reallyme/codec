// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Map-key uniqueness at scale: valid maps up to the entry ceiling are
//! accepted within default limits, duplicates are found regardless of their
//! distance, and adversarial keys stay bounded by the work budget.

#![allow(missing_docs, clippy::unwrap_used)]

use codec_cbor::general::{
    validate_exact, CborError, ErrorReason, Limits, MapOrdering, Profile, MAX_ENTRIES,
};
use std::collections::BTreeSet;

fn reason<T>(result: Result<T, CborError>) -> Option<ErrorReason> {
    result.err().map(CborError::reason)
}

fn map_header(count: usize) -> Vec<u8> {
    let mut input = vec![0xb9];
    input.extend_from_slice(&u16::try_from(count).unwrap().to_be_bytes());
    input
}

fn preferred_unsigned(output: &mut Vec<u8>, value: u16) {
    match value {
        0..=23 => output.push(u8::try_from(value).unwrap()),
        24..=255 => output.extend_from_slice(&[0x18, u8::try_from(value).unwrap()]),
        _ => {
            output.push(0x19);
            output.extend_from_slice(&value.to_be_bytes());
        }
    }
}

fn integer_map(keys: impl Iterator<Item = u16>, count: usize) -> Vec<u8> {
    let mut input = map_header(count);
    for key in keys {
        preferred_unsigned(&mut input, key);
        input.push(0xf6);
    }
    input
}

#[test]
fn maps_at_the_entry_ceiling_validate_within_default_limits() {
    let count = u16::try_from(MAX_ENTRIES).unwrap();
    for keys in [
        integer_map(0..count, MAX_ENTRIES),
        integer_map((0..count).rev(), MAX_ENTRIES),
    ] {
        validate_exact(&keys, Profile::preferred(), Limits::default()).unwrap();
    }

    // A large value early in the map must not be rescanned for each later key.
    let entries = MAX_ENTRIES - 96;
    let mut input = map_header(entries);
    input.push(0x00);
    input.extend_from_slice(&[0x99, 0x0f, 0xa0]);
    input.extend(core::iter::repeat_n(0x00, 4000));
    for key in 1..u16::try_from(entries).unwrap() {
        preferred_unsigned(&mut input, key);
        input.push(0xf6);
    }
    validate_exact(&input, Profile::preferred(), Limits::default()).unwrap();
}

#[test]
fn distant_and_width_equivalent_duplicates_are_found_in_large_maps() {
    let count = u16::try_from(MAX_ENTRIES).unwrap();
    let mut keys: Vec<u16> = (0..count).collect();
    *keys.last_mut().unwrap() = 0;
    let input = integer_map(keys.into_iter(), MAX_ENTRIES);
    assert_eq!(
        reason(validate_exact(
            &input,
            Profile::preferred(),
            Limits::default()
        )),
        Some(ErrorReason::DuplicateMapKey)
    );

    // Key 1 appears first in preferred form and last in a wide form.
    let mut input = map_header(64);
    for key in 1..64_u16 {
        preferred_unsigned(&mut input, key);
        input.push(0xf6);
    }
    input.extend_from_slice(&[0x1a, 0x00, 0x00, 0x00, 0x01, 0xf6]);
    assert_eq!(
        reason(validate_exact(
            &input,
            Profile::received(),
            Limits::default()
        )),
        Some(ErrorReason::DuplicateMapKey)
    );
}

#[test]
fn ordered_profiles_report_duplicates_before_order_in_large_maps() {
    let input = integer_map(
        [
            5_u16, 1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 5,
        ]
        .into_iter(),
        18,
    );
    let ordered = Profile::received().with_map_ordering(MapOrdering::Bytewise);
    assert_eq!(
        reason(validate_exact(&input, ordered, Limits::default())),
        Some(ErrorReason::DuplicateMapKey)
    );
    let unordered = integer_map((0..32_u16).rev(), 32);
    assert_eq!(
        reason(validate_exact(&unordered, ordered, Limits::default())),
        Some(ErrorReason::MapKeyOrder)
    );
}

fn key_map(keys: &[&[u8]]) -> Vec<u8> {
    let mut input = map_header(keys.len());
    for key in keys {
        input.extend_from_slice(key);
        input.push(0xf6);
    }
    input
}

#[test]
fn semantic_equivalence_holds_on_the_sorted_path() {
    let fillers: Vec<Vec<u8>> = (100..120_u16)
        .map(|value| {
            let mut key = Vec::new();
            preferred_unsigned(&mut key, value);
            key
        })
        .collect();
    for (first, second) in [
        (&b"\x62ab"[..], &b"\x7f\x61a\x61b\xff"[..]),
        (b"\x42ab", b"\x5f\x41a\x41b\xff"),
        (b"\xf9\x00\x00", b"\xf9\x80\x00"),
        (b"\xf9\x3c\x00", b"\xfb\x3f\xf0\x00\x00\x00\x00\x00\x00"),
        (b"\xf9\x7e\x00", b"\xf9\xfe\x00"),
        (b"\xa2\x01\x02\x03\x04", b"\xa2\x03\x04\x01\x02"),
        (b"\x82\xa1\x01\x02\x00", b"\x82\xa1\x01\x02\x18\x00"),
        (b"\xc1\x00", b"\xc1\x18\x00"),
    ] {
        let mut keys: Vec<&[u8]> = vec![first];
        keys.extend(fillers.iter().map(Vec::as_slice));
        keys.push(second);
        assert_eq!(
            reason(validate_exact(
                &key_map(&keys),
                Profile::received(),
                Limits::default()
            )),
            Some(ErrorReason::DuplicateMapKey)
        );
    }

    let distinct_maps: Vec<Vec<u8>> = (0..24_u8).map(|value| vec![0xa1, value, 0x00]).collect();
    let keys: Vec<&[u8]> = distinct_maps.iter().map(Vec::as_slice).collect();
    validate_exact(&key_map(&keys), Profile::received(), Limits::default()).unwrap();
}

fn long_key_map(count: usize, key_len: usize) -> Vec<u8> {
    let mut input = map_header(count);
    for index in 0..count {
        input.push(0x59);
        input.extend_from_slice(&u16::try_from(key_len).unwrap().to_be_bytes());
        let start = input.len();
        input.resize(start + key_len, b'k');
        let suffix = u32::try_from(index).unwrap().to_be_bytes();
        input[start + key_len - 4..start + key_len].copy_from_slice(&suffix);
        input.push(0xf6);
    }
    input
}

#[test]
fn long_shared_prefix_keys_are_bounded_by_the_work_budget() {
    let input = long_key_map(512, 1024);
    validate_exact(&input, Profile::received(), Limits::default()).unwrap();
    let tight = Limits::default().with_work(1_000_000).unwrap();
    assert_eq!(
        reason(validate_exact(&input, Profile::received(), tight)),
        Some(ErrorReason::WorkLimit)
    );

    let mut duplicate = input.clone();
    let first_suffix = 3 + 3 + 1024 - 4;
    let last_suffix = first_suffix + 511 * (3 + 1024 + 1);
    let copied = duplicate[first_suffix..first_suffix + 4].to_vec();
    duplicate[last_suffix..last_suffix + 4].copy_from_slice(&copied);
    assert_eq!(
        reason(validate_exact(
            &duplicate,
            Profile::received(),
            Limits::default()
        )),
        Some(ErrorReason::DuplicateMapKey)
    );
}

/// Xorshift so every case is reproducible from its seed.
struct Generator(u64);

impl Generator {
    fn below(&mut self, bound: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % bound
    }
}

#[test]
fn duplicate_detection_matches_a_reference_set_across_the_inline_threshold() {
    let mut generator = Generator(0x6475_7065_9e37_79b9);
    for _ in 0..2_000 {
        let count = usize::try_from(generator.below(48)).unwrap();
        let mut input = map_header(count);
        let mut seen = BTreeSet::new();
        let mut duplicate = false;
        for _ in 0..count {
            let negative = generator.below(2) == 1;
            let value = generator.below(64);
            duplicate |= !seen.insert((negative, value));
            let major = if negative { 0x20 } else { 0x00 };
            match generator.below(3) {
                0 if value < 24 => input.push(major | u8::try_from(value).unwrap()),
                1 => input.extend_from_slice(&[major | 0x18, u8::try_from(value).unwrap()]),
                _ => {
                    input.push(major | 0x1a);
                    input.extend_from_slice(&u32::try_from(value).unwrap().to_be_bytes());
                }
            }
            input.push(0xf6);
        }
        let result = reason(validate_exact(
            &input,
            Profile::received(),
            Limits::default(),
        ));
        if duplicate {
            assert_eq!(result, Some(ErrorReason::DuplicateMapKey));
        } else {
            assert_eq!(result, None);
        }
    }
}
