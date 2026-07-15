// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]
use codec_jcs::{canonicalize_json, JcsError};
use serde_json::json;

#[test]
fn canonicalizes_null() -> Result<(), JcsError> {
    assert_eq!(canonicalize_json(&json!(null))?, "null");
    Ok(())
}

#[test]
fn canonicalizes_booleans() -> Result<(), JcsError> {
    assert_eq!(canonicalize_json(&json!(true))?, "true");
    assert_eq!(canonicalize_json(&json!(false))?, "false");
    Ok(())
}

#[test]
fn canonicalizes_numbers() -> Result<(), JcsError> {
    assert_eq!(canonicalize_json(&json!(0))?, "0");
    assert_eq!(canonicalize_json(&json!(42))?, "42");
    assert_eq!(canonicalize_json(&json!(-1))?, "-1");
    Ok(())
}

#[test]
fn canonicalizes_strings() -> Result<(), JcsError> {
    assert_eq!(canonicalize_json(&json!("hello"))?, "\"hello\"");
    Ok(())
}

#[test]
fn canonicalizes_arrays() -> Result<(), JcsError> {
    let v = json!([1, true, "x", null]);
    assert_eq!(canonicalize_json(&v)?, "[1,true,\"x\",null]");
    Ok(())
}

#[test]
fn canonicalizes_objects_sorted_keys() -> Result<(), JcsError> {
    let v = json!({ "b": 2, "a": 1 });
    assert_eq!(canonicalize_json(&v)?, "{\"a\":1,\"b\":2}");
    Ok(())
}

#[test]
fn canonicalizes_nested_objects() -> Result<(), JcsError> {
    let v = json!({
        "a": [1, 2, { "x": true }],
        "b": null
    });

    assert_eq!(
        canonicalize_json(&v)?,
        "{\"a\":[1,2,{\"x\":true}],\"b\":null}"
    );
    Ok(())
}

#[test]
fn integer_numbers_are_preserved() -> Result<(), JcsError> {
    let v = serde_json::json!(12345678901234567890u128);
    assert_eq!(canonicalize_json(&v)?, "12345678901234567890");
    Ok(())
}
