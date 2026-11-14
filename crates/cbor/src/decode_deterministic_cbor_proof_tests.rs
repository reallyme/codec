// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{
    admit_map_key, bounded_capacity, collect_map_entries, decode_key, decode_map, decode_map_entry,
    decode_simple, read_head, read_value_head, reserve_map, DecodeLimits, DeterministicCborError,
};
use crate::check_map_key_order::{MapKeyOrder, MapKeyOrderError};
use crate::nested_map_proof_tests::{TwoEntryMap, MIXED_LENGTH_CANONICAL, MIXED_LENGTH_REVERSED};
use crate::{DeterministicCborMapEntry, DeterministicCborMapKey, DeterministicCborValue};
use core::cmp::Ordering;

const MAX_HEADER_BYTES: usize = 9;
const MAX_KEY_BYTES: usize = 8;

#[kani::proof]
#[kani::unwind(4)]
fn mixed_length_wire_admission_uses_length_before_lexical_order() {
    let long_first: bool = kani::any();
    let (wire, first_end, second_start, expected) = if long_first {
        (
            &MIXED_LENGTH_REVERSED,
            6,
            7,
            Err(DeterministicCborError::MapKeysOutOfOrder),
        )
    } else {
        (&MIXED_LENGTH_CANONICAL, 5, 6, Ok(()))
    };
    let mut order = MapKeyOrder::default();
    assert!(admit_map_key(&mut order, wire, 3, first_end).is_ok());
    let result = admit_map_key(&mut order, wire, second_start, 9);
    assert_eq!(result, expected);
    kani::cover!(
        long_first && result.is_err(),
        "longer text key first rejected"
    );
    kani::cover!(!long_first && result.is_ok(), "length-first order admitted");
}

#[kani::proof]
#[kani::unwind(4)]
fn mixed_length_canonical_first_span_is_exact() {
    verify_mixed_length_entry::<3, 5, 6>(&MIXED_LENGTH_CANONICAL, b"z", false, 0, Ok(()));
    kani::cover!(true, "first canonical span checked");
}

#[kani::proof]
#[kani::unwind(4)]
fn mixed_length_canonical_second_span_is_exact() {
    verify_mixed_length_entry::<6, 9, 10>(&MIXED_LENGTH_CANONICAL, b"aa", true, 1, Ok(()));
    kani::cover!(true, "second canonical span checked");
}

#[kani::proof]
#[kani::unwind(4)]
fn mixed_length_reversed_first_span_is_exact() {
    verify_mixed_length_entry::<3, 6, 7>(&MIXED_LENGTH_REVERSED, b"aa", true, 0, Ok(()));
    kani::cover!(true, "first reversed span checked");
}

#[kani::proof]
#[kani::unwind(4)]
fn mixed_length_reversed_second_span_is_exact() {
    verify_mixed_length_entry::<7, 9, 10>(
        &MIXED_LENGTH_REVERSED,
        b"z",
        false,
        2,
        Err(DeterministicCborError::MapKeysOutOfOrder),
    );
    kani::cover!(true, "second reversed span checked");
}

#[kani::proof]
#[kani::unwind(4)]
fn mixed_length_traversal_forwards_the_cursor_and_stops_on_rejection() {
    let long_first: bool = kani::any();
    let mut calls = 0;
    let result = collect_map_entries(2, 3, |start| {
        calls += 1;
        match calls {
            1 => {
                assert!(start == 3);
                // The entry checks establish the cursor after each value.
                Ok((long_first, if long_first { 7 } else { 6 }))
            }
            2 if long_first => {
                assert!(start == 7);
                Err(DeterministicCborError::MapKeysOutOfOrder)
            }
            2 => {
                assert!(start == 6);
                Ok((true, 10))
            }
            _ => {
                kani::assert(false, "a two-entry map visits at most two keys");
                Err(DeterministicCborError::UnexpectedEnd)
            }
        }
    });
    assert!(calls == 2);
    if long_first {
        assert_eq!(result, Err(DeterministicCborError::MapKeysOutOfOrder));
        kani::cover!(result.is_err(), "traversal returns the second key error");
    } else {
        assert!(matches!(result, Ok((ref entries, 10)) if entries.as_slice() == [false, true]));
        kani::cover!(result.is_ok(), "canonical traversal consumes ten bytes");
    }
}

