// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Mutate caller-supplied DER schemas, documents, and limits together.

#![no_main]

use codec_proto::generated::proto::reallyme::codec::v1 as proto;
use codec_proto::{decode_protobuf, encode_protobuf, MAX_CODEC_PROTO_MESSAGE_BYTES};
use libfuzzer_sys::fuzz_target;
use proto::codec_api_error::Error;
use proto::codec_api_operation_request::Operation;
use proto::codec_api_operation_response::Outcome;
use proto::codec_api_operation_result::Result as Output;
use reallyme_codec::der::{MAX_DER_ELEMENTS, MAX_DER_NESTING_DEPTH};
use reallyme_codec::operation_contract::process_operation_response;
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::{alloc::System, ops::Range};

#[global_allocator]
static ALLOCATOR: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

// Include boundary admission and response serialization, not just DER output.
// This transport-sized margin still catches the nested SET OF reservation
// regression without depending on allocator RSS or sanitizer quarantine size.
const MAX_OPERATION_ALLOCATED_BYTES: usize = 64 * 1024 * 1024;
const MIN_DER_HEADER_BYTES: usize = 2;

fuzz_target!(|data: &[u8]| {
    let Ok(request) = decode_protobuf::<proto::CodecDerDecodeRequest>(data) else {
        return;
    };
    let envelope = proto::CodecApiOperationRequest {
        operation: Some(Operation::DerDecode(Box::new(request))),
    };
    let Ok(wire) = encode_protobuf(&envelope) else {
        return;
    };
    let measurement = Region::new(ALLOCATOR);
    let encoded_response = process_operation_response(&wire);
    assert!(measurement.change().bytes_allocated < MAX_OPERATION_ALLOCATED_BYTES);
    assert!(encoded_response.len() <= MAX_CODEC_PROTO_MESSAGE_BYTES);
    let response = decode_protobuf::<proto::CodecApiOperationResponse>(&encoded_response)
        .expect("DER dispatcher returned an invalid protobuf response");
    match response.outcome.as_ref() {
        Some(Outcome::Result(result)) => {
            let Some(Output::DerDecode(value)) = result.result.as_ref() else {
                panic!("DER dispatcher returned an unrelated result");
            };
            let Some(Operation::DerDecode(request)) = envelope.operation.as_ref() else {
                panic!("DER operation envelope changed");
            };
            let schema = request
                .schema
                .as_option()
                .expect("accepted DER schema missing");
            assert_eq!(
                received_range(value, request.encoded.len()),
                0..request.encoded.len()
            );
            check_value(value, schema, &request.encoded, 0, &mut 0);
        }
        Some(Outcome::Error(error)) => {
            assert!(matches!(
                error.error.as_ref(),
                Some(Error::Der(_) | Error::Boundary(_))
            ));
        }
        None => panic!("DER dispatcher returned a response without an outcome"),
    }
});

fn span_range(span: &proto::CodecByteSpan, source_len: usize) -> Range<usize> {
    let start = usize::try_from(span.offset).expect("accepted DER offset overflowed");
    let length = usize::try_from(span.length).expect("accepted DER length overflowed");
    let end = start
        .checked_add(length)
        .expect("accepted DER span overflowed");
    assert!(end <= source_len);
    start..end
}

fn received_range(value: &proto::CodecDerValue, source_len: usize) -> Range<usize> {
    span_range(
        value.received.as_option().expect("received span missing"),
        source_len,
    )
}

fn check_value(
    value: &proto::CodecDerValue,
    schema: &proto::CodecDerSchema,
    source: &[u8],
    depth: usize,
    nodes: &mut usize,
) {
    *nodes = nodes.checked_add(1).expect("DER node count overflowed");
    assert!(*nodes <= MAX_DER_ELEMENTS);
    assert_eq!(value.kind, schema.kind);
    let received = received_range(value, source.len());
    let contents = span_range(
        value.contents.as_option().expect("contents span missing"),
        source.len(),
    );
    let header_len = contents
        .start
        .checked_sub(received.start)
        .expect("DER header reversed");
    assert!(header_len >= MIN_DER_HEADER_BYTES);
    assert_eq!(contents.end, received.end);
    let kind = schema.kind.as_known().expect("unknown accepted DER kind");
    let is_set = kind == proto::CodecDerType::CODEC_DER_TYPE_SET_OF;
    if is_set || kind == proto::CodecDerType::CODEC_DER_TYPE_SEQUENCE {
        let depth = depth.checked_add(1).expect("DER depth overflowed");
        assert!(depth <= MAX_DER_NESTING_DEPTH);
        if is_set {
            assert_eq!(schema.children.len(), 1);
        } else {
            assert_eq!(value.children.len(), schema.children.len());
        }
        let mut offset = contents.start;
        for (index, child) in value.children.iter().enumerate() {
            let child_schema = schema
                .children
                .get(if is_set { 0 } else { index })
                .expect("accepted DER child schema missing");
            let child_range = received_range(child, source.len());
            assert_eq!(child_range.start, offset);
            assert!(child_range.end <= contents.end);
            check_value(child, child_schema, source, depth, nodes);
            offset = child_range.end;
        }
        assert_eq!(offset, contents.end);
        return;
    }
    assert!(value.children.is_empty());
    let contents = &source[contents];
    match kind {
        proto::CodecDerType::CODEC_DER_TYPE_UNSIGNED_INTEGER => {
            let magnitude = if contents.len() > 1 && contents.first() == Some(&0) {
                &contents[1..]
            } else {
                contents
            };
            assert!(value.bytes_value.as_ref() == magnitude);
        }
        proto::CodecDerType::CODEC_DER_TYPE_BOOLEAN => {
            assert!(contents == [0] || contents == [0xff]);
            assert_eq!(value.bool_value, Some(contents == [0xff]));
        }
        proto::CodecDerType::CODEC_DER_TYPE_BIT_STRING => {
            let (unused_bits, bytes) = contents.split_first().expect("empty DER bit string");
            assert_eq!(value.unused_bits, Some(u32::from(*unused_bits)));
            assert!(value.bytes_value.as_ref() == bytes);
        }
        proto::CodecDerType::CODEC_DER_TYPE_NULL => assert!(contents.is_empty()),
        _ => assert!(value.bytes_value.as_ref() == contents),
    }
}
