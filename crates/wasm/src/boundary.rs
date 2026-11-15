// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use js_sys::{Function, JsString, Object, Reflect, Symbol, Uint8Array};
use wasm_bindgen::convert::TryFromJsValue;
use wasm_bindgen::{JsCast, JsValue};
use zeroize::Zeroizing;

use crate::map_error::invalid_input;

/// Maximum aggregate caller-controlled input accepted by one WASM operation.
pub(crate) const MAX_WASM_INPUT_BYTES: usize = 1024 * 1024;
const UINT8_ARRAY_BRAND: &str = "Uint8Array";
const SHADOWED_BYTE_METADATA: [&str; 6] = [
    "constructor",
    "length",
    "byteLength",
    "byteOffset",
    "buffer",
    "subarray",
];

fn checked_array_length(value: &Uint8Array) -> Result<u32, JsValue> {
    // A detached typed array reports length zero. Read it before and after
    // the copy so a length race fails closed without reusing caller storage.
    // Raw WASM callers can bypass TypeScript and pass null despite the Rust
    // signature; js-sys length() would throw through a non-catch binding.
    let raw: &JsValue = value.as_ref();
    // The intrinsic brand distinguishes the actual element type even when a
    // different view has been given Uint8Array.prototype. Proxies lack the slot;
    // inspect it before any operation that could invoke a proxy prototype trap.
    let brand = typed_array_getter(Symbol::to_string_tag().as_ref())?
        .call0(raw)
        .map_err(|_| invalid_input())?;
    if brand != JsValue::from_str(UINT8_ARRAY_BRAND) {
        return Err(invalid_input());
    }
    let native_prototype = Object::get_prototype_of(&Uint8Array::new_with_length(0));
    if !Object::is(&Object::get_prototype_of(value), &native_prototype) {
        return Err(invalid_input());
    }
    // Native subarray consults constructor/species. Reject caller overrides
    // before that operation can invoke a getter or substitute different bytes.
    for property in SHADOWED_BYTE_METADATA {
        if !Object::get_own_property_descriptor(value, &JsValue::from_str(property)).is_undefined()
        {
            return Err(invalid_input());
        }
    }
    let length = typed_array_getter(&JsValue::from_str("length"))?
        .call0(raw)
        .map_err(|_| invalid_input())?;
    let number = length.as_f64().ok_or_else(invalid_input)?;
    // Bindgen's numeric conversion follows WebAssembly truncation semantics.
    // Prove the integer fits first, including views larger than four GiB.
    if !number.is_finite() || number < 0.0 || number.fract() != 0.0 || number > f64::from(u32::MAX)
    {
        return Err(invalid_input());
    }
    u32::try_from_js_value(length).map_err(|_| invalid_input())
}

fn typed_array_getter(property: &JsValue) -> Result<Function, JsValue> {
    let typed_array_prototype =
        Object::get_prototype_of(&Object::get_prototype_of(&Uint8Array::new_with_length(0)));
    let descriptor = Object::get_own_property_descriptor(&typed_array_prototype, property);
    Reflect::get(&descriptor, &JsValue::from_str("get"))?
        .dyn_into::<Function>()
        .map_err(|_| invalid_input())
}

fn typed_array_method(name: &str) -> Result<Function, JsValue> {
    let prototype = Object::get_prototype_of(&Uint8Array::new_with_length(0));
    Reflect::get(&prototype, &JsValue::from_str(name))?
        .dyn_into::<Function>()
        .map_err(|_| invalid_input())
}

fn checked_subarray(value: &Uint8Array, start: u32, end: u32) -> Result<Uint8Array, JsValue> {
    typed_array_method("subarray")?
        .call2(value.as_ref(), &JsValue::from(start), &JsValue::from(end))?
        .dyn_into::<Uint8Array>()
        .map_err(|_| invalid_input())
}

fn checked_set(value: &Uint8Array, source: &JsValue, offset: u32) -> Result<(), JsValue> {
    let _ = typed_array_method("set")?.call2(value.as_ref(), source, &JsValue::from(offset))?;
    Ok(())
}

/// Transfer an encoded string to JavaScript while wiping its Rust allocation.
pub(crate) fn js_string_from_owned(value: String) -> JsString {
    let value = Zeroizing::new(value);
    JsString::from(value.as_str())
}

pub(crate) fn byte_array_len(value: &Uint8Array) -> Result<usize, JsValue> {
    usize::try_from(checked_array_length(value).map_err(|_| invalid_input())?)
        .map_err(|_| invalid_input())
}

fn utf8_bytes_for_code_unit(code_unit: u16) -> usize {
    if code_unit <= 0x007f {
        1
    } else if code_unit <= 0x07ff {
        2
    } else {
        3
    }
}

