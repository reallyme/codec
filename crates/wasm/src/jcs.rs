// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use codec_adapter::scalar_ops::{canonicalize_json, JcsError};
use js_sys::JsString;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;

use crate::boundary::{js_string_from_owned, zeroizing_string};
use crate::map_error::{invalid_input, non_canonical, provider_failure};

#[wasm_bindgen(js_name = canonicalizeJson)]
/// Canonicalize a JSON value using RFC 8785 JCS.
pub fn canonicalize_json_wasm(value_json: &JsString) -> Result<JsString, JsValue> {
    let value_json = zeroizing_string(value_json)?;
    canonicalize_json(&value_json)
        .map(js_string_from_owned)
        .map_err(|error| match error {
            JcsError::DuplicateProperty => non_canonical(),
            JcsError::SerializationError | JcsError::UnsupportedNumberRepresentation => {
                provider_failure()
            }
            JcsError::InvalidJson
            | JcsError::NonFiniteNumber
            | JcsError::IntegerOutsideInteroperableRange
            | JcsError::DepthExceeded
            | JcsError::Noncharacter => invalid_input(),
            _ => provider_failure(),
        })
}
