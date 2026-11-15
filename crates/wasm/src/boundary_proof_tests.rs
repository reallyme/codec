// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{
    checked_input_total, matches_observed_length, snapshot_capacity, snapshot_copy_is_complete,
};

#[kani::proof]
fn wasm_snapshot_allocation_requires_an_admitted_source_and_exact_view() {
    let length: u32 = kani::any();
    let maximum: usize = kani::any();
    let bounded_view: Option<u32> = kani::any();
    let capacity = snapshot_capacity(length, maximum);
    let allocation = capacity.filter(|&capacity| matches_observed_length(capacity, bounded_view));
    let expected =
        if u64::from(length) <= u64::try_from(maximum).unwrap() && bounded_view == Some(length) {
            Some(length)
        } else {
            None
        };
    assert_eq!(allocation, expected);
    kani::cover!(allocation == Some(0), "empty validated snapshot");
    kani::cover!(
        allocation == Some(u32::MAX),
        "full uint32 extent when the cap permits it"
    );
    kani::cover!(
        capacity.is_none() && bounded_view == Some(length),
        "cap rejects a matching view"
    );
    kani::cover!(
        capacity.is_some() && bounded_view.is_none(),
        "unavailable view prevents allocation"
    );
    kani::cover!(
        capacity.is_some() && bounded_view.is_some_and(|len| len != length),
        "changed view size prevents allocation"
    );
}

#[kani::proof]
fn wasm_snapshot_copy_requires_both_post_copy_receipts() {
    let expected: u32 = kani::any();
    let copied: bool = kani::any();
    let source: Option<u32> = kani::any();
    let view: Option<u32> = kani::any();
    let actual = snapshot_copy_is_complete(expected, copied, source, view);
    let reference = match (copied, source, view) {
        (true, Some(source), Some(view)) => source == view && source == expected,
        _ => false,
    };
    assert_eq!(actual, reference);
    kani::cover!(actual, "copy may cross into Rust");
    kani::cover!(
        !copied && source == Some(expected) && view == Some(expected),
        "copy operation failed"
    );
    kani::cover!(
        copied && source.is_some_and(|len| len > expected) && view == Some(expected),
        "source grew while bounded view retained its length"
    );
    kani::cover!(
        copied && source == Some(expected) && view.is_none(),
        "bounded view receipt failed"
    );
    kani::cover!(
        copied && view == Some(expected) && source.is_some_and(|len| len < expected),
        "source shrank"
    );
}

#[kani::proof]
fn wasm_aggregate_is_independent_of_the_input_kind() {
    let text_utf8_bytes: usize = kani::any();
    let snapshot_bytes: usize = kani::any();
    let maximum: usize = kani::any();
    let next = checked_input_total(0, text_utf8_bytes, maximum)
        .and_then(|used| checked_input_total(used, snapshot_bytes, maximum));
    let wide = |n| u128::from(u64::try_from(n).unwrap());
    let total = wide(text_utf8_bytes) + wide(snapshot_bytes);
    let expected = (total <= wide(maximum)).then(|| usize::try_from(total).unwrap());
    assert_eq!(next, expected);
    kani::cover!(
        text_utf8_bytes != 0 && snapshot_bytes != 0 && total == wide(maximum),
        "string and array jointly reach the cap exactly"
    );
    kani::cover!(
        total > wide(usize::MAX),
        "overflow cannot wrap into the allocation budget"
    );
}