fn utf8_byte_len_for_js_string(value: &JsString) -> Result<usize, JsValue> {
    // Raw callers can pass null through a JsString export. Calling length()
    // on it would throw inside WASM and leave the bindgen stack unwound.
    let raw: &JsValue = value.as_ref();
    if !raw.is_string() {
        return Err(invalid_input());
    }
    let code_units = usize::try_from(value.length()).map_err(|_| invalid_input())?;
    if code_units > MAX_WASM_INPUT_BYTES {
        return Err(invalid_input());
    }

    let mut bytes = 0_usize;
    let mut units = value.iter().peekable();
    while let Some(code_unit) = units.next() {
        // Reject unpaired surrogates before wasm-bindgen replaces them with
        // U+FFFD. Canonicalization must never sign or hash a repaired document.
        let width = if (0xd800..=0xdbff).contains(&code_unit) {
            match units.peek().copied() {
                Some(0xdc00..=0xdfff) => {
                    let _ = units.next();
                    4
                }
                _ => return Err(invalid_input()),
            }
        } else if (0xdc00..=0xdfff).contains(&code_unit) {
            return Err(invalid_input());
        } else {
            utf8_bytes_for_code_unit(code_unit)
        };
        bytes = bytes.checked_add(width).ok_or_else(invalid_input)?;
    }
    Ok(bytes)
}

fn checked_input_total(used: usize, length: usize, maximum: usize) -> Option<usize> {
    let total = used.checked_add(length)?;
    (total <= maximum).then_some(total)
}

/// Validate JS-owned inputs before wasm-bindgen copies strings into linear memory.
pub(crate) fn validate_js_inputs(
    strings: &[&JsString],
    byte_arrays: &[&Uint8Array],
) -> Result<(), JsValue> {
    let mut aggregate = 0_usize;
    for value in strings {
        aggregate = checked_input_total(
            aggregate,
            utf8_byte_len_for_js_string(value)?,
            MAX_WASM_INPUT_BYTES,
        )
        .ok_or_else(invalid_input)?;
    }
    for value in byte_arrays {
        aggregate = checked_input_total(aggregate, byte_array_len(value)?, MAX_WASM_INPUT_BYTES)
            .ok_or_else(invalid_input)?;
    }
    Ok(())
}

pub(crate) fn zeroizing_string(value: &JsString) -> Result<Zeroizing<String>, JsValue> {
    validate_js_inputs(&[value], &[])?;
    let output = value
        .as_string()
        .map(Zeroizing::new)
        .ok_or_else(invalid_input)?;
    validate_input_lengths(&[output.len()])?;
    Ok(output)
}

pub(crate) fn zeroizing_bytes(value: &Uint8Array) -> Result<Zeroizing<Vec<u8>>, JsValue> {
    zeroizing_bytes_with_maximum(value, MAX_WASM_INPUT_BYTES)
}

fn snapshot_capacity(length: u32, maximum: usize) -> Option<u32> {
    let native_length = usize::try_from(length).ok()?;
    (native_length <= maximum).then_some(length)
}

fn matches_observed_length(expected: u32, observed: Option<u32>) -> bool {
    observed == Some(expected)
}

fn snapshot_copy_is_complete(
    expected: u32,
    copied: bool,
    source_after: Option<u32>,
    view_after: Option<u32>,
) -> bool {
    copied
        && matches_observed_length(expected, source_after)
        && matches_observed_length(expected, view_after)
}

pub(crate) fn zeroizing_bytes_with_maximum(
    value: &Uint8Array,
    maximum: usize,
) -> Result<Zeroizing<Vec<u8>>, JsValue> {
    let source_length = checked_array_length(value).map_err(|_| invalid_input())?;
    let capacity = snapshot_capacity(source_length, maximum).ok_or_else(invalid_input)?;

    // Do not call Uint8Array::to_vec() on caller-owned storage. js-sys sizes
    // that allocation from one length read and performs its unsafe raw copy
    // using another. A length-tracking view over a growable SharedArrayBuffer
    // can change between those reads. Bound the source view explicitly, copy
    // it into a fixed-length JavaScript owner, and only then cross into Rust.
    let bounded_view = checked_subarray(value, 0, capacity).map_err(|_| invalid_input())?;
    if !matches_observed_length(capacity, checked_array_length(&bounded_view).ok()) {
        return Err(invalid_input());
    }
    let snapshot = Uint8Array::new_with_length(capacity);
    let copied = checked_set(&snapshot, bounded_view.as_ref(), 0);
    let source_after = checked_array_length(value).ok();
    let view_after = checked_array_length(&bounded_view).ok();
    if !snapshot_copy_is_complete(capacity, copied.is_ok(), source_after, view_after) {
        snapshot.fill(0, 0, capacity);
        return Err(invalid_input());
    }

    // The snapshot owns a fixed ArrayBuffer, so js-sys cannot observe a
    // changing length during this copy. Wipe the temporary JavaScript owner as
    // soon as Rust has its zeroizing copy.
    let result = Zeroizing::new(snapshot.to_vec());
    snapshot.fill(0, 0, capacity);
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
        aggregate = checked_input_total(aggregate, *length, maximum).ok_or_else(invalid_input)?;
    }
    Ok(())
}

#[cfg(kani)]
#[path = "boundary_proof_tests.rs"]
mod proofs;
