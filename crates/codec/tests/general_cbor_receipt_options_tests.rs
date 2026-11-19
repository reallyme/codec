// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(feature = "operation-contract")]
#![allow(
    missing_docs,
    clippy::unwrap_used,
    clippy::panic,
    reason = "bounded receipt fixtures"
)]

use codec_proto::generated::proto::reallyme::codec::v1 as proto;
use codec_proto::{decode_protobuf, encode_protobuf};
use proto::codec_api_operation_request::Operation;
use proto::codec_api_operation_result::Result as Output;
use reallyme_codec::operation_contract::process_operation_response;

fn execute(operation: Operation) -> (proto::CodecApiOperationResponse, usize) {
    let request = proto::CodecApiOperationRequest::default().with_operation(operation);
    let encoded = process_operation_response(&encode_protobuf(&request).unwrap());
    (decode_protobuf(&encoded).unwrap(), encoded.len())
}

fn result(response: &mut proto::CodecApiOperationResponse) -> &mut Output {
    let Some(proto::codec_api_operation_response::Outcome::Result(result)) =
        response.outcome.as_mut()
    else {
        panic!("expected receipt result");
    };
    result.result.as_mut().unwrap()
}

fn source(encoded: &[u8]) -> Box<proto::CodecGeneralCborSourceRequest> {
    Box::new(proto::CodecGeneralCborSourceRequest {
        encoded: encoded.to_vec().into(),
        options: Default::default(),
    })
}

#[test]
#[allow(
    clippy::print_stderr,
    reason = "input-independent response-size measurements"
)]
fn cursor_shares_normalized_options_and_reduces_the_eight_kib_response() {
    let input = vec![0; 8_192];
    let (mut response, compact_bytes) = execute(Operation::GeneralCborCursor(source(&input)));
    let Output::GeneralCborCursor(frames) = result(&mut response) else {
        panic!("expected cursor")
    };
    assert_eq!(frames.values.len(), input.len());
    assert_eq!(frames.consumed_bytes, u64::try_from(input.len()).unwrap());
    let options = frames.options.as_option().unwrap();
    assert_eq!(options.entries, Some(4_096));
    assert_eq!(options.nodes, Some(65_536));
    let shared_options = options.clone();
    for (offset, frame) in frames.values.iter().enumerate() {
        assert!(frame.options.as_option().is_none());
        assert!(!frame.validated);
        assert_eq!(
            frame.received.as_option().unwrap().offset,
            u64::try_from(offset).unwrap()
        );
        assert_eq!(frame.received.as_option().unwrap().length, 1);
        assert_eq!(frame.header.as_option().unwrap().argument, Some(0));
    }
    // Reconstruct the redundant form through the generated serializer. This
    // keeps the comparison independent of hand-written protobuf size formulas.
    for frame in frames.values.iter_mut() {
        frame.options = Some(shared_options.clone()).into();
    }
    let duplicated_bytes = encode_protobuf(&response).unwrap().len();
    assert!(compact_bytes.checked_mul(2).unwrap() < duplicated_bytes);
    assert!(compact_bytes < 320 * 1_024);
    eprintln!(
        "8 KiB cursor response: {compact_bytes} bytes shared, {duplicated_bytes} bytes repeated"
    );
}

#[test]
fn empty_cursor_and_empty_child_collection_keep_the_shared_policy() {
    let (mut response, _) = execute(Operation::GeneralCborCursor(source(&[])));
    let Output::GeneralCborCursor(frames) = result(&mut response) else {
        panic!("expected cursor")
    };
    assert!(frames.values.is_empty());
    assert!(frames.options.as_option().is_some());
    assert_eq!(frames.consumed_bytes, 0);
    let (mut inspected, _) = inspect(&[0x80]);
    let Output::GeneralCborInspect(inspected) = result(&mut inspected) else {
        panic!("expected inspection")
    };
    let Some(proto::codec_general_cbor_inspection_result::Value::Children(frames)) =
        inspected.value.as_ref()
    else {
        panic!("expected children");
    };
    assert!(frames.values.is_empty());
    assert!(frames.options.as_option().is_some());
}

fn inspect(encoded: &[u8]) -> (proto::CodecApiOperationResponse, usize) {
    execute(Operation::GeneralCborInspect(Box::new(
        proto::CodecGeneralCborInspectRequest {
            encoded: encoded.to_vec().into(),
            inspection: proto::CodecGeneralCborInspection::CODEC_GENERAL_CBOR_INSPECTION_CHILDREN
                .into(),
            validate: true,
            options: Default::default(),
        },
    )))
}

#[test]
fn inspected_children_inherit_policy_while_standalone_frames_keep_theirs() {
    let (mut standalone, _) = execute(Operation::GeneralCborValidateExact(source(&[0x82, 0, 1])));
    let Output::GeneralCborValidateExact(frame) = result(&mut standalone) else {
        panic!("expected frame")
    };
    let expected_options = encode_protobuf(frame.options.as_option().unwrap()).unwrap();
    let (mut inspected, _) = inspect(&[0x82, 0, 1]);
    let Output::GeneralCborInspect(inspected) = result(&mut inspected) else {
        panic!("expected inspection")
    };
    let Some(proto::codec_general_cbor_inspection_result::Value::Children(frames)) =
        inspected.value.as_ref()
    else {
        panic!("expected children");
    };
    assert_eq!(
        encode_protobuf(frames.options.as_option().unwrap()).unwrap(),
        expected_options
    );
    assert_eq!(frames.values.len(), 2);
    assert_eq!(frames.consumed_bytes, 2);
    for frame in &frames.values {
        assert!(frame.options.as_option().is_none());
        assert!(frame.validated);
    }
}

#[test]
fn cursor_preserves_explicit_caller_limits_in_the_shared_options() {
    let (mut standalone, _) = execute(Operation::GeneralCborFrameExact(source(&[0])));
    let Output::GeneralCborFrameExact(frame) = result(&mut standalone) else {
        panic!("expected frame")
    };
    let mut options = frame.options.as_option().unwrap().clone();
    options.input_bytes = Some(2);
    options.nodes = Some(2);
    options.entries = Some(2);
    options.work = Some(128);
    let expected = encode_protobuf(&options).unwrap();
    let mut request = source(&[0, 1]);
    request.options = Some(options).into();
    let (mut response, _) = execute(Operation::GeneralCborCursor(request));
    let Output::GeneralCborCursor(frames) = result(&mut response) else {
        panic!("expected cursor")
    };
    assert_eq!(frames.values.len(), 2);
    assert_eq!(
        encode_protobuf(frames.options.as_option().unwrap()).unwrap(),
        expected
    );
    assert!(frames
        .values
        .iter()
        .all(|frame| frame.options.as_option().is_none()));
}
