// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Generated protobuf and ProtoJSON retain independent general-CBOR budgets.
#![cfg(feature = "operation-contract")]
#![allow(
    missing_docs,
    clippy::unwrap_used,
    clippy::panic,
    reason = "bounded operation fixtures"
)]

use codec_proto::generated::proto::reallyme::codec::v1 as proto;
use codec_proto::{decode_protobuf, encode_protobuf};
use proto::codec_api_operation_request::Operation;
use proto::codec_api_operation_result::Result as Output;
use proto::CodecGeneralCborErrorReason as Reason;
use reallyme_codec::cbor::general::{DEFAULT_BYTES, MAX_BYTES};
use reallyme_codec::operation_contract::{
    process_operation_response, process_operation_response_json,
};
use zeroize::Zeroizing;

fn options() -> proto::CodecGeneralCborOptions {
    proto::CodecGeneralCborOptions {
        serialization:
            proto::CodecGeneralCborSerialization::CODEC_GENERAL_CBOR_SERIALIZATION_RECEIVED.into(),
        lengths:
            proto::CodecGeneralCborLengthPolicy::CODEC_GENERAL_CBOR_LENGTH_POLICY_DEFINITE_ONLY
                .into(),
        floats: proto::CodecGeneralCborFloatPolicy::CODEC_GENERAL_CBOR_FLOAT_POLICY_ALL.into(),
        ordering: proto::CodecGeneralCborMapOrdering::CODEC_GENERAL_CBOR_MAP_ORDERING_RECEIVED
            .into(),
        input_bytes: Some(u64::try_from(MAX_BYTES).unwrap()),
        output_bytes: Some(u64::try_from(MAX_BYTES).unwrap()),
        byte_string_bytes: Some(u64::try_from(MAX_BYTES).unwrap()),
        ..Default::default()
    }
}

fn wire(total: usize) -> Zeroizing<Vec<u8>> {
    let mut value = Zeroizing::new(vec![0x5a]);
    value.extend_from_slice(
        &u32::try_from(total.checked_sub(5).unwrap())
            .unwrap()
            .to_be_bytes(),
    );
    value.resize(total, 0xa5);
    value
}

fn source(
    encoded: &[u8],
    options: Option<proto::CodecGeneralCborOptions>,
) -> Box<proto::CodecGeneralCborSourceRequest> {
    Box::new(proto::CodecGeneralCborSourceRequest {
        encoded: encoded.to_vec().into(),
        options: options.into(),
    })
}

fn execute(operation: Operation) -> proto::CodecApiOperationResponse {
    let request = proto::CodecApiOperationRequest::default().with_operation(operation);
    let binary = encode_protobuf(&request).unwrap();
    let mut json = Zeroizing::new(Vec::new());
    serde_json::to_writer(&mut *json, &request).unwrap();
    let binary_response = process_operation_response(&binary);
    let json_response = process_operation_response_json(&json);
    assert_eq!(binary_response, json_response);
    decode_protobuf(&binary_response).unwrap()
}

fn result(response: &proto::CodecApiOperationResponse) -> &Output {
    let Some(proto::codec_api_operation_response::Outcome::Result(value)) =
        response.outcome.as_ref()
    else {
        panic!("expected operation result");
    };
    value.result.as_ref().unwrap()
}

fn reason(response: &proto::CodecApiOperationResponse, expected: Reason) {
    let Some(proto::codec_api_operation_response::Outcome::Error(value)) =
        response.outcome.as_ref()
    else {
        panic!("expected typed operation failure");
    };
    assert_eq!(
        value.origin.as_known(),
        Some(proto::CodecErrorOrigin::CODEC_ERROR_ORIGIN_CALLER)
    );
    let Some(proto::codec_api_error::Error::GeneralCbor(value)) = value.error.as_ref() else {
        panic!("expected general-CBOR error family");
    };
    assert_eq!(value.detail.as_known(), Some(expected));
}

