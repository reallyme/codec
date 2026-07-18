// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Single executable protobuf boundary for the WASM package.
//!
//! Operation-specific SDK helpers build generated messages before entering
//! this module. Keeping one response operation contract prevents the WASM ABI from
//! becoming a second, hand-maintained protobuf dispatch surface. Binary
//! protobuf and generated ProtoJSON both return a fully discriminated
//! `CodecOperationResponse`.

use codec_core::operation_contract::{
    process_operation_response as process_operation_response_request,
    process_operation_response_json as process_operation_response_json_request,
};
use codec_proto::{MAX_CODEC_PROTO_JSON_BYTES, MAX_CODEC_PROTO_MESSAGE_BYTES};
use js_sys::Uint8Array;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use zeroize::Zeroizing;

use crate::boundary::zeroizing_bytes_with_maximum;

#[wasm_bindgen(js_name = processOperation)]
/// Execute one generated binary protobuf request and return a fully discriminated response.
pub fn process_operation(request: &Uint8Array) -> Result<Uint8Array, JsValue> {
    let request = bounded_proto_request(request, MAX_CODEC_PROTO_MESSAGE_BYTES)?;
    let output: Zeroizing<Vec<u8>> = process_operation_response_request(request.as_slice());
    Ok(Uint8Array::from(output.as_slice()))
}

#[wasm_bindgen(js_name = processOperationJson)]
/// Execute one generated ProtoJSON request and return a discriminated response.
pub fn process_operation_json(request_json: &Uint8Array) -> Result<Uint8Array, JsValue> {
    let request_json = bounded_proto_request(request_json, MAX_CODEC_PROTO_JSON_BYTES)?;
    let output: Zeroizing<Vec<u8>> =
        process_operation_response_json_request(request_json.as_slice());
    Ok(Uint8Array::from(output.as_slice()))
}

fn bounded_proto_request(
    request: &Uint8Array,
    maximum: usize,
) -> Result<Zeroizing<Vec<u8>>, JsValue> {
    if let Ok(snapshot) = zeroizing_bytes_with_maximum(request, maximum) {
        return Ok(snapshot);
    }

    // Resource-limit failures are part of the operation-response operation contract. A
    // bounded sentinel avoids copying an attacker-sized or concurrently
    // resized JS array while still letting the shared Rust lane build that
    // exact typed error response.
    let sentinel_len = maximum
        .checked_add(1)
        .ok_or_else(crate::map_error::invalid_input)?;
    Ok(Zeroizing::new(vec![0_u8; sentinel_len]))
}
