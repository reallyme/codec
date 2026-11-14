// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{
    admit_map_key, collect_map_entries, decode_map_entry, decode_simple, extract_string, open_map,
    read_value_head, CborError, CborValue, DecodeBudget, ManuallyDrop, MapKeyOrder, MT_STRING,
};
use core::cell::Cell;

use crate::nested_map_proof_tests::three_entry::{
    assert_canonical_wire, cover_permutations, wire, Key, CANONICAL_KEYS, FIRST_KEY_OFFSET,
    MAX_WIRE_LEN,
};

#[kani::proof]
#[kani::unwind(5)]
fn a_key_parser_returns_its_exact_end() {
    verify_text_key(Key::A);
    kani::cover!(true, "a key parsed");
}

#[kani::proof]
#[kani::unwind(5)]
fn z_key_parser_returns_its_exact_end() {
    verify_text_key(Key::Z);
    kani::cover!(true, "z key parsed");
}

#[kani::proof]
#[kani::unwind(5)]
fn aa_key_parser_returns_its_exact_end() {
    verify_text_key(Key::Aa);
    kani::cover!(true, "aa key parsed");
}

fn verify_text_key(key: Key) {
    let (start, key_end, len) = symbolic_key_offsets(key);
    let mut wire: [u8; MAX_WIRE_LEN] = kani::any();
    wire[start..key_end].copy_from_slice(key.key_bytes());
    let source = &wire[..len];
    let payload = start + 1;
    let Ok(width) = u64::try_from(key.text().len()) else {
        kani::assert(false, "fixed text payload fits a CBOR length");
        return;
    };
    let mut budget = symbolic_entry_budget();
    let (seen, pending) = (budget.seen, budget.pending);
    assert!(read_value_head(source, start, &mut budget) == Ok((MT_STRING, width, payload)));
    assert!(budget.seen == seen + 1 && budget.pending == pending - 1);
    let text = ManuallyDrop::new(extract_string(source, payload, width));
    assert!(matches!(&*text, Ok((actual, next))
        if actual.as_bytes() == key.text().as_bytes() && *next == key_end));
}

#[kani::proof]
#[kani::unwind(4)]
fn entry_hands_off_exact_span_for_a() {
    verify_entry(Key::A);
}

#[kani::proof]
#[kani::unwind(4)]
fn entry_hands_off_exact_span_for_z() {
    verify_entry(Key::Z);
}

#[kani::proof]
#[kani::unwind(4)]
fn entry_hands_off_exact_span_for_aa() {
    verify_entry(Key::Aa);
}

fn verify_entry(key: Key) {
    let (start, end, len) = symbolic_key_offsets(key);
    let mut wire = [0; MAX_WIRE_LEN];
    wire[start..end].copy_from_slice(key.key_bytes());
    wire[end] = key.value_byte();
    let source = &wire[..len];
    let decision: u8 = kani::any();
    kani::assume(decision < 3);
    let expected_error = match decision {
        0 => None,
        1 => Some(CborError::DuplicateMapKey),
        _ => Some(CborError::MapKeysOutOfOrder),
    };
    let span = Cell::new(None);
    let mut calls = 0;
    let mut budget = symbolic_entry_budget();
    let (seen, pending) = (budget.seen, budget.pending);
    let result = ManuallyDrop::new(decode_map_entry(
        source,
        start,
        2,
        &mut budget,
        |bytes, offset, depth, budget| {
            assert!(core::ptr::eq(bytes, source) && depth == 2);
            calls += 1;
            if calls == 1 {
                assert!(offset == start);
                // The corresponding key-parser check establishes this text and end offset.
                budget.enter()?;
                Ok((CborValue::String(String::from(key.text())), end))
            } else {
                assert!(calls == 2 && offset == end && span.get() == Some((start, end)));
                budget.enter()?;
                Ok((decode_simple(u64::from(key.value_byte() & 0x1f))?, end + 1))
            }
        },
        |range| {
            span.set(Some((range.start, range.end)));
            match decision {
                0 => Ok(()),
                1 => Err(CborError::DuplicateMapKey),
                _ => Err(CborError::MapKeysOutOfOrder),
            }
        },
    ));
    assert!(span.get() == Some((start, end)));
    if let Some(expected) = expected_error {
        assert!(matches!(&*result, Err(error) if *error == expected));
        assert!(calls == 1 && budget.seen == seen + 1 && budget.pending == pending - 1);
    } else {
        assert!(matches!(&*result, Ok(((actual, value), next))
            if actual.as_bytes() == key.text().as_bytes() && value_is_for_key(value, key) && *next == end + 1));
        assert!(calls == 2 && budget.seen == seen + 2 && budget.pending == pending - 2);
    }
    kani::cover!(
        decision == 0 && calls == 2,
        "value read after key admission"
    );
    kani::cover!(
        decision == 1 && calls == 1,
        "duplicate stops before its value"
    );
    kani::cover!(
        decision == 2 && calls == 1,
        "descent stops before its value"
    );
}

