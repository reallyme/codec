// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{
    encode_leaf, encode_value_with, sort_map_entries, write_map_entries, CborValue, EncodingSink,
    ManuallyDrop,
};
use crate::encode_dag_cbor::{write_header, MT_STRING};
use crate::nested_map_proof_tests::text_key_boundary::{
    assert_canonical_wire, CANONICAL_WIRE, LONG_TEXT, SHORT_TEXT, WIRE_LEN,
};

#[kani::proof]
#[kani::unwind(4)]
fn writer_uses_one_head_byte_at_23_and_two_at_24() {
    let extended: bool = kani::any();
    let length = if extended { 24 } else { 23 };
    let mut head = Vec::with_capacity(2);
    assert!(write_header(MT_STRING, length, &mut head).is_ok());
    if extended {
        assert!(matches!(head.as_slice(), [0x78, 0x18]));
        kani::cover!(head.len() == 2, "24-byte text needs one argument byte");
    } else {
        assert!(matches!(head.as_slice(), [0x77]));
        kani::cover!(head.len() == 1, "23-byte text fits in the head byte");
    }
}

#[kani::proof]
#[kani::unwind(4)]
fn sorting_preserves_both_pairs_across_the_header_width_change() {
    let longer_first: bool = kani::any();
    let [short, long] = entries();
    let entries = ManuallyDrop::new(if longer_first {
        [long, short]
    } else {
        [short, long]
    });
    let Ok(sorted) = sort_map_entries(&*entries) else {
        kani::assert(false, "the two distinct boundary keys sort successfully");
        return;
    };
    let [short, long] = sorted.as_slice() else {
        kani::assert(false, "both entries remain after sorting");
        return;
    };
    let short_index = usize::from(longer_first);
    let long_index = usize::from(!longer_first);
    assert!(core::ptr::eq(short.0, &entries[short_index].0));
    assert!(core::ptr::eq(short.1, &entries[short_index].1));
    assert!(core::ptr::eq(long.0, &entries[long_index].0));
    assert!(core::ptr::eq(long.1, &entries[long_index].1));
    assert!(short.0.len() == 23 && matches!(short.1, CborValue::Bool(false)));
    assert!(long.0.len() == 24 && matches!(long.1, CborValue::Bool(true)));
    kani::cover!(longer_first, "longer-first pairs are reordered");
    kani::cover!(!longer_first, "canonical pairs retain their order");
}

#[kani::proof]
#[kani::unwind(5)]
fn counting_sink_accounts_for_the_extra_header_byte() {
    let value = ManuallyDrop::new(CborValue::Map(Vec::from(entries())));
    let mut length = 2;
    let mut nodes = 2;
    // Both input orders yield the same sorted pairs for this pass.
    assert!(encode_value_with(&value, &mut length, 1, &mut nodes, encode_leaf).is_ok());
    assert!(length == WIRE_LEN && nodes == 7);
    kani::cover!(length == WIRE_LEN, "counting sink reserves all 55 bytes");
}

#[kani::proof]
#[kani::unwind(5)]
fn inner_byte_sink_reproduces_the_full_wire() {
    let entries = ManuallyDrop::new(entries());
    let sorted = [
        (&entries[0].0, &entries[0].1),
        (&entries[1].0, &entries[1].1),
    ];
    let mut output = ManuallyDrop::new(Vec::with_capacity(WIRE_LEN));
    output.extend_from_slice(&[0xa1, 0x60]);
    let mut nodes = 5;
    assert!(write_map_entries(&sorted, &mut *output, 2, &mut nodes, encode_leaf).is_ok());
    assert!(nodes == 7 && output.capacity() == WIRE_LEN);
    assert_canonical_wire(&output);
    kani::cover!(
        output.len() == WIRE_LEN,
        "byte sink writes the nested map without growth"
    );
}

#[kani::proof]
#[kani::unwind(5)]
fn outer_map_preserves_the_boundary_child_suffix() {
    let value = ManuallyDrop::new(CborValue::Map(vec![(
        String::new(),
        CborValue::Map(Vec::from(entries())),
    )]));
    let CborValue::Map(entries) = &*value else {
        kani::assert(false, "the proof input has an outer map");
        return;
    };
    let mut length = 0;
    let mut nodes = 0;
    assert!(encode_value_with(
        &value,
        &mut length,
        0,
        &mut nodes,
        |child, length, depth, nodes| {
            assert!(core::ptr::eq(child, &entries[0].1));
            assert!(depth == 1 && *nodes == 2 && *length == 2);
            *length = WIRE_LEN;
            *nodes = 7;
            Ok(())
        }
    )
    .is_ok());
    assert!(length == WIRE_LEN && nodes == 7);

    let mut output = ManuallyDrop::new(Vec::with_capacity(length));
    nodes = 0;
    assert!(encode_value_with(
        &value,
        &mut *output,
        0,
        &mut nodes,
        |child, bytes, depth, nodes| {
            assert!(core::ptr::eq(child, &entries[0].1));
            assert!(depth == 1 && *nodes == 2 && matches!(bytes.as_slice(), [0xa1, 0x60]));
            assert!(bytes.capacity() == WIRE_LEN);
            // The inner byte check proves this suffix given the prefix and capacity.
            EncodingSink::append(bytes, &CANONICAL_WIRE[2..])?;
            *nodes = 7;
            Ok(())
        }
    )
    .is_ok());
    assert!(nodes == 7);
    assert_canonical_wire(&output);
    kani::cover!(output.len() == WIRE_LEN, "outer map preserves every byte");
}

fn entries() -> [(String, CborValue); 2] {
    [
        (String::from(SHORT_TEXT), CborValue::Bool(false)),
        (String::from(LONG_TEXT), CborValue::Bool(true)),
    ]
}
