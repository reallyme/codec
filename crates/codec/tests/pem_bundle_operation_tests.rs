// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(feature = "operation-contract")]
#![allow(clippy::unwrap_used, clippy::panic, missing_docs)]

use codec_proto::generated::proto::reallyme::codec::v1 as proto;
use codec_proto::{decode_protobuf, encode_protobuf};
use reallyme_codec::operation_contract::process_operation_response;
use reallyme_codec::pem::decode_pem_bundle;

fn execute(
    input: &[u8],
    options: proto::CodecPemBundleDecodeOptions,
) -> proto::CodecApiOperationResponse {
    let request = proto::CodecApiOperationRequest::default().with_operation(
        proto::codec_api_operation_request::Operation::PemBundleDecode(Box::new(
            proto::CodecPemBundleDecodeRequest::default()
                .with_pem(input.to_vec())
                .with_options(options),
        )),
    );
    decode_protobuf(&process_operation_response(
        &encode_protobuf(&request).unwrap(),
    ))
    .unwrap()
}

#[test]
fn generated_bundle_operation_preserves_native_result_bytes_and_metadata() {
    let input = b"-----BEGIN CERTIFICATE-----\nAQID\n-----END CERTIFICATE-----\n-----BEGIN UNKNOWN-----\nBAU=\n-----END UNKNOWN-----\n";
    let options = proto::CodecPemBundleDecodeOptions::default();
    let expected = decode_pem_bundle(input, &options).unwrap();
    let response = execute(input, options);
    let Some(proto::codec_api_operation_response::Outcome::Result(result)) =
        response.outcome.as_ref()
    else {
        panic!("expected guarded operation result");
    };
    let Some(proto::codec_api_operation_result::Result::PemBundleDecode(actual)) =
        result.result.as_ref()
    else {
        panic!("expected PEM bundle result");
    };
    assert_eq!(
        encode_protobuf(&expected).unwrap(),
        encode_protobuf(actual.as_ref()).unwrap()
    );
}

#[test]
fn generated_bundle_operation_preserves_typed_resource_failure() {
    let block = b"-----BEGIN CERTIFICATE-----\nAQID\n-----END CERTIFICATE-----\n";
    let input = [block.as_slice(), block.as_slice()].concat();
    let response = execute(
        &input,
        proto::CodecPemBundleDecodeOptions::default().with_max_blocks(1),
    );
    let Some(proto::codec_api_operation_response::Outcome::Error(error)) =
        response.outcome.as_ref()
    else {
        panic!("expected typed resource failure");
    };
    let Some(proto::codec_api_error::Error::Pem(error)) = error.error.as_ref() else {
        panic!("expected PEM error family");
    };
    assert_eq!(
        error.reason.as_known(),
        Some(proto::CodecErrorReason::CODEC_ERROR_REASON_PEM_RESOURCE_LIMIT_EXCEEDED)
    );
}
