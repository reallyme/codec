// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! This test binary has one test so allocator accounting excludes other tests.

#![cfg(feature = "generated")]
#![allow(missing_docs)]

use reallyme_codec_proto::generated::proto::reallyme::codec::v1::{
    CodecDeterministicCborArray, CodecDeterministicCborValue, CodecErrorReason,
};
use reallyme_codec_proto::{decode_json, decode_protobuf, MAX_CODEC_PROTO_MESSAGE_BYTES};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;

#[global_allocator]
static ALLOCATOR: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const EMPTY_MAP_ENTRIES: usize = 32_768;
const BINARY_EMPTY_VALUES: usize = 1_000_000;
const NESTED_ARRAY_VALUES: usize = 32_767;
const MAX_JSON_REJECTION_ALLOCATED_BYTES: usize = 2 * 1024 * 1024;
const MAX_BINARY_REJECTION_ALLOCATED_BYTES: usize = 64 * 1024 * 1024;
const MAX_VALID_NESTED_ARRAY_ALLOCATED_BYTES: usize = 32 * 1024 * 1024;

#[expect(
    clippy::unwrap_used,
    reason = "test depths keep each fixture length below 128"
)]
fn nested_array_element(depth: usize) -> Vec<u8> {
    let mut value = vec![0x0a, 0x00];
    for _ in 0..depth {
        let mut array = vec![0x0a, u8::try_from(value.len()).unwrap()];
        array.extend_from_slice(&value);
        value = vec![0x32, u8::try_from(array.len()).unwrap()];
        value.extend_from_slice(&array);
    }
    let mut element = vec![0x0a, u8::try_from(value.len()).unwrap()];
    element.extend_from_slice(&value);
    element
}

#[test]
fn malformed_json_and_binary_element_excess_reject_with_bounded_allocations() {
    let mut json = Vec::with_capacity(EMPTY_MAP_ENTRIES * 3 + 32);
    json.extend_from_slice(br#"{"mapValue":{"entries":["#);
    for index in 0..EMPTY_MAP_ENTRIES {
        if index != 0 {
            json.push(b',');
        }
        json.extend_from_slice(b"{}");
    }
    json.extend_from_slice(b"]}}");

    let json_measurement = Region::new(ALLOCATOR);
    let json_error = decode_json::<CodecDeterministicCborValue>(&json)
        .err()
        .map(|error| error.reason());
    let json_allocations = json_measurement.change();
    assert_eq!(
        json_error,
        Some(CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MALFORMED_JSON)
    );
    assert!(
        json_allocations.bytes_allocated < MAX_JSON_REJECTION_ALLOCATED_BYTES,
        "empty map entries allocated {} bytes",
        json_allocations.bytes_allocated
    );

    let mut binary = Vec::with_capacity(BINARY_EMPTY_VALUES * 2);
    for _ in 0..BINARY_EMPTY_VALUES {
        binary.extend_from_slice(&[0x0a, 0x00]);
    }
    let binary_measurement = Region::new(ALLOCATOR);
    let binary_error = decode_protobuf::<CodecDeterministicCborArray>(&binary)
        .err()
        .map(|error| error.reason());
    let binary_allocations = binary_measurement.change();
    assert_eq!(
        binary_error,
        Some(CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED)
    );
    assert!(
        binary_allocations.bytes_allocated < MAX_BINARY_REJECTION_ALLOCATED_BYTES,
        "repeated binary values allocated {} bytes",
        binary_allocations.bytes_allocated
    );

    // Every repeated value is an array containing one null. Along with the
    // enclosing array this is 65,535 semantic nodes, below the public cap.
    // Exercise the valid high-amplification shape as well as rejection paths.
    let mut nested = Vec::with_capacity(NESTED_ARRAY_VALUES * 8);
    for _ in 0..NESTED_ARRAY_VALUES {
        nested.extend_from_slice(&[0x0a, 0x06, 0x32, 0x04, 0x0a, 0x02, 0x0a, 0x00]);
    }
    let nested_measurement = Region::new(ALLOCATOR);
    let nested_result = decode_protobuf::<CodecDeterministicCborArray>(&nested);
    let nested_allocations = nested_measurement.change();
    assert!(
        nested_result.is_ok(),
        "valid nested arrays were rejected: {:?}",
        nested_result.err().map(|error| error.reason())
    );
    assert!(
        nested_allocations.bytes_allocated < MAX_VALID_NESTED_ARRAY_ALLOCATED_BYTES,
        "valid nested arrays allocated {} bytes",
        nested_allocations.bytes_allocated
    );

    for depth in [2, 3, 4, 7, 15] {
        let element = nested_array_element(depth);
        let element_count = 65_535 / (depth + 1);
        let mut amplified = Vec::with_capacity(element.len() * element_count);
        for _ in 0..element_count {
            amplified.extend_from_slice(&element);
        }
        let amplified_measurement = Region::new(ALLOCATOR);
        let amplified_result = decode_protobuf::<CodecDeterministicCborArray>(&amplified);
        let amplified_allocations = amplified_measurement.change();
        assert!(
            amplified_allocations.bytes_allocated < MAX_BINARY_REJECTION_ALLOCATED_BYTES,
            "depth {depth} binary values allocated {} bytes; result: {:?}",
            amplified_allocations.bytes_allocated,
            amplified_result.err().map(|error| error.reason())
        );
    }

    for depth in [1, 2, 3] {
        let rejected_element = nested_array_element(depth);
        let element_count = (MAX_CODEC_PROTO_MESSAGE_BYTES - 1) / rejected_element.len();
        let mut rejected_nested = Vec::with_capacity(element_count * rejected_element.len());
        for _ in 0..element_count {
            rejected_nested.extend_from_slice(&rejected_element);
        }
        let rejected_nested_measurement = Region::new(ALLOCATOR);
        let rejected_nested_error =
            decode_protobuf::<CodecDeterministicCborArray>(&rejected_nested)
                .err()
                .map(|error| error.reason());
        let rejected_nested_allocations = rejected_nested_measurement.change();
        assert_eq!(
            rejected_nested_error,
            Some(CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED)
        );
        assert!(
            rejected_nested_allocations.bytes_allocated < MAX_BINARY_REJECTION_ALLOCATED_BYTES,
            "depth {depth} binary excess allocated {} bytes",
            rejected_nested_allocations.bytes_allocated
        );
    }
}
