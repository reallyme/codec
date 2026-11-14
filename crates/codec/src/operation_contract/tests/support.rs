// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use buffa::Message;
use codec_cbor::MAX_DETERMINISTIC_CBOR_OUTPUT_LEN;
use codec_proto::generated::proto::reallyme::codec::v1::{
    __buffa::oneof::codec_error, codec_operation_response, codec_operation_result,
    CodecDagCborVerifyCidRequest, CodecDeterministicCborDecodeRequest,
    CodecDeterministicCborEncodeRequest, CodecErrorOrigin, CodecMulticodecLookupPrefixRequest,
    CodecMulticodecPrefixForNameRequest, CodecMulticodecTableRequest, CodecMultikeyParseRequest,
    CodecPemDecodeRequest,
};
use codec_proto::CodecWireErrorOrigin;

fn table_request() -> CodecOperationRequest {
    CodecOperationRequest {
        operation: Some(CodecOperation::MulticodecTable(Box::new(
            CodecMulticodecTableRequest {

            },
        ))),

    }
}