#[test]
fn source_operations_preserve_large_selected_limits_and_exact_spans() {
    let encoded = wire(MAX_BYTES);
    for operation in [
        Operation::GeneralCborFrameExact(source(&encoded, Some(options()))),
        Operation::GeneralCborFramePrefix(source(&encoded, Some(options()))),
        Operation::GeneralCborValidateExact(source(&encoded, Some(options()))),
        Operation::GeneralCborValidatePrefix(source(&encoded, Some(options()))),
    ] {
        let response = execute(operation);
        let frame = match result(&response) {
            Output::GeneralCborFrameExact(frame)
            | Output::GeneralCborFramePrefix(frame)
            | Output::GeneralCborValidateExact(frame)
            | Output::GeneralCborValidatePrefix(frame) => frame,
            _ => panic!("expected framing result"),
        };
        assert_eq!(frame.received.as_option().unwrap().offset, 0);
        assert_eq!(
            frame.received.as_option().unwrap().length,
            u64::try_from(encoded.len()).unwrap()
        );
        assert_eq!(
            frame.options.as_option().unwrap().input_bytes,
            Some(u64::try_from(MAX_BYTES).unwrap())
        );
    }
    let response = execute(Operation::GeneralCborReadHeader(source(
        &encoded,
        Some(options()),
    )));
    assert!(matches!(
        result(&response),
        Output::GeneralCborReadHeader(_)
    ));
    let response = execute(Operation::GeneralCborValidateExact(source(&encoded, None)));
    reason(
        &response,
        Reason::CODEC_GENERAL_CBOR_ERROR_REASON_INPUT_LIMIT,
    );
    let mut tighter = options();
    tighter.input_bytes = Some(u64::try_from(MAX_BYTES.checked_sub(1).unwrap()).unwrap());
    reason(
        &execute(Operation::GeneralCborFrameExact(source(
            &encoded,
            Some(tighter),
        ))),
        Reason::CODEC_GENERAL_CBOR_ERROR_REASON_INPUT_LIMIT,
    );
    let mut tighter = options();
    tighter.byte_string_bytes = Some(u64::try_from(encoded.len().checked_sub(6).unwrap()).unwrap());
    reason(
        &execute(Operation::GeneralCborValidateExact(source(
            &encoded,
            Some(tighter),
        ))),
        Reason::CODEC_GENERAL_CBOR_ERROR_REASON_BYTE_STRING_LIMIT,
    );
}

#[test]
fn encoding_defaults_output_budget_and_received_input_budget_are_independent() {
    let encoded = wire(MAX_BYTES);
    let value = || {
        proto::CodecGeneralCborValue::default().with_value(
            proto::codec_general_cbor_value::Value::BytesValue(Box::new(
                proto::CodecBytesResult::default().with_value(encoded[5..].to_vec()),
            )),
        )
    };
    let request = |value, options: Option<proto::CodecGeneralCborOptions>| {
        Operation::GeneralCborEncode(Box::new(proto::CodecGeneralCborEncodeRequest {
            value: Some(value).into(),
            options: options.into(),
        }))
    };
    reason(
        &execute(request(value(), None)),
        Reason::CODEC_GENERAL_CBOR_ERROR_REASON_BYTE_STRING_LIMIT,
    );
    let mut selected = options();
    selected.input_bytes = Some(0);
    let response = execute(request(value(), Some(selected)));
    let Output::GeneralCborEncode(bytes) = result(&response) else {
        panic!("expected encoded bytes");
    };
    assert_eq!(bytes.value.as_slice(), encoded.as_slice());
    let mut selected = options();
    selected.output_bytes = Some(u64::try_from(MAX_BYTES.checked_sub(1).unwrap()).unwrap());
    reason(
        &execute(request(value(), Some(selected))),
        Reason::CODEC_GENERAL_CBOR_ERROR_REASON_OUTPUT_LIMIT,
    );
    let received = proto::CodecGeneralCborValue::default().with_value(
        proto::codec_general_cbor_value::Value::ReceivedValue(Box::new(
            proto::CodecBytesResult::default().with_value(encoded.to_vec()),
        )),
    );
    let mut selected = options();
    selected.input_bytes = Some(u64::try_from(DEFAULT_BYTES).unwrap());
    reason(
        &execute(request(received, Some(selected))),
        Reason::CODEC_GENERAL_CBOR_ERROR_REASON_INPUT_LIMIT,
    );
}

