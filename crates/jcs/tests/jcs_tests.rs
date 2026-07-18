// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]
use codec_jcs::{canonicalize_json_text, canonicalize_trusted_json_value, JcsError};
use serde_json::json;

#[test]
fn canonicalizes_null() -> Result<(), JcsError> {
    assert_eq!(canonicalize_trusted_json_value(&json!(null))?, "null");
    Ok(())
}

#[test]
fn canonicalizes_booleans() -> Result<(), JcsError> {
    assert_eq!(canonicalize_trusted_json_value(&json!(true))?, "true");
    assert_eq!(canonicalize_trusted_json_value(&json!(false))?, "false");
    Ok(())
}

#[test]
fn canonicalizes_numbers() -> Result<(), JcsError> {
    assert_eq!(canonicalize_trusted_json_value(&json!(0))?, "0");
    assert_eq!(canonicalize_trusted_json_value(&json!(42))?, "42");
    assert_eq!(canonicalize_trusted_json_value(&json!(-1))?, "-1");
    Ok(())
}

#[test]
fn canonicalizes_strings() -> Result<(), JcsError> {
    assert_eq!(
        canonicalize_trusted_json_value(&json!("hello"))?,
        "\"hello\""
    );
    Ok(())
}

#[test]
fn canonical_output_uses_exact_capacity_for_escaped_sensitive_text() -> Result<(), JcsError> {
    let canonical = canonicalize_json_text(
        "{\"document\":\"line\\nwith\\tcontrols\",\"identifier\":\"did:example:alice\"}",
    )?;

    assert_eq!(
        canonical,
        "{\"document\":\"line\\nwith\\tcontrols\",\"identifier\":\"did:example:alice\"}"
    );
    assert_eq!(canonical.capacity(), canonical.len());
    Ok(())
}

#[test]
fn canonicalizes_arrays() -> Result<(), JcsError> {
    let v = json!([1, true, "x", null]);
    assert_eq!(canonicalize_trusted_json_value(&v)?, "[1,true,\"x\",null]");
    Ok(())
}

#[test]
fn canonicalizes_objects_sorted_keys() -> Result<(), JcsError> {
    let v = json!({ "b": 2, "a": 1 });
    assert_eq!(canonicalize_trusted_json_value(&v)?, "{\"a\":1,\"b\":2}");
    Ok(())
}

#[test]
fn canonicalizes_nested_objects() -> Result<(), JcsError> {
    let v = json!({
        "a": [1, 2, { "x": true }],
        "b": null
    });

    assert_eq!(
        canonicalize_trusted_json_value(&v)?,
        "{\"a\":[1,2,{\"x\":true}],\"b\":null}"
    );
    Ok(())
}

#[test]
fn integer_numbers_outside_interoperable_range_are_rejected() {
    let v = serde_json::json!(12345678901234567890u128);
    assert_eq!(
        canonicalize_trusted_json_value(&v),
        Err(JcsError::IntegerOutsideInteroperableRange)
    );
}

#[test]
fn integer_valued_binary64_numbers_outside_interoperable_range_are_rejected() {
    for input in [
        "1e19",
        "-1e19",
        "9007199254740992.0",
        "18446744073709551616",
    ] {
        assert_eq!(
            canonicalize_json_text(input),
            Err(JcsError::IntegerOutsideInteroperableRange),
            "{input} must not bypass the interoperable integer policy"
        );
    }

    assert_eq!(
        canonicalize_trusted_json_value(&serde_json::json!(1e19)),
        Err(JcsError::IntegerOutsideInteroperableRange)
    );
}

#[test]
fn raw_json_rejects_duplicate_object_members() {
    assert_eq!(
        canonicalize_json_text(r#"{"a":1,"a":2}"#),
        Err(JcsError::DuplicateProperty)
    );
    assert_eq!(
        canonicalize_json_text(r#"{"outer":{"a":1,"a":2}}"#),
        Err(JcsError::DuplicateProperty)
    );
}

#[test]
fn raw_json_rejects_invalid_or_trailing_input() {
    assert_eq!(canonicalize_json_text("{"), Err(JcsError::InvalidJson));
    assert_eq!(
        canonicalize_json_text(r#"{"a":1} {"b":2}"#),
        Err(JcsError::InvalidJson)
    );
}
