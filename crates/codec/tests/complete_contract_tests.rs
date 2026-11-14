// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Every generated operation must execute identically through both transports.
#![cfg(feature = "operation-contract")]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "bounded public conformance fixtures"
)]

use codec_proto::generated::{
    proto::reallyme::codec::v1 as proto, wire_policy::OPERATION_FIELD_NUMBERS,
};
use codec_proto::{decode_json, decode_protobuf, encode_protobuf};
use reallyme_codec::operation_contract::{
    process_operation_response, process_operation_response_json,
};
use serde_json::Value;

fn cases() -> Vec<Value> {
    let manifest: Value = serde_json::from_str(include_str!(
        "../../../vectors/codec-operation-vectors.json"
    ))
    .unwrap();
    assert_eq!(manifest["schemaVersion"], 1);
    manifest["cases"].as_array().unwrap().clone()
}

fn hex(value: &str) -> Vec<u8> {
    reallyme_codec::hex::lower_hex_to_bytes(value).unwrap()
}

fn varint(bytes: &[u8], offset: &mut usize) -> u64 {
    let mut value = 0u64;
    for shift in (0..64).step_by(7) {
        let byte = *bytes.get(*offset).unwrap();
        *offset = offset.checked_add(1).unwrap();
        value |= u64::from(byte & 0x7f).checked_shl(shift).unwrap();
        if byte & 0x80 == 0 {
            return value;
        }
    }
    panic!("invalid fixture varint");
}

fn put_varint(mut value: u64, bytes: &mut Vec<u8>) {
    loop {
        let low = u8::try_from(value & 0x7f).unwrap();
        value >>= 7;
        bytes.push(low | if value == 0 { 0 } else { 0x80 });
        if value == 0 {
            return;
        }
    }
}

fn assert_boundary(bytes: &[u8], reason: proto::CodecErrorReason) {
    let response: proto::CodecApiOperationResponse = decode_protobuf(bytes).unwrap();
    let Some(proto::codec_api_operation_response::Outcome::Error(error)) =
        response.outcome.as_ref()
    else {
        panic!("expected typed rejection");
    };
    assert_eq!(
        error.origin.as_known(),
        Some(proto::CodecErrorOrigin::CODEC_ERROR_ORIGIN_CALLER)
    );
    assert!(
        matches!(error.error.as_ref(), Some(proto::codec_api_error::Error::Boundary(boundary)) if boundary.reason.as_known() == Some(reason))
    );
}

#[test]
fn every_schema_operation_has_binary_json_and_semantic_conformance() {
    let mut tags = Vec::new();
    for case in cases() {
        let operation = case["operation"].as_str().unwrap();
        let json = case["requestJson"].as_str().unwrap().as_bytes();
        let request: proto::CodecApiOperationRequest = decode_json(json).unwrap();
        let binary = hex(case["requestHex"].as_str().unwrap());
        assert_eq!(
            encode_protobuf(&request).unwrap().as_slice(),
            binary,
            "{operation}"
        );
        let mut offset = 0;
        let tag = varint(&binary, &mut offset) >> 3;
        assert_eq!(case["fieldNumber"].as_u64().unwrap(), tag);
        tags.push(tag);
        let binary_response = process_operation_response(&binary);
        let json_response = process_operation_response_json(json);
        assert_eq!(binary_response, json_response, "{operation}");
        let response: proto::CodecApiOperationResponse = decode_protobuf(&binary_response).unwrap();
        let response = serde_json::to_value(&response).unwrap();
        let result = response
            .get("result")
            .and_then(|result| result.get(operation))
            .unwrap_or_else(|| panic!("missing successful result for {operation}"));
        for (pointer, expected) in case["checks"].as_object().unwrap() {
            assert_eq!(
                result.pointer(pointer),
                Some(expected),
                "{operation} {pointer}"
            );
        }
    }
    tags.sort_unstable();
    assert_eq!(
        tags, OPERATION_FIELD_NUMBERS,
        "new schema operations require conformance vectors"
    );
}

#[test]
fn every_operation_rejects_unknown_request_fields_with_stable_errors() {
    for case in cases() {
        let binary = hex(case["requestHex"].as_str().unwrap());
        let mut offset = 0;
        let tag = varint(&binary, &mut offset);
        let length = varint(&binary, &mut offset);
        assert_eq!(
            usize::try_from(length).unwrap(),
            binary.len().checked_sub(offset).unwrap()
        );
        let mut changed = Vec::new();
        put_varint(tag, &mut changed);
        put_varint(length.checked_add(3).unwrap(), &mut changed);
        changed.extend_from_slice(&binary[offset..]);
        changed.extend_from_slice(&[0x98, 0x06, 0x01]); // unknown request field 99
        assert_boundary(
            &process_operation_response(&changed),
            proto::CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MALFORMED_PROTOBUF,
        );

        let mut request: Value =
            serde_json::from_str(case["requestJson"].as_str().unwrap()).unwrap();
        request[case["operation"].as_str().unwrap()]["unexpected"] = true.into();
        assert_boundary(
            &process_operation_response_json(&serde_json::to_vec(&request).unwrap()),
            proto::CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MALFORMED_JSON,
        );
    }
}
