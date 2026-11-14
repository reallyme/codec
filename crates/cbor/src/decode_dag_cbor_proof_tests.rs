// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{
    admit_map_key, collect_map_entries, decode_map_entries, decode_map_entry, decode_simple,
    decode_value_with, descend, extract_string, read_argument, read_value_head, require_text_key,
    reserve_map, CborError, CborValue, DecodeBudget, MapKeyOrder, MAX_NESTING_DEPTH, MT_MAP,
    MT_STRING,
};
use crate::nested_map_proof_tests::{
    MIXED_KEY_BYTEWISE, MIXED_KEY_CANONICAL, MIXED_LENGTH_CANONICAL, MIXED_LENGTH_REVERSED,
};
use core::mem::ManuallyDrop;
use zeroize::Zeroizing;

#[kani::proof]
#[kani::unwind(5)]
fn extended_argument_requires_the_shortest_encoding() {
    let additional: u8 = kani::any();
    let value: u64 = kani::any();
    let available: usize = kani::any();
    let width = match additional {
        24 => 1,
        25 => 2,
        26 => 4,
        27 => 8,
        _ => return,
    };
    let encoded = value.to_be_bytes();
    let argument = &encoded[encoded.len() - width..];
    kani::assume(available <= width);
    // The leading byte partitions the extended widths: one-byte arguments
    // start at 24, and wider arguments cannot have their high half all zero.
    let shortest = if width == 1 {
        argument[0] >= 24
    } else {
        argument[..width / 2].iter().any(|&byte| byte != 0)
    };
    let actual = read_argument(&argument[..available], 0, additional);
    kani::cover!(matches!(actual, Ok((_, 8))), "eight-byte argument");
    kani::cover!(
        actual == Err(CborError::NonCanonicalInteger),
        "overlong argument"
    );
    kani::cover!(
        actual == Err(CborError::TruncatedArgument),
        "truncated argument"
    );
    if available < width {
        assert_eq!(actual, Err(CborError::TruncatedArgument));
    } else if shortest {
        let decoded = value & (u64::MAX >> ((encoded.len() - width) * 8));
        assert_eq!(actual, Ok((decoded, width)));
    } else {
        assert_eq!(actual, Err(CborError::NonCanonicalInteger));
    }
}

#[kani::proof]
fn descent_enforces_the_depth_limit_without_wrapping() {
    let depth: usize = kani::any();
    let actual = descend(depth);
    kani::cover!(
        depth == MAX_NESTING_DEPTH - 1 && actual == Ok(MAX_NESTING_DEPTH),
        "depth reaches ceiling"
    );
    kani::cover!(
        depth == MAX_NESTING_DEPTH && actual == Err(CborError::DepthExceeded),
        "depth ceiling exceeded"
    );
    assert_eq!(
        actual,
        match depth {
            usize::MAX => Err(CborError::OffsetOverflow),
            _ if depth >= MAX_NESTING_DEPTH => Err(CborError::DepthExceeded),
            _ => Ok(depth + 1),
        }
    );
}

#[kani::proof]
#[kani::unwind(4)]
fn dag_canonical_first_key_supplies_exact_span() {
    let mut order = MapKeyOrder::default();
    let mut budget = inner_entry_budget();
    verify_text_entry::<3, 5, 6>(
        &MIXED_LENGTH_CANONICAL,
        b"z",
        false,
        false,
        &mut budget,
        &mut order,
    );
    assert!(budget.seen == 5 && budget.pending == 2);
    kani::cover!(budget.seen == 5, "first canonical entry consumed");
}

#[kani::proof]
#[kani::unwind(4)]
fn dag_canonical_second_key_supplies_exact_span() {
    let wire = &MIXED_LENGTH_CANONICAL;
    let mut order = MapKeyOrder::default();
    assert!(admit_map_key(&mut order, wire, 3..5).is_ok());
    let mut budget = DecodeBudget {
        seen: 5,
        pending: 2,
    };
    verify_text_entry::<6, 9, 10>(wire, b"aa", true, false, &mut budget, &mut order);
    assert!(budget.seen == 7 && budget.pending == 0);
    kani::cover!(budget.pending == 0, "second canonical entry consumed");
}

