// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(feature = "operation-contract")]
#![allow(
    missing_docs,
    clippy::unwrap_used,
    clippy::panic,
    reason = "fixed CID syntax fixtures"
)]

use codec_proto::generated::proto::reallyme::codec::v1 as proto;
use codec_proto::{decode_protobuf, encode_protobuf};
use proto::codec_api_operation_request::Operation;
use proto::codec_api_operation_result::Result as Output;
use reallyme_codec::operation_contract::process_operation_response;

fn execute(operation: Operation) -> proto::CodecApiOperationResult {
    let request = proto::CodecApiOperationRequest::default().with_operation(operation);
    let mut response: proto::CodecApiOperationResponse = decode_protobuf(
        &process_operation_response(&encode_protobuf(&request).unwrap()),
    )
    .unwrap();
    let Some(proto::codec_api_operation_response::Outcome::Result(result)) =
        response.outcome.as_mut()
    else {
        panic!("expected CID operation result");
    };
    core::mem::take(result.as_mut())
}

#[test]
fn cid_inspection_operations_preserve_syntax_admission_and_verify_content_separately() {
    let expected = reallyme_codec::cbor::compute_cid_dag_cbor(&[0xf6]);
    for text in [
        "bafyrebfqwkmiw",
        "bafy777ydecylfgelno7hes5m3jpj4ushg3paxr624qoennbbhriodu25jzprg",
        concat!(
            "bafyqaqffuws2ljnfuws2ljnfuws2ljnfuws2ljnfuws2ljnfuws2ljnfuws2ljn",
            "fuws2ljnfuws2ljnfuws2ljnfuws2ljnfuws2ljnfuws2k"
        ),
    ] {
        let text_request = || {
            Box::new(proto::CodecTextRequest {
                value: text.to_owned().into(),
            })
        };
        let valid = execute(Operation::IsValidCidString(text_request()));
        let Some(Output::IsValidCidString(valid)) = valid.result.as_ref() else {
            panic!("expected CID syntax result");
        };
        assert!(valid.value);
        let parsed = execute(Operation::TryParseCid(text_request()));
        let Some(Output::TryParseCid(parsed)) = parsed.result.as_ref() else {
            panic!("expected parsed CID result");
        };
        assert_eq!(
            parsed.value.as_option().map(|value| value.value.as_str()),
            Some(text)
        );
        let verified = execute(Operation::DagCborVerifyCid(Box::new(
            proto::CodecDagCborVerifyCidRequest {
                cid: text.to_owned().into(),
                payload: vec![0xf6].into(),
            },
        )));
        let Some(Output::DagCborVerifyCid(verified)) = verified.result.as_ref() else {
            panic!("expected CID verification result");
        };
        assert!(!verified.valid);
        assert_eq!(verified.expected_cid.as_str(), expected);
        assert_eq!(verified.actual_cid.as_str(), text);
    }
}
