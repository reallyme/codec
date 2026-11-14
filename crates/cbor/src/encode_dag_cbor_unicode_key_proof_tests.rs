// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{
    encode_leaf, encode_value_with, sort_map_entries, write_map_entries, CborError, CborValue,
    EncodingSink, ManuallyDrop,
};
use crate::nested_map_proof_tests::unicode_keys::{
    assert_canonical, CANONICAL, DECOMPOSED, PRECOMPOSED,
};

#[kani::proof]
#[kani::unwind(5)]
fn sorting_keeps_both_spellings_and_their_values() {
    let reversed: bool = kani::any();
    let [short, long] = entries();
    let entries = ManuallyDrop::new(if reversed {
        [long, short]
    } else {
        [short, long]
    });
    let Ok(sorted) = sort_map_entries(&*entries) else {
        kani::assert(false, "the two spellings are distinct keys");
        return;
    };
    let [short, long] = sorted.as_slice() else {
        kani::assert(false, "sorting retains both entries");
        return;
    };
    let short_index = usize::from(reversed);
    let long_index = usize::from(!reversed);
    assert!(core::ptr::eq(short.0, &entries[short_index].0));
    assert!(core::ptr::eq(short.1, &entries[short_index].1));
    assert!(core::ptr::eq(long.0, &entries[long_index].0));
    assert!(core::ptr::eq(long.1, &entries[long_index].1));
    assert!(
        matches!(short.0.as_bytes(), [0xc3, 0xa9]) && matches!(short.1, CborValue::Bool(false))
    );
    assert!(
        matches!(long.0.as_bytes(), [0x65, 0xcc, 0x81]) && matches!(long.1, CborValue::Bool(true))
    );
    kani::cover!(
        reversed,
        "distinct spellings sorted from reversed entry order"
    );
    kani::cover!(
        !reversed,
        "distinct spellings remain in canonical entry order"
    );
}

#[kani::proof]
#[kani::unwind(5)]
fn sorting_rejects_either_spelling_repeated_with_distinct_values() {
    let precomposed: bool = kani::any();
    let text = if precomposed { PRECOMPOSED } else { DECOMPOSED };
    let entries = ManuallyDrop::new([
        (String::from(text), CborValue::Bool(false)),
        (String::from(text), CborValue::Bool(true)),
    ]);
    let result = sort_map_entries(&*entries);
    assert!(matches!(result, Err(CborError::DuplicateMapKey)));
    kani::cover!(
        precomposed && result.is_err(),
        "repeated precomposed text rejected"
    );
    kani::cover!(
        !precomposed && result.is_err(),
        "repeated decomposed text rejected"
    );
}

#[kani::proof]
#[kani::unwind(5)]
fn counting_sink_reserves_bytes_for_both_spellings() {
    let entries = ManuallyDrop::new(entries());
    let sorted = [
        (&entries[0].0, &entries[0].1),
        (&entries[1].0, &entries[1].1),
    ];
    // The outer prefix occupies two bytes. The maps and keys account for
    // five nodes before the shared writer visits the two boolean leaves.
    let mut length = 2;
    let mut nodes = 5;
    assert!(write_map_entries(&sorted, &mut length, 2, &mut nodes, encode_leaf).is_ok());
    assert!(length == CANONICAL.len() && nodes == 7);
    kani::cover!(
        length == CANONICAL.len(),
        "output capacity includes both UTF-8 spellings"
    );
}

#[kani::proof]
#[kani::unwind(5)]
fn byte_sink_reproduces_both_spellings_exactly() {
    let entries = ManuallyDrop::new(entries());
    // Sorting establishes these exact pairs from either source order.
    let sorted = [
        (&entries[0].0, &entries[0].1),
        (&entries[1].0, &entries[1].1),
    ];
    let mut output = ManuallyDrop::new(Vec::with_capacity(CANONICAL.len()));
    output.extend_from_slice(&[0xa1, 0x60]);
    let mut nodes = 5;
    assert!(write_map_entries(&sorted, &mut *output, 2, &mut nodes, encode_leaf).is_ok());
    assert!(nodes == 7 && output.capacity() == CANONICAL.len());
    assert_canonical(&output);
    kani::cover!(
        output.len() == CANONICAL.len(),
        "both spellings were written in full"
    );
}

#[kani::proof]
#[kani::unwind(5)]
fn outer_map_preserves_the_unicode_child_suffix() {
    let outer = ManuallyDrop::new(CborValue::Map(vec![(
        String::new(),
        CborValue::Map(Vec::from(entries())),
    )]));
    let CborValue::Map(entries) = &*outer else {
        kani::assert(false, "the proof input has an outer map");
        return;
    };
    let mut length = 0;
    let mut nodes = 0;
    assert!(encode_value_with(
        &outer,
        &mut length,
        0,
        &mut nodes,
        |child, length, depth, nodes| {
            assert!(core::ptr::eq(child, &entries[0].1) && depth == 1);
            assert!(*length == 2 && *nodes == 2);
            *length = CANONICAL.len();
            *nodes = 7;
            Ok(())
        }
    )
    .is_ok());
    assert!(length == CANONICAL.len() && nodes == 7);
    let mut output = ManuallyDrop::new(Vec::with_capacity(length));
    nodes = 0;
    assert!(encode_value_with(
        &outer,
        &mut *output,
        0,
        &mut nodes,
        |child, bytes, depth, nodes| {
            assert!(core::ptr::eq(child, &entries[0].1) && depth == 1);
            assert!(*nodes == 2 && matches!(bytes.as_slice(), [0xa1, 0x60]));
            assert!(bytes.capacity() == CANONICAL.len());
            // The inner byte sink establishes this suffix with the same prefix.
            EncodingSink::append(bytes, &CANONICAL[2..])?;
            *nodes = 7;
            Ok(())
        }
    )
    .is_ok());
    assert!(nodes == 7);
    assert_canonical(&output);
    kani::cover!(
        output.len() == CANONICAL.len(),
        "outer writer keeps both key encodings unchanged"
    );
}

fn entries() -> [(String, CborValue); 2] {
    [
        (String::from(PRECOMPOSED), CborValue::Bool(false)),
        (String::from(DECOMPOSED), CborValue::Bool(true)),
    ]
}