fn verify_mixed_length_entry<const START: usize, const END: usize, const NEXT: usize>(
    wire: &[u8; 10],
    expected_key: &[u8],
    bit: bool,
    preceding_key_bytes: usize,
    admission: Result<(), DeterministicCborError>,
) {
    let second = START != 3;
    let mut limits = DecodeLimits {
        nodes: if second { 5 } else { 3 },
        pending_nodes: if second { 2 } else { 4 },
        aggregate_text_bytes: preceding_key_bytes,
        ..DecodeLimits::default()
    };
    let mut key_span = None;
    let mut child_called = false;
    let entry = decode_map_entry(
        wire,
        START,
        2,
        &mut limits,
        |source, start, depth, budget| {
            child_called = true;
            assert!(core::ptr::eq(source, wire.as_slice()));
            assert!(start == END && depth == 2);
            let (major, argument, next) = read_value_head(source, start, budget)?;
            assert!(major == 7 && next == NEXT);
            Ok((decode_simple(argument)?.into_value(), next))
        },
        |span| {
            // Observe the span prior to the independent admission decision.
            key_span = Some((span.start, span.end));
            admission
        },
    );
    let entry = core::mem::ManuallyDrop::new(entry);
    assert!(key_span == Some((START, END)));
    assert!(child_called == admission.is_ok());
    assert!(limits.aggregate_text_bytes == preceding_key_bytes + expected_key.len());
    if let Err(expected) = admission {
        assert!(matches!(&*entry, Err(actual) if *actual == expected));
        assert!(limits.nodes == 6 && limits.pending_nodes == 1);
        return;
    }
    let Ok((entry, next)) = &*entry else {
        kani::assert(false, "entry supplies its key span and value");
        return;
    };
    assert!(*next == NEXT);
    assert!(limits.nodes == if second { 7 } else { 5 });
    assert!(limits.pending_nodes == if second { 0 } else { 2 });
    assert!(
        matches!(entry.key(), DeterministicCborMapKey::Text(key) if key.as_bytes() == expected_key)
    );
    assert!(matches!(entry.value(), DeterministicCborValue::Bool(value) if *value == bit));
}

#[kani::proof]
#[kani::unwind(3)]
fn nested_prefix_reserves_two_entries_at_the_first_key() {
    let example = TwoEntryMap::symbolic();
    let wire = example.wire();
    verify_nested_two_entry_prefix(&wire);
    kani::cover!(
        example.first == 0 && example.second == 0x7f,
        "inner traversal can start"
    );
}

#[kani::proof]
#[kani::unwind(3)]
fn mixed_length_prefix_starts_at_the_same_offset_in_both_orders() {
    // The nested prefix is identical. Check that the additional payload byte
    // does not change the reservation or the initial cursor in either order.
    verify_nested_two_entry_prefix(&MIXED_LENGTH_CANONICAL);
    verify_nested_two_entry_prefix(&MIXED_LENGTH_REVERSED);
    kani::cover!(true, "both mixed-length prefixes reserve their entries");
}

fn verify_nested_two_entry_prefix(wire: &[u8]) {
    let mut limits = DecodeLimits::default();
    let Ok((5, count, 1)) = read_value_head(wire, 0, &mut limits) else {
        kani::assert(false, "outer map header");
        return;
    };
    assert!(reserve_map(wire.len(), 1, 0, count, &mut limits) == Ok((1, 1)));
    let Ok((DeterministicCborMapKey::Text(ref key), 2)) = decode_key(wire, 1, &mut limits) else {
        kani::assert(false, "outer empty text key consumes one byte");
        return;
    };
    assert!(key.is_empty());
    let mut outer_order = MapKeyOrder::default();
    assert!(admit_map_key(&mut outer_order, wire, 1, 2).is_ok());
    let Ok((5, count, 3)) = read_value_head(wire, 2, &mut limits) else {
        kani::assert(false, "inner map header");
        return;
    };
    assert!(reserve_map(wire.len(), 3, 1, count, &mut limits) == Ok((2, 2)));
    assert!(limits.nodes == 3 && limits.pending_nodes == 4);
    assert!(limits.aggregate_text_bytes == 0 && limits.aggregate_byte_string_bytes == 0);
}

