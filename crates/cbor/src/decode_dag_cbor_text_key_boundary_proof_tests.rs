// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{
    admit_map_key, collect_map_entries, decode_map_entry, open_map, read_value_head, CborError,
    CborValue, DecodeBudget, ManuallyDrop, MapKeyOrder, MT_STRING,
};
use crate::decode_dag_cbor::{checked_end, declared_contents};
use crate::nested_map_proof_tests::text_key_boundary::{
    CANONICAL_WIRE, LONGER_FIRST_WIRE, LONG_TEXT, SHORT_TEXT, WIRE_LEN,
};

#[kani::proof]
#[kani::unwind(4)]
fn both_wire_orders_use_the_shortest_headers() {
    let longer_first: bool = kani::any();
    let (wire, first, second) = if longer_first {
        (&LONGER_FIRST_WIRE, (24, 5), (30, 23, 31))
    } else {
        (&CANONICAL_WIRE, (23, 4), (28, 24, 30))
    };
    let mut first_budget = start_budget(false);
    let mut second_budget = start_budget(true);
    assert!(read_value_head(wire, 3, &mut first_budget) == Ok((MT_STRING, first.0, first.1)));
    let (second_key_start, length, payload) = second;
    assert!(
        read_value_head(wire, second_key_start, &mut second_budget)
            == Ok((MT_STRING, length, payload))
    );
    assert!(first_budget.seen == 4 && first_budget.pending == 3);
    assert!(second_budget.seen == 6 && second_budget.pending == 1);
    kani::cover!(
        longer_first,
        "canonical text heads in the longer-first wire"
    );
    kani::cover!(
        !longer_first,
        "canonical text heads in the shorter-first wire"
    );
}

#[kani::proof]
#[kani::unwind(4)]
fn all_wider_text_headers_are_rejected_at_the_boundary() {
    let length: u8 = kani::any();
    kani::assume(length == 23 || length == 24);
    let additional: u8 = kani::any();
    kani::assume((24..=27).contains(&additional));
    let bytes = match additional {
        24 => [0x78, length, 0, 0, 0, 0, 0, 0, 0],
        25 => [0x79, 0, length, 0, 0, 0, 0, 0, 0],
        26 => [0x7a, 0, 0, 0, length, 0, 0, 0, 0],
        _ => [0x7b, 0, 0, 0, 0, 0, 0, 0, length],
    };
    let mut budget = DecodeBudget::default();
    let result = read_value_head(&bytes, 0, &mut budget);
    if length == 24 && additional == 24 {
        assert!(result == Ok((MT_STRING, 24, 2)));
        kani::cover!(
            result.is_ok(),
            "one argument byte is shortest for length 24"
        );
    } else {
        assert!(result == Err(CborError::NonCanonicalInteger));
        kani::cover!(length == 23 && additional == 24, "78 17 is overlong");
        kani::cover!(length == 24 && additional == 25, "79 00 18 is overlong");
        kani::cover!(additional == 26, "four argument bytes are overlong");
        kani::cover!(additional == 27, "eight argument bytes are overlong");
    }
}

#[kani::proof]
#[kani::unwind(4)]
fn short_payload_ends_at_its_value_in_both_orders() {
    let longer_first: bool = kani::any();
    let (wire, start, end) = if longer_first {
        (&LONGER_FIRST_WIRE, 31, 54)
    } else {
        (&CANONICAL_WIRE, 4, 27)
    };
    verify_payload(wire, start, end, 23, SHORT_TEXT);
    kani::cover!(longer_first, "short payload ends at 54");
    kani::cover!(!longer_first, "short payload ends at 27");
}

#[kani::proof]
#[kani::unwind(4)]
fn long_payload_ends_at_its_value_in_both_orders() {
    let longer_first: bool = kani::any();
    let (wire, start, end) = if longer_first {
        (&LONGER_FIRST_WIRE, 5, 29)
    } else {
        (&CANONICAL_WIRE, 30, 54)
    };
    verify_payload(wire, start, end, 24, LONG_TEXT);
    kani::cover!(longer_first, "long payload ends at 29");
    kani::cover!(!longer_first, "long payload ends at 54");
}

fn verify_payload(wire: &[u8; WIRE_LEN], start: usize, end: usize, length: u64, text: &str) {
    let Ok(payload) = declared_contents(wire, start, length) else {
        kani::assert(false, "the complete text body is present");
        return;
    };
    assert!(core::ptr::eq(payload, &wire[start..end]));
    assert!(payload.len() == text.len() && checked_end(start, payload.len()) == Ok(end));
    // Exact equality to a valid str establishes the UTF-8 precondition
    // without unwinding the standard library's validator and allocator.
    let index: usize = kani::any();
    kani::assume(index < text.len());
    assert!(payload[index] == text.as_bytes()[index]);
}

