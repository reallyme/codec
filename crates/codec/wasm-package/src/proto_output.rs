// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Single executable protobuf boundary for the WASM package.
//!
//! Operation-specific SDK helpers build generated messages before entering
//! this module. Keeping one binary entrypoint prevents the WASM ABI from
//! becoming a second, hand-maintained protobuf dispatch contract. Both inputs
//! return a binary `CodecProtoResultEnvelope`.

use codec_core::proto_process::{
    process_proto as process_proto_request, process_proto_json as process_proto_json_request,
};
use codec_proto::{MAX_CODEC_PROTO_JSON_BYTES, MAX_CODEC_PROTO_MESSAGE_BYTES};
use js_sys::Uint8Array;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use zeroize::Zeroizing;

use crate::boundary::zeroizing_bytes_with_maximum;

#[wasm_bindgen(js_name = processProto)]
/// Execute one generated binary protobuf request and return a binary result envelope.
pub fn process_proto(request: &Uint8Array) -> Result<Uint8Array, JsValue> {
    let request = bounded_proto_request(request, MAX_CODEC_PROTO_MESSAGE_BYTES)?;
    let output: Zeroizing<Vec<u8>> = process_proto_request(request.as_slice());
    Ok(Uint8Array::from(output.as_slice()))
}

#[wasm_bindgen(js_name = processProtoJson)]
/// Execute one generated ProtoJSON request and return a binary result envelope.
pub fn process_proto_json(request_json: &Uint8Array) -> Result<Uint8Array, JsValue> {
    let request_json = bounded_proto_request(request_json, MAX_CODEC_PROTO_JSON_BYTES)?;
    let output: Zeroizing<Vec<u8>> = process_proto_json_request(request_json.as_slice());
    Ok(Uint8Array::from(output.as_slice()))
}

fn bounded_proto_request(
    request: &Uint8Array,
    maximum: usize,
) -> Result<Zeroizing<Vec<u8>>, JsValue> {
    if let Ok(snapshot) = zeroizing_bytes_with_maximum(request, maximum) {
        return Ok(snapshot);
    }

    // Resource-limit failures are part of the protobuf result contract. A
    // bounded sentinel avoids copying an attacker-sized or concurrently
    // resized JS array while still letting the shared Rust lane build that
    // exact typed error envelope.
    let sentinel_len = maximum
        .checked_add(1)
        .ok_or_else(crate::map_error::invalid_input)?;
    Ok(Zeroizing::new(vec![0_u8; sentinel_len]))
}
