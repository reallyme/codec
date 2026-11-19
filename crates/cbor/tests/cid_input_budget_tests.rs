// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! The CID text cap must run before any multibase allocation. This isolated
//! test binary keeps allocator accounting independent from other tests.

#![allow(missing_docs)]

use codec_cbor::{is_valid_cid_string, MAX_CID_STRING_LEN};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;

#[global_allocator]
static ALLOCATOR: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[test]
fn oversized_cid_text_is_rejected_before_multibase_allocation() {
    // This is deliberately a valid base58 alphabet string. Its CID payload is
    // invalid, so checking only the boolean result would let a removed length
    // guard pass; the allocation assertion proves the guard runs first.
    let mut candidate = String::with_capacity(MAX_CID_STRING_LEN + 2);
    candidate.push('z');
    candidate.extend(core::iter::repeat_n('1', MAX_CID_STRING_LEN + 1));

    // The allocator is process-wide: libtest's reporting thread can allocate
    // while this test runs. Take the minimum across bounded samples to remove
    // that additive noise. A missing guard still decodes into a fresh owned
    // multibase buffer on every call, so no sample could be allocation-free.
    const ALLOCATION_SAMPLES: usize = 32;
    let mut minimum_allocated = usize::MAX;
    for _ in 0..ALLOCATION_SAMPLES {
        let measured = Region::new(ALLOCATOR);
        let accepted = is_valid_cid_string(&candidate);
        let allocations = measured.change();
        assert!(!accepted);
        minimum_allocated = minimum_allocated.min(allocations.bytes_allocated);
    }
    assert_eq!(minimum_allocated, 0);
}
