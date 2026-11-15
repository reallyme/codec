// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{
    check_sorted_runs, check_unique_by, map_key_order_by, merge_sort, Budget, CborError,
    ErrorReason, KeyStarts, INLINE_KEYS,
};
use crate::general::compare::maps_equivalent_by;
use crate::general::{read_header_prefix, Limits};
use core::cmp::Ordering;

const FIRST: usize = 0;
const DECOY: usize = 8;
const LAST_INLINE: usize = 15;
const LAST_INDEXED: usize = 16;
const SORTED: [usize; 17] = [1, 2, 3, 4, 5, 6, 7, 9, 10, 11, 12, 13, 14, 15, 0, 8, 16];

#[kani::proof]
#[kani::unwind(18)]
fn key_offsets_survive_spilling_the_inline_index() {
    assert_eq!(INLINE_KEYS, 16);
    let indexed: bool = kani::any();
    let count: usize = if indexed { 17 } else { 16 };
    let offsets: [usize; 17] = kani::any();
    let mut keys = KeyStarts::new();
    for &offset in &offsets[..count] {
        keys.push(offset).unwrap();
    }
    let stored = keys.as_slice().unwrap();
    assert_eq!(stored.len(), count);
    for (&actual, &expected) in stored.iter().zip(&offsets[..count]) {
        assert_eq!(actual, expected);
    }
    assert_eq!(keys.heap.is_empty(), !indexed);
    kani::cover!(!indexed, "sixteen key offsets remain inline");
    kani::cover!(indexed, "seventeenth key spills without losing offsets");
}

#[kani::proof]
#[kani::unwind(9)]
fn reordered_map_keys_compare_equal_and_reach_the_same_preorder_class() {
    use MapKind::{ChangedSecond, Decoy, Original};
    let (left_kind, right_kind): (MapKind, MapKind) = kani::any();
    let (left_reordered, right_reordered): (bool, bool) = kani::any();
    let left = map_fixture_body(left_kind, left_reordered);
    let right = map_fixture_body(right_kind, right_reordered);

    // These are map bodies after structural framing. Each child is a
    // complete one-octet unsigned integer, whose basic equality is its
    // decoded argument. Runtime regressions supply the same received maps.
    let mut budget = Budget::new(Limits::default());
    assert_eq!(
        map_key_order_by(&left, &right, &mut budget, read_fixture_uint),
        Ok(Ordering::Equal),
    );
    assert_eq!(
        maps_equivalent_by(&left, &right, &mut budget, read_fixture_uint, |a, b, _| Ok(
            a == b
        )),
        Ok(left_kind == right_kind),
    );
    kani::cover!(
        left_kind == right_kind && left_reordered != right_reordered,
        "reordered entries describe the same key"
    );
    kani::cover!(
        left_kind == right_kind && left_reordered == right_reordered,
        "same entry order describes the same key"
    );
    kani::cover!(
        left_kind == Original && right_kind == Decoy && left_reordered != right_reordered,
        "decoy differs from the original"
    );
    kani::cover!(
        left_kind == Original && right_kind == ChangedSecond && left_reordered != right_reordered,
        "changed value breaks equality without breaking the tie"
    );
    kani::cover!(
        left_kind == Decoy && right_kind == ChangedSecond,
        "decoy differs from the distinct-key control"
    );
}

#[derive(Clone, Copy, Eq, PartialEq, kani::Arbitrary)]
enum MapKind {
    Original,
    Decoy,
    ChangedSecond,
}

fn map_fixture_body(kind: MapKind, reordered: bool) -> [u8; 4] {
    let (first, second) = match kind {
        MapKind::Original => (0x01, 0x02),
        MapKind::Decoy => (0x00, 0x02),
        MapKind::ChangedSecond => (0x01, 0x03),
    };
    if reordered {
        [0x01, second, 0x00, first]
    } else {
        [0x00, first, 0x01, second]
    }
}

