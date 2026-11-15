// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{
    admit_map_key, collect_map_entries, decode_map_entry, extract_string, open_map,
    read_value_head, CborValue, DecodeBudget, ManuallyDrop, MapKeyOrder, MT_STRING,
};
use crate::decode_dag_cbor::{checked_end, declared_contents};
use crate::nested_map_proof_tests::unicode_keys::{Map, DECOMPOSED, PRECOMPOSED};

#[kani::proof]
#[kani::unwind(5)]
fn first_key_supplies_its_original_utf8_bytes() {
    verify_text_key(false);
}

#[kani::proof]
#[kani::unwind(5)]
fn second_key_supplies_its_original_utf8_bytes() {
    verify_text_key(true);
}

fn verify_text_key(second: bool) {
    let fixture = Map::symbolic();
    let wire = fixture.wire();
    let (start, end) = fixture.spans()[usize::from(second)];
    let expected = fixture.text(second);
    let mut budget = entry_budget(second);
    let payload = start + 1;
    let length = if expected == PRECOMPOSED { 2 } else { 3 };
    assert!(read_value_head(wire, start, &mut budget) == Ok((MT_STRING, length, payload)));
    let Ok(bytes) = declared_contents(wire, payload, length) else {
        kani::assert(false, "the complete key payload follows the header");
        return;
    };
    assert!(checked_end(payload, bytes.len()) == Ok(end));
    assert!(matches_spelling(bytes, expected));
    fixture.cover_all();
}

#[kani::proof]
#[kani::unwind(5)]
fn precomposed_utf8_extraction_preserves_both_bytes() {
    verify_utf8_extraction(&[0xc3, 0xa9], 2, PRECOMPOSED);
    kani::cover!(true, "precomposed spelling extracted verbatim");
}

#[kani::proof]
#[kani::unwind(5)]
fn decomposed_utf8_extraction_preserves_all_three_bytes() {
    verify_utf8_extraction(&[0x65, 0xcc, 0x81], 3, DECOMPOSED);
    kani::cover!(true, "decomposed spelling extracted verbatim");
}

fn verify_utf8_extraction(payload: &[u8], length: u64, expected: &str) {
    // The key checks establish this same complete payload at each wire offset.
    let result = ManuallyDrop::new(extract_string(payload, 0, length));
    assert!(matches!(&*result, Ok((text, end))
        if *end == payload.len() && matches_spelling(text.as_bytes(), expected)));
}

#[kani::proof]
#[kani::unwind(5)]
fn first_entry_preserves_its_text_and_value() {
    verify_entry(false);
}

#[kani::proof]
#[kani::unwind(5)]
fn second_entry_preserves_distinct_text_and_rejects_repeated_bytes() {
    verify_entry(true);
}

fn verify_entry(second: bool) {
    let fixture = Map::symbolic();
    let wire = fixture.wire();
    let [first, last] = fixture.spans();
    let (start, end) = if second { last } else { first };
    let admission_error = || if second { fixture.second_error() } else { None };
    let spelling = fixture.text(second);
    let bit = fixture.bit(second);
    let mut budget = entry_budget(second);
    let mut calls = 0;
    let mut span = None;
    let result = ManuallyDrop::new(decode_map_entry(
        wire,
        start,
        2,
        &mut budget,
        |source, offset, depth, _| {
            assert!(core::ptr::eq(source, wire) && depth == 2);
            calls += 1;
            if calls == 1 {
                assert!(offset == start);
                // The leaf check supplies this exact spelling and cursor.
                Ok((CborValue::String(String::from(spelling)), end))
            } else {
                assert!(calls == 2 && offset == end);
                Ok((CborValue::Bool(bit), end + 1))
            }
        },
        |range| {
            span = Some((range.start, range.end));
            // Traversal checks this admission outcome against the wire.
            if let Some(error) = admission_error() {
                Err(error)
            } else {
                Ok(())
            }
        },
    ));
    assert!(span == Some((start, end)));
    if let Some(error) = admission_error() {
        assert!(matches!(&*result, Err(actual) if *actual == error) && calls == 1);
    } else {
        assert!(matches!(&*result, Ok(((key, CborValue::Bool(value)), next))
            if matches_spelling(key.as_bytes(), spelling) && *value == bit && *next == end + 1));
        assert!(calls == 2);
    }
    fixture.cover_all();
}

#[kani::proof]
#[kani::unwind(5)]
fn map_traversal_retains_both_keys_or_stops_at_the_second_key() {
    let fixture = Map::symbolic();
    let source = fixture.wire();
    let mut budget = DecodeBudget::default();
    assert!(open_map(source, 0, 0, &mut budget) == (1, 1, 1));
    assert!(read_value_head(source, 1, &mut budget) == Ok((MT_STRING, 0, 2)));
    assert!(open_map(source, 2, 1, &mut budget) == (3, 2, 2));
    assert!(budget.seen == 3 && budget.pending == 4);
    let mut entries = Vec::with_capacity(2);
    let mut order = MapKeyOrder::default();
    let mut calls = 0;
    let result = collect_map_entries(2, 3, &mut entries, |offset| {
        assert!(calls < 2);
        let (start, end) = fixture.spans()[calls];
        let second = calls == 1;
        assert!(offset == start);
        calls += 1;
        // These spans and the preserved text are established by the entry checks.
        budget.enter()?;
        admit_map_key(&mut order, source, start..end)?;
        let (major, argument, next) = read_value_head(source, end, &mut budget)?;
        let bit = fixture.bit(second);
        assert!(major == 7 && argument == if bit { 21 } else { 20 });
        Ok(((fixture.text(second), bit), next))
    });
    assert!(calls == 2);
    if let Some(expected) = fixture.second_error() {
        assert!(result == Err(expected) && entries.len() == 1);
        assert!(budget.seen == 6 && budget.pending == 1);
    } else {
        assert!(result == Ok(source.len()));
        assert!(matches!(entries.as_slice(), [(short, false), (long, true)]
            if matches_spelling(short.as_bytes(), PRECOMPOSED) && matches_spelling(long.as_bytes(), DECOMPOSED)));
        assert!(budget.seen == 7 && budget.pending == 0);
    }
    fixture.cover_all();
}

fn entry_budget(second: bool) -> DecodeBudget {
    if second {
        DecodeBudget {
            seen: 5,
            pending: 2,
        }
    } else {
        DecodeBudget {
            seen: 3,
            pending: 4,
        }
    }
}

fn matches_spelling(actual: &[u8], expected: &str) -> bool {
    match actual {
        [0xc3, 0xa9] => expected == PRECOMPOSED,
        [0x65, 0xcc, 0x81] => expected == DECOMPOSED,
        _ => false,
    }
}
