// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! This file has one test so global allocator accounting cannot include
//! allocations made by another test thread.

#![allow(missing_docs)]

use codec_cbor::{decode_dag_cbor, CborError, MAX_DAG_CBOR_CONTAINER_ENTRIES};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;

#[global_allocator]
static ALLOCATOR: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const NESTING_LEVELS: usize = 48;
const MAX_TEST_ALLOCATED_BYTES: usize = 4 * 1024 * 1024;

#[test]
fn nested_declared_arrays_reject_before_large_reservations() {
    // Each header is canonical and its count is plausible given the trailing
    // bytes. Without the aggregate node budget, every nested level can
    // reserve the same 16,384 element slots before input exhaustion.
    let mut bytes = Vec::new();
    for _ in 0..NESTING_LEVELS {
        bytes.extend_from_slice(&[0x99, 0x40, 0x00]);
    }
    bytes.extend(core::iter::repeat_n(0x00, MAX_DAG_CBOR_CONTAINER_ENTRIES));

    let measured = Region::new(ALLOCATOR);
    let result = decode_dag_cbor(&bytes);
    let allocations = measured.change();

    assert_eq!(result, Err(CborError::NodeLimitExceeded));
    assert!(
        allocations.bytes_allocated < MAX_TEST_ALLOCATED_BYTES,
        "nested length headers allocated {} bytes",
        allocations.bytes_allocated
    );
}