fn symbolic_key_offsets(key: Key) -> (usize, usize, usize) {
    let start: usize = kani::any();
    let len: usize = kani::any();
    // Two preceding wide entries can move the third key to offset 11.
    kani::assume((FIRST_KEY_OFFSET..=11).contains(&start));
    let end = start + key.key_bytes().len();
    kani::assume((12..=MAX_WIRE_LEN).contains(&len) && end < len);
    (start, end, len)
}

fn symbolic_entry_budget() -> DecodeBudget {
    let preceding: u8 = kani::any();
    kani::assume(preceding < 3);
    let nodes = 2 * usize::from(preceding);
    DecodeBudget {
        seen: 3 + nodes,
        pending: 6 - nodes,
    }
}

fn value_is_for_key(value: &CborValue, key: Key) -> bool {
    matches!(
        (value, key),
        (CborValue::Bool(false), Key::A)
            | (CborValue::Bool(true), Key::Z)
            | (CborValue::Null, Key::Aa)
    )
}

#[kani::proof]
#[kani::unwind(5)]
fn traversal_admits_canonical_order_and_stops_at_first_violation() {
    let keys = [Key::symbolic(), Key::symbolic(), Key::symbolic()];
    let (storage, len) = wire(keys);
    let source = &storage[..len];
    let mut budget = DecodeBudget::default();
    assert!(open_map(source, 0, 0, &mut budget) == (1, 1, 1));
    assert!(read_value_head(source, 1, &mut budget) == Ok((MT_STRING, 0, 2)));
    assert!(open_map(source, 2, 1, &mut budget) == (FIRST_KEY_OFFSET, 3, 2));
    assert!(budget.seen == 3 && budget.pending == 6);

    // Compare semantic ranks for the oracle. Admission sees only the wire bytes.
    let rejected_at = if keys[0] >= keys[1] {
        Some(1)
    } else if keys[1] >= keys[2] {
        Some(2)
    } else {
        None
    };
    let mut order = MapKeyOrder::default();
    let mut entries = Vec::with_capacity(3);
    let mut visited = 0;
    let mut spans = [(0, 0); 3];
    let mut cursor = FIRST_KEY_OFFSET;
    let result = collect_map_entries(3, FIRST_KEY_OFFSET, &mut entries, |start| {
        assert!(visited < keys.len() && start == cursor);
        let key = keys[visited];
        let end = start + key.key_bytes().len();
        assert!(source[start..end] == *key.key_bytes());
        // The parser and entry checks give admission this range, excluding the value.
        spans[visited] = (start, end);
        visited += 1;
        budget.enter()?;
        admit_map_key(&mut order, source, start..end)?;
        assert!(source[end] == key.value_byte());
        budget.enter()?;
        cursor = end + 1;
        Ok((key, cursor))
    });

    if let Some(index) = rejected_at {
        let duplicate = keys[index - 1] == keys[index];
        let expected = if duplicate {
            CborError::DuplicateMapKey
        } else {
            CborError::MapKeysOutOfOrder
        };
        assert!(result == Err(expected));
        assert!(visited == index + 1 && entries.as_slice() == &keys[..index]);
        assert!(budget.seen == 4 + index * 2 && budget.pending == 5 - index * 2);
        kani::cover!(duplicate && index == 1, "duplicate rejected at key two");
        kani::cover!(duplicate && index == 2, "duplicate rejected at key three");
        kani::cover!(!duplicate && index == 1, "descent rejected at key two");
        kani::cover!(!duplicate && index == 2, "descent rejected at key three");
        kani::cover!(
            !duplicate && index == 1 && keys[1] == keys[2],
            "earlier descent precedes a later duplicate"
        );
    } else {
        assert!(keys == CANONICAL_KEYS && result == Ok(len));
        assert!(entries.as_slice() == CANONICAL_KEYS && visited == 3);
        assert!(budget.seen == 9 && budget.pending == 0);
        assert_canonical_wire(source);
    }
    assert_permutation_spans(keys, spans);
    cover_permutations(keys);
}

fn assert_permutation_spans(keys: [Key; 3], spans: [(usize, usize); 3]) {
    use Key::{Aa, A, Z};
    // Zero pairs mark unvisited slots. This table is independent of the
    // cursor calculations in the fixture builder and traversal callback.
    let expected = match keys {
        [A, Z, Aa] => [(3, 5), (6, 8), (9, 12)],
        [A, Aa, Z] | [Z, Aa, A] => [(3, 5), (6, 9), (10, 12)],
        [Z, A, Aa] => [(3, 5), (6, 8), (0, 0)],
        [Aa, A, Z] | [Aa, Z, A] => [(3, 6), (7, 9), (0, 0)],
        _ => return,
    };
    assert!(spans == expected);
}