#[kani::proof]
#[kani::unwind(4)]
fn inner_traversal_passes_each_returned_offset_to_the_next_entry() {
    let example = TwoEntryMap::symbolic();
    let mut calls = 0;
    let result = collect_map_entries(2, 3, |start| {
        // These are the offsets established by the individual entry checks.
        let (key, bit, next) = if calls == 0 {
            assert!(start == 3);
            (example.first, false, 6)
        } else {
            assert!(calls == 1 && start == 6);
            (example.second, true, 9)
        };
        calls += 1;
        Ok(((key, bit), next))
    });
    let Ok((entries, 9)) = result else {
        kani::assert(false, "traversal consumes the complete inner map");
        return;
    };
    assert!(calls == 2);
    assert!(
        matches!(entries.as_slice(), [(first, false), (second, true)]
        if *first == example.first && *second == example.second)
    );
    kani::cover!(
        example.first == 0 && example.second == 0x7f,
        "both key positions retained"
    );
}

#[kani::proof]
#[kani::unwind(4)]
fn first_inner_entry_supplies_exact_key_span() {
    verify_inner_entry::<3, 5, 6>(false);
}

#[kani::proof]
#[kani::unwind(4)]
fn second_inner_entry_supplies_exact_key_span() {
    verify_inner_entry::<6, 8, 9>(true);
}

fn verify_inner_entry<const START: usize, const END: usize, const NEXT: usize>(bit: bool) {
    let example = TwoEntryMap::symbolic();
    let wire = example.wire();
    let mut limits = DecodeLimits {
        nodes: if bit { 5 } else { 3 },
        pending_nodes: if bit { 2 } else { 4 },
        aggregate_text_bytes: usize::from(bit),
        ..DecodeLimits::default()
    };
    let mut observed = None;
    let rejected: bool = kani::any();
    let mut child_called = false;
    let result = decode_map_entry(
        &wire,
        START,
        2,
        &mut limits,
        |source, offset, depth, limits| {
            child_called = true;
            assert!(core::ptr::eq(source, wire.as_slice()));
            assert!(offset == END && depth == 2);
            let (major, argument, next) = read_value_head(source, offset, limits)?;
            assert!(major == 7 && next == NEXT);
            Ok((decode_simple(argument)?.into_value(), next))
        },
        |span| {
            observed = Some((span.start, span.end));
            if rejected {
                Err(DeterministicCborError::DuplicateMapKey)
            } else {
                Ok(())
            }
        },
    );
    let result = core::mem::ManuallyDrop::new(result);
    assert!(observed == Some((START, END)));
    assert!(child_called != rejected);
    if rejected {
        assert!(matches!(
            &*result,
            Err(DeterministicCborError::DuplicateMapKey)
        ));
        kani::cover!(
            rejected,
            "key admission error stops before decoding the child"
        );
        return;
    }
    let Ok((entry, next)) = &*result else {
        kani::assert(false, "entry parses");
        return;
    };
    assert!(*next == NEXT);
    let expected = if bit { example.second } else { example.first };
    assert_text_entry(entry, expected, bit);
    assert!(limits.nodes == if bit { 7 } else { 5 });
    assert!(limits.pending_nodes == if bit { 0 } else { 2 });
    assert!(limits.aggregate_text_bytes == if bit { 2 } else { 1 });
    kani::cover!(
        expected == 0x7f,
        "highest single ASCII byte reaches admission"
    );
}

fn assert_text_entry(entry: &DeterministicCborMapEntry, expected: u8, bit: bool) {
    assert!(matches!(entry.key(), DeterministicCborMapKey::Text(key)
        if matches!(key.as_bytes(), [byte] if *byte == expected)));
    assert!(matches!(entry.value(), DeterministicCborValue::Bool(actual) if *actual == bit));
}

#[kani::proof]
#[kani::unwind(3)]
fn two_map_layers_retain_the_leaf_and_consume_the_wire() {
    let bit: bool = kani::any();
    let wire = [0xa1, 0x60, 0xa1, 0x60, 0xf4 | u8::from(bit)];
    let mut limits = DecodeLimits::default();
    let decoded = decode_proof_map(
        &wire,
        0,
        0,
        1,
        &mut limits,
        |wire, offset, depth, limits| {
            assert!(offset == 2 && depth == 1);
            decode_proof_map(
                wire,
                offset,
                depth,
                1,
                limits,
                |wire, offset, depth, limits| {
                    assert!(offset == 4 && depth == 2);
                    let (major, arg, end) = read_value_head(wire, offset, limits)?;
                    assert!(major == 7);
                    Ok((decode_simple(arg)?.into_value(), end))
                },
            )
        },
    );
    // Retain the returned tree so the harness ends at the parser's result.
    // The runtime zeroization tests exercise the owner's subsequent drop.
    let decoded = core::mem::ManuallyDrop::new(decoded);
    let Ok((outer, end)) = &*decoded else {
        kani::assert(false, "both map layers decode");
        return;
    };
    assert!(*end == wire.len());
    let Some(inner) = single_empty_key_child(outer) else {
        kani::assert(false, "outer map owns the inner map under its empty key");
        return;
    };
    assert!(
        matches!(single_empty_key_child(inner), Some(DeterministicCborValue::Bool(actual)) if *actual == bit)
    );
    assert!(limits.nodes == 5 && limits.pending_nodes == 0);
    kani::cover!(bit, "true leaf retained through both map layers");
    kani::cover!(!bit, "false leaf retained through both map layers");
}

