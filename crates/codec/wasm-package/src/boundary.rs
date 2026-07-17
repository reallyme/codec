// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use js_sys::{JsString, Uint8Array};
use wasm_bindgen::JsValue;
use zeroize::Zeroizing;

use crate::map_error::invalid_input;

/// Maximum aggregate caller-controlled input accepted by one WASM operation.
pub(crate) const MAX_WASM_INPUT_BYTES: usize = 1024 * 1024;

// JavaScript exposes string length in UTF-16 code units, not UTF-8 bytes. This
// first-stage limit intentionally preserves the public one-megabyte ASCII
// boundary while bounding the unavoidable wasm-bindgen UTF-8 copy to at most
// three bytes per accepted code unit. The byte-accurate check after conversion
// remains authoritative for accepted inputs.
const MAX_WASM_STRING_CODE_UNITS: usize = MAX_WASM_INPUT_BYTES;

pub(crate) fn byte_array_len(value: &Uint8Array) -> Result<usize, JsValue> {
    usize::try_from(value.length()).map_err(|_| invalid_input())
}

fn js_string_code_units(value: &JsString) -> Result<usize, JsValue> {
    usize::try_from(value.length()).map_err(|_| invalid_input())
}

/// Validate JS-owned inputs before wasm-bindgen copies strings into linear memory.
pub(crate) fn validate_js_inputs(
    strings: &[&JsString],
    byte_arrays: &[&Uint8Array],
) -> Result<(), JsValue> {
    let mut aggregate = 0_usize;
    for value in strings {
        aggregate = aggregate
            .checked_add(js_string_code_units(value)?)
            .ok_or_else(invalid_input)?;
        if aggregate > MAX_WASM_STRING_CODE_UNITS {
            return Err(invalid_input());
        }
    }
    for value in byte_arrays {
        aggregate = aggregate
            .checked_add(byte_array_len(value)?)
            .ok_or_else(invalid_input)?;
        if aggregate > MAX_WASM_INPUT_BYTES {
            return Err(invalid_input());
        }
    }
    Ok(())
}

pub(crate) fn zeroizing_string(value: &JsString) -> Result<Zeroizing<String>, JsValue> {
    validate_js_inputs(&[value], &[])?;
    value
        .as_string()
        .map(Zeroizing::new)
        .ok_or_else(invalid_input)
}

pub(crate) fn zeroizing_bytes(value: &Uint8Array) -> Result<Zeroizing<Vec<u8>>, JsValue> {
    zeroizing_bytes_with_maximum(value, MAX_WASM_INPUT_BYTES)
}

pub(crate) fn zeroizing_bytes_with_maximum(
    value: &Uint8Array,
    maximum: usize,
) -> Result<Zeroizing<Vec<u8>>, JsValue> {
    let expected_length_u32 = value.length();
    let expected_length = usize::try_from(expected_length_u32).map_err(|_| invalid_input())?;
    if expected_length > maximum {
        return Err(invalid_input());
    }

    // Do not call Uint8Array::to_vec() on caller-owned storage. js-sys sizes
    // that allocation from one length read and performs its unsafe raw copy
    // using another. A length-tracking view over a growable SharedArrayBuffer
    // can change between those reads. Bound the source view explicitly, copy
    // it into a fixed-length JavaScript owner, and only then cross into Rust.
    let bounded_view = value.subarray(0, expected_length_u32);
    if bounded_view.length() != expected_length_u32 {
        return Err(invalid_input());
    }
    let snapshot = Uint8Array::new_with_length(expected_length_u32);
    snapshot.set(bounded_view.as_ref(), 0);
    if value.length() != expected_length_u32 || bounded_view.length() != expected_length_u32 {
        snapshot.fill(0, 0, expected_length_u32);
        return Err(invalid_input());
    }

    // The snapshot owns a fixed ArrayBuffer, so js-sys cannot observe a
    // changing length during this copy. Wipe the temporary JavaScript owner as
    // soon as Rust has its zeroizing copy.
    let result = Zeroizing::new(snapshot.to_vec());
    snapshot.fill(0, 0, expected_length_u32);
    Ok(result)
}

pub(crate) fn validate_input_lengths(lengths: &[usize]) -> Result<(), JsValue> {
    validate_input_lengths_with_maximum(lengths, MAX_WASM_INPUT_BYTES)
}

pub(crate) fn validate_input_lengths_with_maximum(
    lengths: &[usize],
    maximum: usize,
) -> Result<(), JsValue> {
    let mut aggregate = 0_usize;
    for length in lengths {
        aggregate = aggregate.checked_add(*length).ok_or_else(invalid_input)?;
        if aggregate > maximum {
            return Err(invalid_input());
        }
    }
    Ok(())
}
