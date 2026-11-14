// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Host-only lifecycle contracts remain separate from pure codec dispatch.
#![cfg(feature = "generated")]
#![allow(clippy::unwrap_used, reason = "bounded public lifecycle fixtures")]

use reallyme_codec_proto::generated::proto::reallyme::codec::v1 as proto;
use reallyme_codec_proto::{decode_json, decode_protobuf, encode_protobuf};

#[test]
fn lifecycle_requests_and_results_keep_matching_field_numbers() {
    for (request, result, tag) in [
        (r#"{"info":{}}"#, r#"{"info":{"abiVersion":6}}"#, 0x0a),
        (
            r#"{"verifyLibrary":{"absolutePath":"/tmp/codec.dylib","expectedSha256":"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="}}"#,
            r#"{"verifiedLibrary":{"provider":{"abiVersion":6}}}"#,
            0x12,
        ),
        (
            r#"{"loadLibrary":{"absolutePath":"/tmp/codec.dylib"}}"#,
            r#"{"loadedLibrary":{"provider":{"abiVersion":6}}}"#,
            0x1a,
        ),
        (
            r#"{"initializeWasm":{"moduleBytes":"AGFzbQ=="}}"#,
            r#"{"initializedWasm":{"abiVersion":6}}"#,
            0x22,
        ),
        (
            r#"{"reinstantiateWasm":{}}"#,
            r#"{"reinstantiatedWasm":{}}"#,
            0x2a,
        ),
    ] {
        let request: proto::CodecProviderOperationRequest =
            decode_json(request.as_bytes()).unwrap();
        let result: proto::CodecProviderOperationResult = decode_json(result.as_bytes()).unwrap();
        assert_eq!(encode_protobuf(&request).unwrap().first(), Some(&tag));
        assert_eq!(encode_protobuf(&result).unwrap().first(), Some(&tag));
    }
}

#[test]
fn cached_module_recovery_has_no_caller_module_input() {
    let request: proto::CodecProviderOperationRequest = decode_protobuf(&[0x2a, 0]).unwrap();
    assert!(matches!(
        request.operation.as_ref(),
        Some(proto::codec_provider_operation_request::Operation::ReinstantiateWasm(_))
    ));
    assert_eq!(encode_protobuf(&request).unwrap().as_slice(), &[0x2a, 0]);
    assert!(decode_json::<proto::CodecProviderOperationRequest>(
        br#"{"reinstantiateWasm":{"moduleBytes":"AGFzbQ=="}}"#,
    )
    .is_err());
    fn assert_owner<T: zeroize::ZeroizeOnDrop>() {}
    assert_owner::<proto::CodecWasmReinstantiateRequest>();
}
