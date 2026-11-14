// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Independent framing fixtures exercise native views and both wire transports.
#![cfg(feature = "operation-contract")]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "public synthetic fixtures"
)]

use codec_proto::generated::proto::reallyme::codec::v1 as proto;
use codec_proto::{codec_api_error, decode_json, decode_protobuf, encode_protobuf, CodecWireError};
use reallyme_codec::operation_contract::{
    process_operation_response, process_operation_response_json,
};
use reallyme_codec::tlv;
use serde_json::Value;

fn native(
    request: &proto::CodecApiOperationRequest,
) -> Result<proto::CodecApiOperationResult, CodecWireError> {
    use proto::codec_api_operation_request::Operation as Op;
    use proto::codec_api_operation_result::Result as Out;
    let operation = request.operation.as_ref().unwrap();
    let source = match operation {
        Op::TlvReadHeader(value)
        | Op::TlvFramePrefix(value)
        | Op::TlvFrameExact(value)
        | Op::TlvReadIdentifier(value)
        | Op::TlvReadLength(value) => value,
        _ => panic!("unexpected fixture operation"),
    };
    let fallback = tlv::Options::default();
    let options = source.options.as_option().unwrap_or(&fallback);
    let input = source.encoded.as_slice();
    let error = |error: tlv::TlvError| CodecWireError::from_tlv_reason(error.reason());
    let result = match operation {
        Op::TlvReadHeader(_) => {
            let header = tlv::read_header(input, options).map_err(error)?;
            assert_eq!(header.received_bytes().as_ptr(), input.as_ptr());
            assert_eq!(header.identifier_bytes().as_ptr(), input.as_ptr());
            assert_eq!(
                header.supplied_remainder(),
                &input[header.consumed_bytes()..]
            );
            assert_eq!(
                header.length_octets().len() + header.identifier_bytes().len(),
                header.consumed_bytes()
            );
            Out::TlvReadHeader(Box::new(header.into_contract()))
        }
        Op::TlvFramePrefix(_) => {
            let (frame, remaining) = tlv::frame_prefix(input, options).map_err(error)?;
            assert_eq!(frame.received_bytes().as_ptr(), input.as_ptr());
            assert_eq!(remaining, &input[frame.consumed_bytes()..]);
            assert_eq!(
                frame.value_bytes().as_ptr(),
                input[frame.header().consumed_bytes()..].as_ptr()
            );
            Out::TlvFramePrefix(Box::new(frame.into_contract()))
        }
        Op::TlvFrameExact(_) => {
            let frame = tlv::frame_exact(input, options).map_err(error)?;
            assert_eq!(frame.received_bytes(), input);
            assert_eq!(
                frame.value_bytes().len(),
                usize::try_from(frame.header().declared_value_length()).unwrap()
            );
            Out::TlvFrameExact(Box::new(frame.into_contract()))
        }
        Op::TlvReadIdentifier(_) => {
            let identifier = tlv::read_identifier(input, options).map_err(error)?;
            assert_eq!(identifier.identifier_bytes().as_ptr(), input.as_ptr());
            assert_eq!(
                identifier.supplied_remainder(),
                &input[identifier.consumed_bytes()..]
            );
            Out::TlvReadIdentifier(Box::new(identifier.into_contract()))
        }
        Op::TlvReadLength(_) => {
            let length = tlv::read_length(input, options).map_err(error)?;
            assert_eq!(length.length_octets().as_ptr(), input.as_ptr());
            assert_eq!(
                length.supplied_remainder(),
                &input[length.consumed_bytes()..]
            );
            Out::TlvReadLength(Box::new(length.into_contract()))
        }
        _ => panic!("unexpected fixture operation"),
    };
    Ok(proto::CodecApiOperationResult {
        result: Some(result),
    })
}

#[test]
fn all_acceptance_cases_match_borrowed_and_generated_contracts_byte_for_byte() {
    let manifest: Value =
        serde_json::from_str(include_str!("../../../vectors/tlv-framing-vectors.json")).unwrap();
    assert_eq!(manifest["schemaVersion"], 1);
    for case in manifest["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        let json = case["requestJson"].as_str().unwrap().as_bytes();
        let binary =
            reallyme_codec::hex::lower_hex_to_bytes(case["requestHex"].as_str().unwrap()).unwrap();
        let expected =
            reallyme_codec::hex::lower_hex_to_bytes(case["responseHex"].as_str().unwrap()).unwrap();
        let request: proto::CodecApiOperationRequest = decode_json(json).unwrap();
        assert_eq!(
            encode_protobuf(&request).unwrap().as_slice(),
            binary,
            "{name}"
        );
        assert_eq!(
            process_operation_response(&binary).as_slice(),
            expected,
            "{name} binary"
        );
        assert_eq!(
            process_operation_response_json(json).as_slice(),
            expected,
            "{name} JSON"
        );
        let outcome = match native(&request) {
            Ok(result) => proto::codec_api_operation_response::Outcome::Result(Box::new(result)),
            Err(error) => proto::codec_api_operation_response::Outcome::Error(Box::new(
                codec_api_error(error),
            )),
        };
        let actual = proto::CodecApiOperationResponse {
            outcome: Some(outcome),
        };
        assert_eq!(
            encode_protobuf(&actual).unwrap().as_slice(),
            expected,
            "{name} native"
        );
        // Golden responses must remain valid under the generated schema too.
        let decoded: proto::CodecApiOperationResponse = decode_protobuf(&expected).unwrap();
        assert_eq!(encode_protobuf(&decoded).unwrap().as_slice(), expected);
    }
}
