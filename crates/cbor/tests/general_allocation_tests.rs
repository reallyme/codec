// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! A single test isolates allocator accounting from other test threads.
#![allow(missing_docs, clippy::unwrap_used)]

use codec_cbor::general::{
    encode, frame_exact, validate_exact, LengthPolicy, Limits, MapEntry, Profile, ValueRef,
};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;

#[global_allocator]
static ALLOCATOR: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[test]
fn borrowed_validation_allocates_nothing_and_encoding_never_reallocates_payload() {
    // Nested, unsorted, compound keys exercise semantic comparison, not merely
    // the scalar fast path. No semantic tree or concatenated keys are needed.
    let input = [0xa2, 0x81, 0x02, 0x41, 0xaa, 0x81, 0x01, 0x41, 0xbb];
    let region = Region::new(ALLOCATOR);
    let frame = frame_exact(&input, Limits::default(), LengthPolicy::AllowIndefinite).unwrap();
    validate_exact(&input, Profile::received(), Limits::default()).unwrap();
    let allocations = region.change();
    assert_eq!(allocations.allocations, 0);
    assert_eq!(allocations.reallocations, 0);

    let payload = [0xab; 512];
    let entries = [
        MapEntry::new(ValueRef::Unsigned(2), ValueRef::Bytes(&payload)),
        MapEntry::new(ValueRef::Unsigned(1), ValueRef::Received(&frame)),
    ];
    let region = Region::new(ALLOCATOR);
    let encoded = encode(
        &ValueRef::Map(&entries),
        Profile::received(),
        Limits::default(),
    )
    .unwrap();
    let allocations = region.change();
    assert_eq!(allocations.allocations, 1);
    assert_eq!(allocations.reallocations, 0);
    assert_eq!(allocations.bytes_allocated, encoded.as_bytes().len());

    let region = Region::new(ALLOCATOR);
    assert!(encode(
        &ValueRef::Map(&entries),
        Profile::received(),
        Limits::default().with_output_bytes(1).unwrap()
    )
    .is_err());
    assert_eq!(region.change().allocations, 0);
}