#[kani::proof]
#[kani::unwind(5)]
fn traversal_supplies_exact_spans_and_rejects_the_longer_first_wire() {
    let longer_first: bool = kani::any();
    let source = if longer_first {
        &LONGER_FIRST_WIRE
    } else {
        &CANONICAL_WIRE
    };
    let mut budget = DecodeBudget::default();
    assert!(open_map(source, 0, 0, &mut budget) == (1, 1, 1));
    assert!(read_value_head(source, 1, &mut budget) == Ok((MT_STRING, 0, 2)));
    assert!(open_map(source, 2, 1, &mut budget) == (3, 2, 2));
    assert!(budget.seen == 3 && budget.pending == 4);

    let mut order = MapKeyOrder::default();
    let mut entries = Vec::with_capacity(2);
    let mut spans = [(0, 0); 2];
    let mut calls = 0;
    let result = collect_map_entries(2, 3, &mut entries, |start| {
        assert!(calls < 2);
        // The leaf checks establish the end of the text at each cursor. The
        // entry checks establish that admission receives that exact range.
        let (expected_start, end, bit) = match (longer_first, calls) {
            (false, 0) => (3, 27, false),
            (false, _) => (28, 54, true),
            (true, 0) => (3, 29, true),
            (true, _) => (30, 54, false),
        };
        assert!(start == expected_start);
        spans[calls] = (start, end);
        calls += 1;
        budget.enter()?;
        admit_map_key(&mut order, source, start..end)?;
        let (major, value, next) = read_value_head(source, end, &mut budget)?;
        assert!(major == 7 && value == if bit { 21 } else { 20 });
        Ok((bit, next))
    });
    assert!(calls == 2);
    if longer_first {
        assert!(spans == [(3, 29), (30, 54)] && result == Err(CborError::MapKeysOutOfOrder));
        assert!(matches!(entries.as_slice(), [true]));
        assert!(budget.seen == 6 && budget.pending == 1);
        kani::cover!(
            result.is_err(),
            "longer-first traversal stops before the second value"
        );
    } else {
        assert!(spans == [(3, 27), (28, 54)] && result == Ok(WIRE_LEN));
        assert!(matches!(entries.as_slice(), [false, true]));
        assert!(budget.seen == 7 && budget.pending == 0);
        kani::cover!(
            result.is_ok(),
            "canonical traversal consumes the complete wire"
        );
    }
}

#[kani::proof]
#[kani::unwind(5)]
fn entry_forwards_the_text_span_before_reading_its_value() {
    let longer_first: bool = kani::any();
    let second: bool = kani::any();
    let (wire, start, end) = match (longer_first, second) {
        (false, false) => (&CANONICAL_WIRE, 3, 27),
        (false, true) => (&CANONICAL_WIRE, 28, 54),
        (true, false) => (&LONGER_FIRST_WIRE, 3, 29),
        (true, true) => (&LONGER_FIRST_WIRE, 30, 54),
    };
    let reject = longer_first && second;
    let bit = longer_first != second;
    let expected_text = if bit { LONG_TEXT } else { SHORT_TEXT };
    let mut calls = 0;
    let mut observed = None;
    let mut budget = start_budget(second);
    let result = ManuallyDrop::new(decode_map_entry(
        wire,
        start,
        2,
        &mut budget,
        |source, offset, depth, _| {
            assert!(core::ptr::eq(source, wire.as_slice()) && depth == 2);
            calls += 1;
            if calls == 1 {
                assert!(offset == start);
                // Use the proved text on success. On rejection only the
                // type and cursor reach admission, so an empty owner keeps
                // this check independent of the full buffer wipe.
                let text = if reject {
                    String::new()
                } else {
                    String::from(expected_text)
                };
                Ok((CborValue::String(text), end))
            } else {
                assert!(calls == 2 && offset == end);
                Ok((CborValue::Bool(bit), end + 1))
            }
        },
        |span| {
            observed = Some((span.start, span.end));
            if reject {
                Err(CborError::MapKeysOutOfOrder)
            } else {
                Ok(())
            }
        },
    ));
    assert!(observed == Some((start, end)));
    if reject {
        assert!(calls == 1 && matches!(&*result, Err(CborError::MapKeysOutOfOrder)));
    } else {
        let Ok(((text, CborValue::Bool(value)), next)) = &*result else {
            kani::assert(false, "admitted entry retains its text and boolean");
            return;
        };
        assert!(calls == 2 && *value == bit && *next == end + 1);
        assert!(text.len() == expected_text.len());
        let index: usize = kani::any();
        kani::assume(index < expected_text.len());
        assert!(text.as_bytes()[index] == expected_text.as_bytes()[index]);
    }
    kani::cover!(
        !longer_first && !second,
        "canonical first span reaches admission"
    );
    kani::cover!(
        !longer_first && second,
        "canonical second span reaches admission"
    );
    kani::cover!(
        longer_first && !second,
        "reversed first span reaches admission"
    );
    kani::cover!(
        longer_first && second && calls == 1,
        "reversed second key's value stays unread"
    );
}

fn start_budget(second: bool) -> DecodeBudget {
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
