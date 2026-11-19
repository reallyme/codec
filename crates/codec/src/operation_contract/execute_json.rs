// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use codec_proto::generated::proto::reallyme::codec::v1::{
    codec_api_operation_request::Operation, codec_api_operation_result::Result as OperationResult,
};
use codec_proto::{CodecWireError, CodecWireErrorBranch};

pub(super) fn execute(operation: &mut Operation) -> Result<OperationResult, CodecWireError> {
    match operation {
        Operation::JsonDecode(request) => {
            let result = crate::json::decode(&request.encoded, request.limits.as_option())
                .map_err(|error| {
                    super::wire_error(CodecWireErrorBranch::Canonicalization, error.reason())
                })?;
            Ok(OperationResult::JsonDecode(Box::new(result)))
        }
        _ => Err(super::malformed_request_wire_error()),
    }
}
