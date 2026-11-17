// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Isolated allocator accounting for a rejected, maximum-size binary input.
#![cfg(feature = "generated")]
#![allow(
    missing_docs,
    clippy::unwrap_used,
    reason = "fixed allocation regression"
)]

use reallyme_codec_proto::generated::proto::reallyme::codec::v1::{
    CodecDeterministicCborArray, CodecErrorReason,
};
use reallyme_codec_proto::{decode_protobuf, MAX_CODEC_PROTO_MESSAGE_BYTES};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;

#[global_allocator]
static ALLOCATOR: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

#[test]
#[allow(
    clippy::print_stderr,
    reason = "input-independent allocation measurements"
)]
fn rejected_binary_tree_has_a_measured_allocation_regression_bound() {
    // Array.values = Value.array_value = empty Array. Each four-byte element
    // materializes a repeated Value and a separately boxed oneof payload. Keep
    // this independent wire fixture so generator changes cannot silently grow
    // the request before the measured decode begins.
    const ELEMENT: [u8; 4] = [0x0a, 0x02, 0x32, 0];
    // Leave headroom above the measured 4.3x native volume without pinning
    // exact generated layouts. This fixture ceiling is separate from RSS.
    const MAX_ALLOCATION_VOLUME_PER_WIRE_BYTE: usize = 6;
    assert_eq!(MAX_CODEC_PROTO_MESSAGE_BYTES % ELEMENT.len(), 0);
    let count = MAX_CODEC_PROTO_MESSAGE_BYTES
        .checked_div(ELEMENT.len())
        .unwrap();
    let mut encoded = Vec::with_capacity(MAX_CODEC_PROTO_MESSAGE_BYTES);
    for _ in 0..count {
        encoded.extend_from_slice(&ELEMENT);
    }
    assert_eq!(encoded.len(), MAX_CODEC_PROTO_MESSAGE_BYTES);

    let measured = Region::new(ALLOCATOR);
    let error = decode_protobuf::<CodecDeterministicCborArray>(&encoded)
        .err()
        .unwrap();
    let allocations = measured.change();
    assert_eq!(
        error.reason(),
        CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED
    );
    // stats_alloc includes Vec growth in bytes_allocated. This is cumulative
    // requested allocation volume, not process RSS or a universal heap cap.
    // The caller-owned input was allocated before the measured region.
    let input_and_allocations = encoded
        .len()
        .checked_add(allocations.bytes_allocated)
        .unwrap();
    let regression_bound = encoded
        .len()
        .checked_mul(MAX_ALLOCATION_VOLUME_PER_WIRE_BYTE)
        .unwrap();
    assert!(allocations.bytes_allocated > encoded.len());
    assert!(input_and_allocations <= regression_bound);
    assert_eq!(allocations.bytes_allocated, allocations.bytes_deallocated);
    eprintln!(
        "rejected binary tree: wire={} bytes, decode allocation volume={} bytes, input plus allocation volume={} bytes",
        encoded.len(), allocations.bytes_allocated, input_and_allocations
    );
}
