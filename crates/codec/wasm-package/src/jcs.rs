// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use codec_jcs::canonicalize_json;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use crate::map_error::{invalid_input, non_canonical};

#[wasm_bindgen(js_name = canonicalizeJson)]
/// Canonicalize a JSON value using RFC 8785 JCS.
pub fn canonicalize_json_wasm(value_json: &str) -> Result<String, JsValue> {
    let value: serde_json::Value = serde_json::from_str(value_json).map_err(|_| invalid_input())?;
    canonicalize_json(&value).map_err(|_| non_canonical())
}
