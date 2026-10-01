// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(missing_docs)]
use codec_jcs::{
    canonicalize_json_text, canonicalize_trusted_json_value, JcsError, MAX_NESTING_DEPTH,
};

#[test]
fn private_number_token_never_collides_with_numeric_json() {
    let number = canonicalize_json_text(r#"{"a":1.5}"#);
    let literal = canonicalize_json_text(r#"{"a":{"$serde_json::private::Number":"1.5"}}"#);
    let valid = match (&number, &literal) {
        (Ok(number), Ok(literal)) => number != literal,
        (
            Err(JcsError::UnsupportedNumberRepresentation),
            Err(JcsError::UnsupportedNumberRepresentation),
        ) => true,
        _ => false,
    };
    assert!(
        valid,
        "unexpected number representation: {number:?}, {literal:?}"
    );
}

#[test]
fn rejects_ijson_noncharacters_in_names_and_values() {
    for input in [
        r#"{"a":"\uFFFF"}"#,
        r#"{"\uFDD0":1}"#,
        r#"{"a":"\uDBFF\uDFFF"}"#,
    ] {
        assert_eq!(canonicalize_json_text(input), Err(JcsError::Noncharacter));
    }
}
use serde_json::json;

#[test]
fn trusted_value_depth_accepts_exact_limit_and_rejects_one_more() {
    // This entry point accepts caller-built values, which do not pass through
    // the text parser's independent recursion guard.
    let mut exact = serde_json::Value::Null;
    for _ in 0..MAX_NESTING_DEPTH {
        exact = serde_json::Value::Array(vec![exact]);
    }
    let expected = format!(
        "{}null{}",
        "[".repeat(MAX_NESTING_DEPTH),
        "]".repeat(MAX_NESTING_DEPTH)
    );
    assert_eq!(canonicalize_trusted_json_value(&exact), Ok(expected));

    let over_limit = serde_json::Value::Array(vec![exact]);
    assert_eq!(
        canonicalize_trusted_json_value(&over_limit),
        Err(JcsError::DepthExceeded)
    );
}

#[test]
fn json_text_depth_guard_accepts_limit_and_rejects_deeper_inputs() {
    let exact_array = format!(
        "{}null{}",
        "[".repeat(MAX_NESTING_DEPTH),
        "]".repeat(MAX_NESTING_DEPTH)
    );
    assert_eq!(
        canonicalize_json_text(&exact_array),
        Ok(exact_array.clone())
    );

    let over_array = format!("[{exact_array}]");
    assert_eq!(
        canonicalize_json_text(&over_array),
        Err(JcsError::DepthExceeded)
    );

    let over_object = format!(
        "{}null{}",
        "{\"a\":".repeat(MAX_NESTING_DEPTH + 1),
        "}".repeat(MAX_NESTING_DEPTH + 1)
    );
    assert_eq!(
        canonicalize_json_text(&over_object),
        Err(JcsError::DepthExceeded)
    );

    // The guard must stop adversarial depth before serde or Rust recurses far
    // enough to exhaust a WASM or FFI stack.
    assert_eq!(
        canonicalize_json_text(&"[".repeat(100_000)),
        Err(JcsError::DepthExceeded)
    );
    assert_eq!(
        canonicalize_json_text(&"{\"a\":".repeat(100_000)),
        Err(JcsError::DepthExceeded)
    );
}

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

#[test]
fn malformed_objects_fail_after_accepting_sensitive_keys_and_values() {
    for input in [
        r#"{"identity-key":{"name":"example"},"pending-key":"#,
        r#"{"identity-key":{"name":"example"},"pending-key":[true,}"#,
        r#"{"identity-key":{"name":"example"},"unterminated-key"#,
        r#"{"identity-key":{"name":"example"},"identity-key":{"broken":}"#,
    ] {
        assert_eq!(canonicalize_json_text(input), Err(JcsError::InvalidJson));
    }
}
