// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use wasm_bindgen::JsValue;

pub(crate) fn invalid_input() -> JsValue {
    JsValue::from_str("invalid-input")
}

pub(crate) fn non_canonical() -> JsValue {
    JsValue::from_str("non-canonical")
}

pub(crate) fn unsupported_codec() -> JsValue {
    JsValue::from_str("unsupported-codec")
}

pub(crate) fn provider_failure() -> JsValue {
    JsValue::from_str("provider-failure")
}