#[kani::proof]
#[kani::unwind(4)]
fn dag_longer_first_key_supplies_exact_span() {
    let mut order = MapKeyOrder::default();
    let mut budget = inner_entry_budget();
    verify_text_entry::<3, 6, 7>(
        &MIXED_LENGTH_REVERSED,
        b"aa",
        true,
        false,
        &mut budget,
        &mut order,
    );
    assert!(budget.seen == 5 && budget.pending == 2);
    kani::cover!(budget.seen == 5, "longer first key and value consumed");
}

#[kani::proof]
#[kani::unwind(4)]
fn dag_longer_first_second_key_supplies_exact_span_and_is_rejected() {
    let wire = &MIXED_LENGTH_REVERSED;
    let mut order = MapKeyOrder::default();
    assert!(admit_map_key(&mut order, wire, 3..6).is_ok());
    let mut budget = DecodeBudget {
        seen: 5,
        pending: 2,
    };
    verify_text_entry::<7, 9, 10>(wire, b"z", false, true, &mut budget, &mut order);
    assert!(budget.seen == 6 && budget.pending == 1);
    kani::cover!(budget.pending == 1, "second value remains unread");
}

#[kani::proof]
#[kani::unwind(4)]
fn dag_aa_decodes_at_the_first_key_offset() {
    let mut budget = inner_entry_budget();
    verify_aa_leaf::<3, 4, 6>(&MIXED_LENGTH_REVERSED, &mut budget);
    assert!(budget.seen == 4 && budget.pending == 3);
    kani::cover!(budget.seen == 4, "longer first key ends at offset 6");
}

#[kani::proof]
#[kani::unwind(4)]
fn dag_aa_decodes_at_the_second_key_offset() {
    let mut budget = DecodeBudget {
        seen: 5,
        pending: 2,
    };
    verify_aa_leaf::<6, 7, 9>(&MIXED_LENGTH_CANONICAL, &mut budget);
    assert!(budget.seen == 6 && budget.pending == 1);
    kani::cover!(budget.seen == 6, "longer second key ends at offset 9");
}

fn verify_aa_leaf<const START: usize, const PAYLOAD: usize, const END: usize>(
    wire: &[u8; 10],
    budget: &mut DecodeBudget,
) {
    assert!(read_value_head(wire, START, budget) == Ok((MT_STRING, 2, PAYLOAD)));
    let decoded = ManuallyDrop::new(extract_string(wire, PAYLOAD, 2));
    assert!(matches!(&*decoded, Ok((text, end))
        if *end == END && matches!(text.as_bytes(), [b'a', b'a'])));
}

fn inner_entry_budget() -> DecodeBudget {
    // Two map heads and the outer key have been read. Four inner nodes are
    // reserved by the two-entry map before its first key is decoded.
    DecodeBudget {
        seen: 3,
        pending: 4,
    }
}

fn verify_text_entry<'a, const START: usize, const END: usize, const NEXT: usize>(
    wire: &'a [u8; 10],
    key: &[u8],
    bit: bool,
    reject: bool,
    budget: &mut DecodeBudget,
    order: &mut MapKeyOrder<'a>,
) {
    let mut observed = None;
    let mut child_calls = 0;
    let result = ManuallyDrop::new(decode_map_entry(
        wire,
        START,
        2,
        budget,
        |bytes, offset, depth, budget| {
            assert!(core::ptr::eq(bytes, wire.as_slice()) && depth == 2);
            child_calls += 1;
            if child_calls == 1 {
                assert!(offset == START);
                if matches!(&wire[START..END], [0x62, b'a', b'a']) {
                    // The longer-key leaf checks supply the decoded text,
                    // end offset and one-node budget change at this cursor.
                    budget.enter()?;
                    return Ok((CborValue::String(String::from("aa")), END));
                }
                let (major, argument, payload) = read_value_head(wire, START, budget)?;
                assert!(major == MT_STRING);
                let (key, end) = extract_string(wire, payload, argument)?;
                Ok((CborValue::String(key), end))
            } else {
                assert!(child_calls == 2 && offset == END);
                // The boolean-leaf check establishes the decoded value and
                // one-byte advance for each boolean header on these wires.
                assert!(wire[END] == if bit { 0xf5 } else { 0xf4 });
                budget.enter()?;
                Ok((CborValue::Bool(bit), NEXT))
            }
        },
        |span| {
            observed = Some((span.start, span.end));
            admit_map_key(order, wire, span)
        },
    ));
    assert!(observed == Some((START, END)));
    if reject {
        assert!(matches!(&*result, Err(CborError::MapKeysOutOfOrder)));
        assert!(child_calls == 1);
    } else {
        assert!(matches!(&*result,
            Ok(((actual_key, CborValue::Bool(actual_bit)), next))
                if actual_key.as_bytes() == key && *actual_bit == bit && *next == NEXT
        ));
        assert!(child_calls == 2);
    }
}