#[test]
fn cursor_children_and_embedded_operations_retain_the_selected_options() {
    let mut encoded = Zeroizing::new(vec![0x81]);
    encoded.extend_from_slice(&wire(DEFAULT_BYTES.checked_add(5).unwrap()));
    let response = execute(Operation::GeneralCborCursor(source(
        &encoded[1..],
        Some(options()),
    )));
    let Output::GeneralCborCursor(frames) = result(&response) else {
        panic!("expected cursor");
    };
    assert_eq!(frames.values.len(), 1);
    assert_eq!(
        frames.options.as_option().unwrap().input_bytes,
        options().input_bytes
    );
    assert!(frames.values[0].options.as_option().is_none());
    let response = execute(Operation::GeneralCborInspect(Box::new(
        proto::CodecGeneralCborInspectRequest {
            encoded: encoded.to_vec().into(),
            inspection: proto::CodecGeneralCborInspection::CODEC_GENERAL_CBOR_INSPECTION_CHILDREN
                .into(),
            validate: true,
            options: Some(options()).into(),
        },
    )));
    let Output::GeneralCborInspect(value) = result(&response) else {
        panic!("expected inspection");
    };
    let Some(proto::codec_general_cbor_inspection_result::Value::Children(frames)) =
        value.value.as_ref()
    else {
        panic!("expected child receipts");
    };
    assert_eq!(frames.values.len(), 1);
    assert_eq!(frames.values[0].received.as_option().unwrap().offset, 1);
    assert_eq!(
        frames.options.as_option().unwrap().byte_string_bytes,
        options().byte_string_bytes
    );
    let mut outer = Zeroizing::new(vec![0xd8, 0x18, 0x5a]);
    outer.extend_from_slice(&u32::try_from(encoded.len()).unwrap().to_be_bytes());
    outer.extend_from_slice(&encoded);
    for (embedded, expected) in [(None, false), (Some(options()), true)] {
        let response = execute(Operation::GeneralCborValidateEmbedded(Box::new(
            proto::CodecGeneralCborEmbeddedRequest {
                encoded: outer.to_vec().into(),
                outer_options: Some(options()).into(),
                embedded_options: embedded.into(),
            },
        )));
        if expected {
            let Output::GeneralCborValidateEmbedded(frame) = result(&response) else {
                panic!("expected embedded receipt");
            };
            assert_eq!(frame.received.as_option().unwrap().offset, 7);
            assert_eq!(
                frame.received.as_option().unwrap().length,
                u64::try_from(encoded.len()).unwrap()
            );
        } else {
            reason(
                &response,
                Reason::CODEC_GENERAL_CBOR_ERROR_REASON_INPUT_LIMIT,
            );
        }
    }
}

#[test]
fn generated_limits_above_the_ceiling_or_platform_size_are_typed_failures() {
    for limit in [
        u64::try_from(MAX_BYTES).unwrap().checked_add(1).unwrap(),
        u64::MAX,
    ] {
        for field in [0, 1, 2] {
            let mut selected = options();
            match field {
                0 => selected.input_bytes = Some(limit),
                1 => selected.output_bytes = Some(limit),
                _ => selected.byte_string_bytes = Some(limit),
            }
            reason(
                &execute(Operation::GeneralCborFrameExact(source(
                    &[0],
                    Some(selected),
                ))),
                Reason::CODEC_GENERAL_CBOR_ERROR_REASON_INVALID_LIMIT,
            );
        }
    }
}