fn read_fixture_uint(input: &[u8], offset: &mut usize, _: &mut Budget) -> Result<u64, CborError> {
    let octet = *input.get(*offset).unwrap();
    assert!(octet <= 3);
    *offset = offset.checked_add(1).unwrap();
    Ok(u64::from(octet))
}

#[kani::proof]
#[kani::unwind(18)]
fn children_of_the_map_fixture_are_single_octet_unsigned_values() {
    let octet: u8 = kani::any();
    kani::assume(octet <= 3);
    let header = read_header_prefix(core::slice::from_ref(&octet)).unwrap();
    assert!(header.received_bytes().len() == 1);
    assert_eq!(header.major_type(), crate::general::MajorType::Unsigned);
    assert_eq!(header.argument(), Some(u64::from(octet)));
    kani::cover!(octet == 0, "first map key");
    kani::cover!(octet == 3, "changed map value");
}

#[kani::proof]
#[kani::unwind(18)]
fn sixteen_entries_visit_the_distant_equivalent_pair_inline() {
    let changed: bool = kani::any();
    let result = check_unique_by(
        16,
        &mut Budget::new(Limits::default()),
        |_, _, _| {
            kani::assert(false, "inline traversal does not invoke ordering");
            Ok(Ordering::Equal)
        },
        |left, right, _| Ok(fixture_equal(left, right, LAST_INLINE, changed)),
    );
    assert_duplicate_outcome(result, changed);
    kani::cover!(!changed, "sixteen entries reject equivalent keys");
    kani::cover!(changed, "inline traversal accepts distinct ties");
}

#[kani::proof]
#[kani::unwind(18)]
fn merge_sort_keeps_all_three_compound_candidates_in_one_stable_run() {
    let mut order = core::array::from_fn::<_, 17, _>(|index| index);
    let mut scratch = order;
    merge_sort(
        &mut order,
        &mut scratch,
        &mut Budget::new(Limits::default()),
        &mut fixture_compare,
    )
    .unwrap();
    for (actual, expected) in order.into_iter().zip(SORTED) {
        assert_eq!(actual, expected);
    }
    kani::cover!(
        order[14] == FIRST && order[15] == DECOY && order[16] == LAST_INDEXED,
        "the shared class retains the distinct decoy between its candidates"
    );
}

#[kani::proof]
#[kani::unwind(18)]
fn seventeen_entries_test_nonadjacent_members_of_the_tie_run() {
    let changed: bool = kani::any();
    let mut saw_decoy = false;
    let result = check_sorted_runs(
        &SORTED,
        &mut Budget::new(Limits::default()),
        &mut fixture_compare,
        &mut |left, right, _| {
            saw_decoy |= left == FIRST && right == DECOY;
            Ok(fixture_equal(left, right, LAST_INDEXED, changed))
        },
    );
    assert!(saw_decoy);
    assert_duplicate_outcome(result, changed);
    kani::cover!(
        !changed,
        "seventeen entries reject nonadjacent equivalent keys"
    );
    kani::cover!(
        changed,
        "sorted traversal accepts semantically distinct ties"
    );
}

fn fixture_compare(left: usize, right: usize, _: &mut Budget) -> Result<Ordering, CborError> {
    // All map keys share a class after the distinct unsigned fillers.
    let class = |index| match index {
        FIRST | DECOY | LAST_INDEXED => 17,
        other => other,
    };
    Ok(class(left).cmp(&class(right)))
}

fn fixture_equal(left: usize, right: usize, last: usize, changed: bool) -> bool {
    left == right
        || (!changed && ((left == FIRST && right == last) || (left == last && right == FIRST)))
}

fn assert_duplicate_outcome(result: Result<(), CborError>, changed: bool) {
    if changed {
        assert_eq!(result, Ok(()));
    } else {
        assert!(matches!(result, Err(error) if error.reason() == ErrorReason::DuplicateMapKey));
    }
}
