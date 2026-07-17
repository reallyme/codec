// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use codec_multicodec::{
    CodecLookupResult, CodecSpec, CodecTag, KeyMaterialKind, VARIABLE_KEY_LENGTH,
};
use js_sys::{Object, Reflect, Uint8Array};
use wasm_bindgen::JsValue;

use crate::map_error::provider_failure;

pub(crate) fn set_bool(object: &Object, name: &str, value: bool) -> Result<(), JsValue> {
    Reflect::set(object, &JsValue::from_str(name), &JsValue::from_bool(value))
        .map_err(|_| provider_failure())?;
    Ok(())
}

pub(crate) fn set_bytes(object: &Object, name: &str, value: &[u8]) -> Result<(), JsValue> {
    Reflect::set(object, &JsValue::from_str(name), &Uint8Array::from(value))
        .map_err(|_| provider_failure())?;
    Ok(())
}

pub(crate) fn set_string(object: &Object, name: &str, value: &str) -> Result<(), JsValue> {
    Reflect::set(object, &JsValue::from_str(name), &JsValue::from_str(value))
        .map_err(|_| provider_failure())?;
    Ok(())
}

pub(crate) fn set_u32(object: &Object, name: &str, value: usize) -> Result<(), JsValue> {
    let value = u32::try_from(value).map_err(|_| provider_failure())?;
    Reflect::set(
        object,
        &JsValue::from_str(name),
        &JsValue::from_f64(f64::from(value)),
    )
    .map_err(|_| provider_failure())?;
    Ok(())
}

fn codec_tag_name(tag: CodecTag) -> Result<&'static str, JsValue> {
    match tag {
        CodecTag::Encryption => Ok("encryption"),
        CodecTag::Hash => Ok("hash"),
        CodecTag::Key => Ok("key"),
        CodecTag::Multihash => Ok("multihash"),
        CodecTag::Multikey => Ok("multikey"),
        _ => Err(provider_failure()),
    }
}

fn key_material_name(kind: KeyMaterialKind) -> Result<&'static str, JsValue> {
    match kind {
        KeyMaterialKind::NotKey => Ok("not-key"),
        KeyMaterialKind::PublicKey => Ok("public-key"),
        KeyMaterialKind::PrivateKey => Ok("private-key"),
        KeyMaterialKind::SymmetricKey => Ok("symmetric-key"),
        _ => Err(provider_failure()),
    }
}

pub(crate) fn codec_spec_to_js(name: &str, spec: &CodecSpec) -> Result<JsValue, JsValue> {
    let object = Object::new();
    set_string(&object, "name", name)?;
    set_string(&object, "alg", spec.alg)?;
    set_string(&object, "tag", codec_tag_name(spec.tag)?)?;
    set_string(
        &object,
        "keyMaterial",
        key_material_name(spec.key_material)?,
    )?;
    set_bytes(&object, "prefix", spec.codec)?;
    if spec.key_length != VARIABLE_KEY_LENGTH {
        set_u32(&object, "expectedKeyLength", spec.key_length)?;
    }
    Ok(object.into())
}

pub(crate) fn codec_lookup_to_js(found: CodecLookupResult) -> Result<JsValue, JsValue> {
    let object = Object::new();
    set_string(&object, "name", found.name)?;
    set_string(&object, "alg", found.alg)?;
    set_string(&object, "tag", codec_tag_name(found.tag)?)?;
    set_string(
        &object,
        "keyMaterial",
        key_material_name(found.key_material)?,
    )?;
    set_bytes(&object, "prefix", found.codec)?;
    if found.key_length != VARIABLE_KEY_LENGTH {
        set_u32(&object, "expectedKeyLength", found.key_length)?;
    }
    Ok(object.into())
}
