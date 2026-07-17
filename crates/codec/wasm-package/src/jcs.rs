// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use codec_jcs::canonicalize_json_text;
use js_sys::JsString;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use crate::boundary::{validate_input_lengths, validate_js_inputs, zeroizing_string};
use crate::map_error::invalid_input;

#[wasm_bindgen(js_name = canonicalizeJson)]
/// Canonicalize a JSON value using RFC 8785 JCS.
pub fn canonicalize_json_wasm(value_json: &JsString) -> Result<String, JsValue> {
    validate_js_inputs(&[value_json], &[])?;
    let value_json = zeroizing_string(value_json)?;
    validate_input_lengths(&[value_json.len()])?;
    canonicalize_json_text(&value_json).map_err(|_| invalid_input())
}