fn decode_leaf(
    bytes: &[u8],
    offset: usize,
    depth: usize,
    budget: &mut DecodeBudget,
) -> Result<(CborValue, usize), CborError> {
    decode_value_with(bytes, offset, depth, budget, |_, _, _, _| {
        kani::assert(false, "the proof's leaf has no children");
        Err(CborError::UnexpectedEnd)
    })
}

#[kani::proof]
#[kani::unwind(5)]
fn dag_inner_traversal_preserves_canonical_entries_and_stops_on_rejection() {
    let long_first: bool = kani::any();
    let wire = if long_first {
        &MIXED_LENGTH_REVERSED
    } else {
        &MIXED_LENGTH_CANONICAL
    };
    let mut budget = before_inner_map();
    let (offset, count, depth) = open_map(wire, 2, 1, &mut budget);
    assert!(offset == 3 && count == 2 && depth == 2);
    assert!(budget.seen == 3 && budget.pending == 4);
    let mut entries = Vec::with_capacity(count);
    let mut visited = 0;
    let result = collect_map_entries(count, offset, &mut entries, |start| {
        visited += 1;
        // The entry checks establish these outcomes and the next key offset.
        match (long_first, visited) {
            (false, 1) => {
                assert!(start == 3);
                Ok(((b"z".as_slice(), false), 6))
            }
            (false, 2) => {
                assert!(start == 6);
                Ok(((b"aa".as_slice(), true), 10))
            }
            (true, 1) => {
                assert!(start == 3);
                Ok(((b"aa".as_slice(), true), 7))
            }
            (true, 2) => {
                assert!(start == 7);
                Err(CborError::MapKeysOutOfOrder)
            }
            _ => {
                kani::assert(false, "the traversal visits only the reserved entries");
                Err(CborError::UnexpectedEnd)
            }
        }
    });
    assert!(visited == 2);
    if long_first {
        assert_eq!(result, Err(CborError::MapKeysOutOfOrder));
        assert!(matches!(entries.as_slice(), [(key, true)] if matches!(key, [b'a', b'a'])));
        kani::cover!(
            result.is_err(),
            "longer-first traversal propagates key rejection"
        );
    } else {
        assert_eq!(result, Ok(10));
        assert!(matches!(entries.as_slice(), [(short, false), (long, true)]
            if matches!(short, [b'z']) && matches!(long, [b'a', b'a'])));
        kani::cover!(
            result.is_ok(),
            "canonical traversal consumes the complete wire"
        );
    }
}

#[kani::proof]
#[kani::unwind(5)]
fn dag_outer_traversal_hands_off_inner_map_after_empty_key() {
    let long_first: bool = kani::any();
    let wire = if long_first {
        &MIXED_LENGTH_REVERSED
    } else {
        &MIXED_LENGTH_CANONICAL
    };
    let mut budget = DecodeBudget::default();
    let mut calls = 0;
    let mut entries = ManuallyDrop::new(Zeroizing::new(Vec::with_capacity(1)));
    let (offset, count, depth) = open_map(wire, 0, 0, &mut budget);
    assert!(offset == 1 && count == 1 && depth == 1);
    let result = decode_map_entries(
        wire,
        offset,
        depth,
        count,
        &mut budget,
        &mut entries,
        |source, offset, depth, budget| {
            assert!(core::ptr::eq(source, wire.as_slice()) && depth == 1);
            calls += 1;
            if calls == 1 {
                assert!(offset == 1 && budget.seen == 1 && budget.pending == 2);
                let result = decode_leaf(source, offset, depth, budget);
                assert!(matches!(&result, Ok((CborValue::String(key), 2)) if key.is_empty()));
                return result;
            }
            assert!(calls == 2 && offset == 2 && budget.seen == 2 && budget.pending == 1);
            // The inner traversal establishes the returned tree, budget and error.
            if long_first {
                budget.seen = 6;
                budget.pending = 1;
                return Err(CborError::MapKeysOutOfOrder);
            }
            budget.seen = 7;
            budget.pending = 0;
            Ok((
                CborValue::Map(vec![
                    (String::from("z"), CborValue::Bool(false)),
                    (String::from("aa"), CborValue::Bool(true)),
                ]),
                10,
            ))
        },
    );
    assert!(calls == 2);
    if long_first {
        assert_eq!(result, Err(CborError::MapKeysOutOfOrder));
        assert!(entries.is_empty() && budget.seen == 6 && budget.pending == 1);
        kani::cover!(
            entries.is_empty(),
            "outer map propagates inner admission error"
        );
    } else {
        assert_eq!(result, Ok(10));
        assert!(matches!(entries.as_slice(), [(key, CborValue::Map(inner))]
            if key.is_empty() && is_expected_inner_entries(inner)));
        assert!(budget.seen == 7 && budget.pending == 0);
        kani::cover!(
            budget.pending == 0,
            "outer map preserves the complete tree and end offset"
        );
    }
}