#[kani::proof]
#[kani::unwind(4)]
fn nested_two_entry_key_spans_admit_only_increasing_keys() {
    let example = TwoEntryMap::symbolic();
    let wire = example.wire();
    let mut order = MapKeyOrder::default();
    assert!(admit_map_key(&mut order, &wire, 3, 5).is_ok());
    let result = admit_map_key(&mut order, &wire, 6, 8);
    match example.first.cmp(&example.second) {
        Ordering::Less => {
            assert_eq!(result, Ok(()));
            kani::cover!(
                example.first == 0 && example.second == 0x7f,
                "increasing keys"
            );
        }
        Ordering::Equal => {
            assert_eq!(result, Err(DeterministicCborError::DuplicateMapKey));
            kani::cover!(result.is_err(), "duplicate keys");
        }
        Ordering::Greater => {
            assert_eq!(result, Err(DeterministicCborError::MapKeysOutOfOrder));
            kani::cover!(result.is_err(), "descending keys");
        }
    }
}

#[kani::proof]
#[kani::unwind(3)]
fn one_byte_text_key_decodes_to_the_same_ascii_byte() {
    let example = TwoEntryMap::symbolic();
    let wire = [0x61, example.first, 0xf4];
    let mut limits = DecodeLimits::default();
    let decoded = decode_key(&wire, 0, &mut limits);
    assert!(
        matches!(&decoded, Ok((DeterministicCborMapKey::Text(key), 2))
        if matches!(key.as_bytes(), [byte] if *byte == example.first))
    );
    assert!(limits.nodes == 1 && limits.aggregate_text_bytes == 1);
    kani::cover!(example.first == 0x7f, "highest single ASCII byte");
}

fn decode_proof_map(
    wire: &[u8],
    offset: usize,
    depth: usize,
    expected_entries: u64,
    limits: &mut DecodeLimits,
    decode_child: impl FnMut(
        &[u8],
        usize,
        usize,
        &mut DecodeLimits,
    ) -> Result<(DeterministicCborValue, usize), DeterministicCborError>,
) -> Result<(DeterministicCborValue, usize), DeterministicCborError> {
    let (major, argument, start) = read_value_head(wire, offset, limits)?;
    assert!(major == 5 && argument == expected_entries);
    decode_map(wire, start, depth, argument, limits, decode_child)
}

fn single_empty_key_child(value: &DeterministicCborValue) -> Option<&DeterministicCborValue> {
    let DeterministicCborValue::Map(entries) = value else {
        return None;
    };
    let [entry] = entries.as_slice() else {
        return None;
    };
    match entry.key() {
        DeterministicCborMapKey::Text(key) if key.is_empty() => Some(entry.value()),
        _ => None,
    }
}

