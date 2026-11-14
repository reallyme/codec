// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{
    count_node, encode_bool, encode_value_with, sort_map_entries, write_map_entries, CborError,
    CborValue, EncodingSink,
};
use crate::nested_map_proof_tests::{assert_mixed_length_wire, MIXED_LENGTH_CANONICAL};
use core::mem::ManuallyDrop;

#[kani::proof]
#[kani::unwind(5)]
fn dag_inner_counting_sink_sizes_either_entry_order() {
    let long_first: bool = kani::any();
    let inner = ManuallyDrop::new(inner_value(long_first));
    // The outer map has counted its header, empty key, and two nodes.
    let mut length = 2;
    let mut nodes = 2;
    assert!(encode_value_with(&inner, &mut length, 1, &mut nodes, encode_leaf).is_ok());
    assert!(length == MIXED_LENGTH_CANONICAL.len() && nodes == 7);
    kani::cover!(long_first && nodes == 7, "reversed entries counted");
    kani::cover!(!long_first && nodes == 7, "canonical entries counted");
}

#[kani::proof]
#[kani::unwind(4)]
fn dag_inner_sort_keeps_each_value_with_its_key() {
    let long_first: bool = kani::any();
    let entries = ManuallyDrop::new(inner_entries(long_first));
    let sorted = sort_map_entries(&*entries);
    let Ok(sorted) = sorted else {
        kani::assert(false, "two distinct keys sort successfully");
        return;
    };
    let [short, long] = sorted.as_slice() else {
        kani::assert(false, "sorting returns both entries");
        return;
    };
    let short_index = usize::from(long_first);
    let long_index = usize::from(!long_first);
    assert!(core::ptr::eq(short.0, &entries[short_index].0));
    assert!(core::ptr::eq(short.1, &entries[short_index].1));
    assert!(core::ptr::eq(long.0, &entries[long_index].0));
    assert!(core::ptr::eq(long.1, &entries[long_index].1));
    kani::cover!(long_first, "longer-first entries reordered by length");
    kani::cover!(!long_first, "canonical entries preserve their order");
}

#[kani::proof]
#[kani::unwind(4)]
fn dag_inner_byte_sink_writes_every_canonical_byte() {
    let entries = ManuallyDrop::new(inner_entries(false));
    // Sorting establishes these exact pairs from either input order.
    let sorted = [
        (&entries[0].0, &entries[0].1),
        (&entries[1].0, &entries[1].1),
    ];
    let mut output = ManuallyDrop::new(Vec::with_capacity(MIXED_LENGTH_CANONICAL.len()));
    output.extend_from_slice(&[0xa1, 0x60]);
    // Before writing, the outer map and the inner map have counted their keys.
    let mut nodes = 5;
    assert!(write_map_entries(&sorted, &mut *output, 2, &mut nodes, encode_leaf).is_ok());
    assert!(nodes == 7 && output.capacity() == MIXED_LENGTH_CANONICAL.len());
    assert_mixed_length_wire(&output);
    kani::cover!(
        nodes == 7 && output.len() == 10,
        "byte sink writes the complete canonical wire"
    );
}

#[kani::proof]
#[kani::unwind(5)]
fn dag_outer_map_passes_the_inner_value_after_its_prefix() {
    let long_first: bool = kani::any();
    let outer = ManuallyDrop::new(CborValue::Map(vec![(
        String::new(),
        inner_value(long_first),
    )]));
    let CborValue::Map(entries) = &*outer else {
        kani::assert(false, "outer input is a map");
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
            assert!(core::ptr::eq(child, &entries[0].1));
            assert!(depth == 1 && *nodes == 2 && *length == 2);
            // The inner counting harness establishes these terminal counts.
            *nodes = 7;
            *length = MIXED_LENGTH_CANONICAL.len();
            Ok(())
        }
    )
    .is_ok());
    assert!(length == MIXED_LENGTH_CANONICAL.len() && nodes == 7);

    let mut output = ManuallyDrop::new(Vec::with_capacity(length));
    nodes = 0;
    assert!(encode_value_with(
        &outer,
        &mut *output,
        0,
        &mut nodes,
        |child, output, depth, nodes| {
            assert!(core::ptr::eq(child, &entries[0].1));
            assert!(depth == 1 && *nodes == 2);
            assert!(matches!(output.as_slice(), [0xa1, 0x60]));
            assert!(output.capacity() == MIXED_LENGTH_CANONICAL.len());
            // The inner byte sink establishes this suffix from either entry order.
            EncodingSink::append(output, &MIXED_LENGTH_CANONICAL[2..])?;
            *nodes = 7;
            Ok(())
        }
    )
    .is_ok());
    assert!(nodes == 7);
    assert_mixed_length_wire(&output);
    kani::cover!(
        long_first && nodes == 7,
        "outer writer preserves the sorted suffix"
    );
    kani::cover!(
        !long_first && nodes == 7,
        "outer writer preserves the canonical suffix"
    );
}

fn inner_value(long_first: bool) -> CborValue {
    CborValue::Map(Vec::from(inner_entries(long_first)))
}

fn inner_entries(long_first: bool) -> [(String, CborValue); 2] {
    let short = (String::from("z"), CborValue::Bool(false));
    let long = (String::from("aa"), CborValue::Bool(true));
    if long_first {
        [long, short]
    } else {
        [short, long]
    }
}

fn encode_leaf(
    value: &CborValue,
    output: &mut impl EncodingSink,
    depth: usize,
    nodes: &mut usize,
) -> Result<(), CborError> {
    assert!(depth == 2);
    let CborValue::Bool(bit) = value else {
        kani::assert(false, "the inner map reaches only its boolean leaves");
        return Err(CborError::DisallowedMajorType { major: 5 });
    };
    count_node(nodes)?;
    encode_bool(*bit, output)
}
