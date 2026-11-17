// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(feature = "operation-contract")]
#![allow(
    missing_docs,
    clippy::unwrap_used,
    clippy::panic,
    reason = "bounded key fixtures"
)]

use codec_proto::generated::proto::reallyme::codec::v1 as proto;
use codec_proto::{decode_protobuf, encode_protobuf};
use reallyme_codec::operation_contract::process_operation_response;

fn assert_strip_outcome(prefix: &[u8], payload: &[u8], accepted: bool) {
    let mut value = prefix.to_vec();
    value.extend_from_slice(payload);
    let request = proto::CodecApiOperationRequest::default().with_operation(
        proto::codec_api_operation_request::Operation::MulticodecStripPrefix(Box::new(
            proto::CodecBytesRequest {
                value: value.into(),
            },
        )),
    );
    let response_bytes = process_operation_response(&encode_protobuf(&request).unwrap());
    let response: proto::CodecApiOperationResponse = decode_protobuf(&response_bytes).unwrap();
    if accepted {
        let Some(proto::codec_api_operation_response::Outcome::Result(result)) =
            response.outcome.as_ref()
        else {
            panic!("expected valid public-key payload");
        };
        let Some(proto::codec_api_operation_result::Result::MulticodecStripPrefix(result)) =
            result.result.as_ref()
        else {
            panic!("expected stripped public-key payload");
        };
        assert_eq!(result.value.as_ref(), payload);
    } else {
        let Some(proto::codec_api_operation_response::Outcome::Error(error)) =
            response.outcome.as_ref()
        else {
            panic!("expected typed multicodec rejection");
        };
        assert_eq!(
            error.origin.as_known(),
            Some(proto::CodecErrorOrigin::CODEC_ERROR_ORIGIN_CALLER)
        );
        let Some(proto::codec_api_error::Error::Multiformat(error)) = error.error.as_ref() else {
            panic!("expected multiformat error family");
        };
        assert_eq!(
            error.reason.as_known(),
            Some(proto::CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_INVALID_MULTICODEC_PREFIX)
        );
        // Even the reported 600 KiB RSA reproduction returns a small failure.
        assert!(response_bytes.len() < 64);
    }
}

#[test]
fn strip_operation_rejects_bad_ec_tags_for_every_supported_curve() {
    for (prefix, length) in [
        ([0x80, 0x24], 33),
        ([0x81, 0x24], 49),
        ([0x82, 0x24], 67),
        ([0xe7, 0x01], 33),
    ] {
        for tag in [0x02, 0x03, 0x04] {
            let mut payload = vec![0; length];
            payload[0] = tag;
            assert_strip_outcome(&prefix, &payload, tag != 0x04);
        }
        assert_strip_outcome(&prefix, &vec![0x02; length.checked_sub(1).unwrap()], false);
    }
}

#[test]
fn strip_operation_enforces_rsa_maximum_before_allocating_a_result() {
    for length in [0, 1, 4_096, 4_097, 600_000] {
        assert_strip_outcome(
            &[0x85, 0x24],
            &vec![0x30; length],
            (1..=4_096).contains(&length),
        );
    }
}