fn open_map(
    wire: &[u8],
    start: usize,
    depth: usize,
    budget: &mut DecodeBudget,
) -> (usize, usize, usize) {
    let Ok((MT_MAP, argument, offset)) = read_value_head(wire, start, budget) else {
        kani::assert(false, "the proof input has a map head");
        return (0, 0, 0);
    };
    let Ok((count, child_depth)) = reserve_map(wire.len(), offset, depth, argument, budget) else {
        kani::assert(false, "the proof input can reserve its map entries");
        return (0, 0, 0);
    };
    (offset, count, child_depth)
}

fn before_inner_map() -> DecodeBudget {
    DecodeBudget {
        seen: 2,
        pending: 1,
    }
}

fn is_expected_inner_entries(entries: &[(String, CborValue)]) -> bool {
    matches!(entries, [(short, CborValue::Bool(false)), (long, CborValue::Bool(true))]
            if matches!(short.as_bytes(), [b'z']) && matches!(long.as_bytes(), [b'a', b'a']))
}

#[kani::proof]
#[kani::unwind(4)]
fn dag_unsigned_256_decodes_as_an_integer_in_either_key_position() {
    let unsigned_first: bool = kani::any();
    let (wire, start, end, mut budget) = if unsigned_first {
        (&MIXED_KEY_BYTEWISE, 3, 6, inner_entry_budget())
    } else {
        (
            &MIXED_KEY_CANONICAL,
            6,
            9,
            DecodeBudget {
                seen: 5,
                pending: 2,
            },
        )
    };
    let result = ManuallyDrop::new(decode_leaf(wire, start, 2, &mut budget));
    assert!(matches!(&*result, Ok((CborValue::Int(256), next)) if *next == end));
    kani::cover!(unsigned_first, "first-position integer spans 3..6");
    kani::cover!(!unsigned_first, "second-position integer spans 6..9");
}

#[kani::proof]
fn dag_map_key_type_requires_text_for_unsigned_256() {
    // The two-position argument check establishes this ordinary integer.
    let mut key = ManuallyDrop::new(CborValue::Int(256));
    let result = require_text_key(&mut key);
    assert!(matches!(result, Err(CborError::MapKeyMustBeString)));
    kani::cover!(
        result.is_err(),
        "unsigned 256 rejected at the map key type check"
    );
}

#[kani::proof]
#[kani::unwind(3)]
fn dag_boolean_leaves_advance_one_byte() {
    let bit: bool = kani::any();
    let wire = [if bit { 0xf5 } else { 0xf4 }];
    let mut budget = DecodeBudget {
        seen: 4,
        pending: 3,
    };
    let result = (|| {
        let (major, argument, end) = read_value_head(&wire, 0, &mut budget)?;
        assert!(major == 7);
        Ok::<_, CborError>((decode_simple(argument)?, end))
    })();
    let result = ManuallyDrop::new(result);
    assert!(matches!(&*result, Ok((CborValue::Bool(value), 1)) if *value == bit));
    assert!(budget.seen == 5 && budget.pending == 2);
    kani::cover!(bit, "true leaf advances one byte");
    kani::cover!(!bit, "false leaf advances one byte");
}
