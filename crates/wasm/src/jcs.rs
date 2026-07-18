// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use codec_core::scalar_ops::canonicalize_json;
use js_sys::JsString;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use crate::boundary::zeroizing_string;
use crate::map_error::invalid_input;

#[wasm_bindgen(js_name = canonicalizeJson)]
/// Canonicalize a JSON value using RFC 8785 JCS.
pub fn canonicalize_json_wasm(value_json: &JsString) -> Result<String, JsValue> {
    let value_json = zeroizing_string(value_json)?;
    canonicalize_json(&value_json).map_err(|_| invalid_input())
}
