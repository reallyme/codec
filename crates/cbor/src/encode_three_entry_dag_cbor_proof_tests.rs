// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{
    count_node, encode_bool, encode_value_with, sort_map_entries, write_map_entries, CborError,
    CborValue, EncodingSink, ManuallyDrop,
};
use crate::encode_dag_cbor::push_byte;
use crate::nested_map_proof_tests::three_entry::{
    assert_canonical_wire, cover_permutations, Key, CANONICAL_KEYS, CANONICAL_WIRE,
};

#[kani::proof]
#[kani::unwind(6)]
fn sort_produces_the_same_pairs_from_every_permutation() {
    use Key::{Aa, A, Z};
    let keys = [Key::symbolic(), Key::symbolic(), Key::symbolic()];
    kani::assume(keys[0] != keys[1] && keys[0] != keys[2] && keys[1] != keys[2]);
    // Construct the fixed owners before selecting their order. Each source
    // entry has its own live allocation for the duration of the sort.
    let [a, z, aa] = entries(CANONICAL_KEYS);
    let entries = ManuallyDrop::new(match keys {
        [A, Z, Aa] => [a, z, aa],
        [A, Aa, Z] => [a, aa, z],
        [Z, A, Aa] => [z, a, aa],
        [Z, Aa, A] => [z, aa, a],
        [Aa, A, Z] => [aa, a, z],
        [Aa, Z, A] => [aa, z, a],
        _ => {
            kani::assert(
                false,
                "the proof visits only permutations of the three keys",
            );
            return;
        }
    });
    verify_sorted_pairs(&entries);
    cover_permutations(keys);
}

fn verify_sorted_pairs(entries: &[(String, CborValue); 3]) {
    let Ok(sorted) = sort_map_entries(entries) else {
        kani::assert(false, "the three distinct keys sort successfully");
        return;
    };
    assert!(matches!(sorted.as_slice(),
        [(a, CborValue::Bool(false)), (z, CborValue::Bool(true)), (aa, CborValue::Null)]
            if matches!(a.as_bytes(), [b'a']) && matches!(z.as_bytes(), [b'z'])
                && matches!(aa.as_bytes(), [b'a', b'a'])));
}

#[kani::proof]
#[kani::unwind(6)]
fn counting_sink_reserves_the_complete_canonical_wire() {
    let inner = ManuallyDrop::new(CborValue::Map(Vec::from(entries(CANONICAL_KEYS))));
    let mut count = 2;
    let mut nodes = 2;
    // Every input order has the same counts after sorting its three pairs.
    assert!(encode_value_with(&inner, &mut count, 1, &mut nodes, encode_leaf).is_ok());
    assert!(count == CANONICAL_WIRE.len() && nodes == 9);
    kani::cover!(
        count == CANONICAL_WIRE.len(),
        "whole nested map length measured"
    );
}

#[kani::proof]
#[kani::unwind(5)]
fn byte_sink_preserves_each_key_and_reproduces_the_wire() {
    let entries = ManuallyDrop::new(entries(CANONICAL_KEYS));
    let sorted = [
        (&entries[0].0, &entries[0].1),
        (&entries[1].0, &entries[1].1),
        (&entries[2].0, &entries[2].1),
    ];
    let mut output = ManuallyDrop::new(Vec::with_capacity(CANONICAL_WIRE.len()));
    output.extend_from_slice(&[0xa1, 0x60]);
    let mut nodes = 6;
    assert!(write_map_entries(&sorted, &mut *output, 2, &mut nodes, encode_leaf).is_ok());
    assert!(nodes == 9 && output.capacity() == CANONICAL_WIRE.len());
    assert_canonical_wire(&output);
    kani::cover!(
        output.len() == CANONICAL_WIRE.len(),
        "all bytes written without growth"
    );
}

#[kani::proof]
#[kani::unwind(5)]
fn outer_map_preserves_the_three_entry_child_suffix() {
    let outer = ManuallyDrop::new(CborValue::Map(vec![(
        String::new(),
        CborValue::Map(Vec::from(entries(CANONICAL_KEYS))),
    )]));
    let CborValue::Map(entries) = &*outer else {
        kani::assert(false, "proof input is an outer map");
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
            // The counting sink establishes the whole length and node budget.
            *length = CANONICAL_WIRE.len();
            *nodes = 9;
            Ok(())
        }
    )
    .is_ok());
    assert!(length == CANONICAL_WIRE.len() && nodes == 9);
    let mut output = ManuallyDrop::new(Vec::with_capacity(length));
    nodes = 0;
    assert!(encode_value_with(
        &outer,
        &mut *output,
        0,
        &mut nodes,
        |child, bytes, depth, nodes| {
            assert!(core::ptr::eq(child, &entries[0].1));
            assert!(depth == 1 && *nodes == 2 && matches!(bytes.as_slice(), [0xa1, 0x60]));
            assert!(bytes.capacity() == CANONICAL_WIRE.len());
            EncodingSink::append(bytes, &CANONICAL_WIRE[2..])?;
            *nodes = 9;
            Ok(())
        }
    )
    .is_ok());
    assert!(nodes == 9);
    assert_canonical_wire(&output);
    kani::cover!(
        output.len() == CANONICAL_WIRE.len(),
        "outer map preserves the canonical suffix"
    );
}

fn entries([first, second, third]: [Key; 3]) -> [(String, CborValue); 3] {
    [
        first.owned_entry(),
        second.owned_entry(),
        third.owned_entry(),
    ]
}

fn encode_leaf(
    value: &CborValue,
    output: &mut impl EncodingSink,
    depth: usize,
    nodes: &mut usize,
) -> Result<(), CborError> {
    assert!(depth == 2);
    count_node(nodes)?;
    match value {
        CborValue::Bool(bit) => encode_bool(*bit, output),
        CborValue::Null => push_byte(output, 0xf6),
        _ => {
            kani::assert(
                false,
                "three-entry fixture has only boolean and null leaves",
            );
            Err(CborError::DisallowedMajorType { major: 5 })
        }
    }
}