#[kani::proof]
#[kani::unwind(9)]
fn map_key_admission_matches_length_first_order() {
    let previous: [u8; MAX_KEY_BYTES] = kani::any();
    let current: [u8; MAX_KEY_BYTES] = kani::any();
    let previous_len: usize = kani::any();
    let current_len: usize = kani::any();
    kani::assume((1..=MAX_KEY_BYTES).contains(&previous_len));
    kani::assume((1..=MAX_KEY_BYTES).contains(&current_len));

    // Equal-length octet strings order like unsigned big-endian integers.
    // Ignore the bytes outside the prefix by shifting them out of each rank.
    let previous_rank = u64::from_be_bytes(previous) >> ((MAX_KEY_BYTES - previous_len) * 8);
    let current_rank = u64::from_be_bytes(current) >> ((MAX_KEY_BYTES - current_len) * 8);
    let expected = if (previous_len, previous_rank) < (current_len, current_rank) {
        Ok(())
    } else if (previous_len, previous_rank) == (current_len, current_rank) {
        Err(MapKeyOrderError::Duplicate)
    } else {
        Err(MapKeyOrderError::Descending)
    };
    let mut order = MapKeyOrder::default();
    assert_eq!(order.admit(&previous[..previous_len]), Ok(()));
    let actual = order.admit(&current[..current_len]);
    kani::cover!(
        previous_len < current_len && previous[0] > current[0] && actual.is_ok(),
        "length takes precedence over the first byte"
    );
    kani::cover!(
        previous_len == current_len && actual.is_ok(),
        "equal-length keys in byte order"
    );
    kani::cover!(actual == Err(MapKeyOrderError::Duplicate), "duplicate key");
    kani::cover!(
        actual == Err(MapKeyOrderError::Descending),
        "descending key order"
    );
    assert_eq!(actual, expected);
}

#[kani::proof]
#[kani::unwind(10)]
fn integer_header_requires_the_shortest_encoding() {
    let mut bytes: [u8; MAX_HEADER_BYTES] = kani::any();
    let available: usize = kani::any();
    kani::assume(available <= bytes.len());
    // Both integer major types share the same unsigned wire argument. Keeping
    // the argument symbolic covers all u64 values, including the sign boundary.
    bytes[0] &= 0x3f;
    let additional = bytes[0] & 0x1f;
    let actual = read_head(&bytes[..available], 0);
    let width = match additional {
        0..=23 => 0,
        24 => 1,
        25 => 2,
        26 => 4,
        27 => 8,
        _ => 0,
    };
    let expected = if available == 0 {
        Err(DeterministicCborError::UnexpectedEnd)
    } else if additional >= 28 {
        Err(DeterministicCborError::UnsupportedAdditionalInfo)
    } else if available <= width {
        Err(DeterministicCborError::TruncatedArgument)
    } else {
        // Interpret the payload as one zero-padded big-endian integer. Its bit
        // length determines the only permitted extended encoding width.
        let end = width + 1;
        let argument = if width == 0 {
            u64::from(additional)
        } else {
            let mut padded = [0; 8];
            padded[8 - width..].copy_from_slice(&bytes[1..end]);
            u64::from_be_bytes(padded)
        };
        let preferred_width = match argument {
            0..=23 => 0,
            _ => (u64::BITS - argument.leading_zeros())
                .div_ceil(8)
                .next_power_of_two(),
        };
        if u32::try_from(width) == Ok(preferred_width) {
            Ok((bytes[0] >> 5, argument, end))
        } else {
            Err(DeterministicCborError::NonCanonicalInteger)
        }
    };
    kani::cover!(
        matches!(actual, Ok((0, _, 9))),
        "eight-byte unsigned argument"
    );
    kani::cover!(
        matches!(actual, Ok((1, _, 9))),
        "eight-byte negative argument"
    );
    kani::cover!(
        actual == Err(DeterministicCborError::NonCanonicalInteger),
        "overlong argument"
    );
    kani::cover!(
        actual == Err(DeterministicCborError::TruncatedArgument),
        "truncated argument"
    );
    assert_eq!(actual, expected);
}

#[kani::proof]
fn container_capacity_fits_remaining_source() {
    let count: usize = kani::any();
    let total: usize = kani::any();
    let offset: usize = kani::any();
    let is_map: bool = kani::any();
    // The array and map callers require at least one and two bytes per
    // entry respectively. Widen the reference arithmetic to avoid wraparound.
    let bytes_per_entry = if is_map { 2 } else { 1 };
    let required_end = wide(count) * wide(bytes_per_entry) + wide(offset);
    let expected = if offset > total {
        Err(DeterministicCborError::OffsetOverflow)
    } else if required_end > wide(total) {
        Err(DeterministicCborError::ContainerLengthExceedsInput)
    } else {
        Ok(())
    };
    let actual = bounded_capacity(count, total, offset, bytes_per_entry);
    kani::cover!(is_map && actual.is_ok(), "map admitted");
    kani::cover!(!is_map && actual.is_ok(), "array admitted");
    kani::cover!(
        actual == Err(DeterministicCborError::ContainerLengthExceedsInput),
        "entry count exceeds remaining bytes"
    );
    assert_eq!(actual, expected);
}

fn wide(value: usize) -> u128 {
    u128::from(u64::try_from(value).unwrap())
}
