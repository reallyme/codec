// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! One test keeps global allocation accounting independent of other tests.

#![cfg(feature = "operation-contract")]
#![allow(clippy::unwrap_used, clippy::panic, reason = "bounded test fixtures")]

#[path = "der/support/mod.rs"]
#[allow(dead_code, reason = "shared fixtures also provide integer helpers")]
mod support;

use codec_proto::generated::proto::reallyme::codec::v1 as proto;
use codec_proto::{decode_protobuf, encode_protobuf};
use proto::codec_api_operation_request::Operation;
use proto::codec_api_operation_response::Outcome;
use proto::codec_api_operation_result::Result as Output;
use reallyme_codec::der::{MAX_DER_ELEMENTS, MAX_DER_INPUT_LEN, MAX_DER_NESTING_DEPTH};
use reallyme_codec::operation_contract::process_operation_response;
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;
use support::tlv;

#[global_allocator]
static ALLOCATOR: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

const SMALL_ALLOCATION_CEILING: usize = 1024 * 1024;
const DOCUMENT_ALLOCATION_CEILING: usize = 32 * 1024 * 1024;

fn schema(
    kind: proto::CodecDerType,
    children: Vec<proto::CodecDerSchema>,
) -> proto::CodecDerSchema {
    proto::CodecDerSchema {
        kind: kind.into(),
        children: children.into(),
    }
}

fn chain(depth: usize, payload_len: usize) -> (Vec<u8>, proto::CodecDerSchema) {
    let mut encoded = tlv(0x04, &vec![0x5a; payload_len]);
    let mut shape = schema(proto::CodecDerType::CODEC_DER_TYPE_OCTET_STRING, vec![]);
    for _ in 0..depth {
        encoded = tlv(0x31, &encoded);
        shape = schema(proto::CodecDerType::CODEC_DER_TYPE_SET_OF, vec![shape]);
    }
    (encoded, shape)
}

fn execute(
    encoded: Vec<u8>,
    shape: proto::CodecDerSchema,
    limits: Option<proto::CodecDerLimits>,
    allocation_ceiling: usize,
) -> Outcome {
    let request = proto::CodecApiOperationRequest {
        operation: Some(Operation::DerDecode(Box::new(
            proto::CodecDerDecodeRequest {
                encoded: encoded.into(),
                schema: Some(shape).into(),
                limits: limits.into(),
            },
        ))),
    };
    let wire = encode_protobuf(&request).unwrap();
    let measurement = Region::new(ALLOCATOR);
    let response = process_operation_response(&wire);
    let allocated = measurement.change().bytes_allocated;
    assert!(
        allocated < allocation_ceiling,
        "DER operation allocated {allocated} bytes; ceiling is {allocation_ceiling}"
    );
    decode_protobuf::<proto::CodecApiOperationResponse>(&response)
        .unwrap()
        .outcome
        .take()
        .unwrap()
}

fn success(mut outcome: Outcome) -> proto::CodecDerValue {
    let Outcome::Result(ref mut result) = outcome else {
        panic!("expected successful DER decoding");
    };
    let Some(Output::DerDecode(value)) = result.result.as_mut() else {
        panic!("expected DER value");
    };
    core::mem::take(&mut **value)
}

fn rejected(outcome: Outcome, reason: proto::CodecDerErrorReason) {
    let Outcome::Error(ref error) = outcome else {
        panic!("expected typed DER rejection");
    };
    let Some(proto::codec_api_error::Error::Der(error)) = error.error.as_ref() else {
        panic!("expected DER error family");
    };
    assert_eq!(error.detail.as_known(), Some(reason));
}

#[test]
fn sets_reserve_actual_children_with_bounded_document_allocations() {
    // Run a small reproducer first: a regression fails without allocating the
    // nearly two gigabytes requested by the full default-limit attack shape.
    let (small, shape) = chain(12, 8192);
    let value = success(execute(small, shape, None, SMALL_ALLOCATION_CEILING));
    assert_eq!(value.children.len(), 1);
    drop(value);

    // The outer set plus 31 nested sets reaches the depth ceiling. Its 126
    // equal-shaped siblings contain 4,033 values and exactly one MiB of DER.
    let depth = MAX_DER_NESTING_DEPTH - 1;
    let siblings = 126;
    let (child, shape) = chain(depth, 8193);
    let mut contents = child.repeat(siblings - 1);
    let missing = MAX_DER_INPUT_LEN - tlv(0x31, &child.repeat(siblings)).len();
    let (last, _) = chain(depth, 8193 + missing);
    contents.extend_from_slice(&last);
    let document = tlv(0x31, &contents);
    assert_eq!(document.len(), MAX_DER_INPUT_LEN);
    assert!(siblings * (depth + 1) < MAX_DER_ELEMENTS);
    let outer = schema(proto::CodecDerType::CODEC_DER_TYPE_SET_OF, vec![shape]);
    let value = success(execute(document, outer, None, DOCUMENT_ALLOCATION_CEILING));
    assert_eq!(value.children.len(), siblings);
    for child in &value.children {
        let mut leaf = child;
        for _ in 0..depth {
            assert_eq!(leaf.children.len(), 1);
            leaf = &leaf.children[0];
        }
        assert!(leaf.bytes_value.iter().all(|byte| *byte == 0x5a));
    }
    drop(value);

    let null = || schema(proto::CodecDerType::CODEC_DER_TYPE_NULL, vec![]);
    let set = || schema(proto::CodecDerType::CODEC_DER_TYPE_SET_OF, vec![null()]);
    assert!(success(execute(
        vec![0x31, 0],
        set(),
        None,
        SMALL_ALLOCATION_CEILING
    ))
    .children
    .is_empty());
    let flat = tlv(0x31, &[0x05, 0].repeat(MAX_DER_ELEMENTS - 1));
    assert_eq!(
        success(execute(flat, set(), None, DOCUMENT_ALLOCATION_CEILING))
            .children
            .len(),
        MAX_DER_ELEMENTS - 1
    );
    rejected(
        execute(
            tlv(0x31, &[0x05, 0].repeat(MAX_DER_ELEMENTS)),
            set(),
            None,
            DOCUMENT_ALLOCATION_CEILING,
        ),
        proto::CodecDerErrorReason::TooManyElements,
    );
    rejected(
        execute(
            vec![0x31, 4, 0x05, 0, 0x05, 0],
            set(),
            Some(proto::CodecDerLimits {
                elements: Some(2),
                ..Default::default()
            }),
            SMALL_ALLOCATION_CEILING,
        ),
        proto::CodecDerErrorReason::TooManyElements,
    );
    for (encoded, reason) in [
        (
            vec![0x31, 1, 0x05],
            proto::CodecDerErrorReason::TruncatedInput,
        ),
        (
            vec![0x31, 3, 0x05, 0x81, 0],
            proto::CodecDerErrorReason::InvalidLength,
        ),
    ] {
        rejected(
            execute(encoded, set(), None, SMALL_ALLOCATION_CEILING),
            reason,
        );
    }
    let booleans = schema(
        proto::CodecDerType::CODEC_DER_TYPE_SET_OF,
        vec![schema(proto::CodecDerType::CODEC_DER_TYPE_BOOLEAN, vec![])],
    );
    rejected(
        execute(
            vec![0x31, 6, 0x01, 1, 0xff, 0x01, 1, 0],
            booleans,
            None,
            SMALL_ALLOCATION_CEILING,
        ),
        proto::CodecDerErrorReason::SetOrdering,
    );
}
