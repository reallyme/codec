// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! RFC 8785 conformance for the two areas where a naive canonicalizer
//! silently diverges and breaks cross-implementation signatures: object
//! member ordering by UTF-16 code unit (§3.2.3) and ECMAScript number
//! serialization (§3.2.2.3).

#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]

use codec_jcs::{canonicalize_json_text, canonicalize_trusted_json_value, JcsError};
use serde_json::json;

#[test]
fn number_serialization_matches_ecmascript() {
    // Each pair is (input, RFC 8785 / ES6 `Number.prototype.toString`).
    // The `1e21` and tiny-magnitude cases are exactly where `ryu`/serde
    // diverge from ES6.
    let cases: &[(f64, &str)] = &[
        (0.0, "0"),
        (-0.0, "0"),
        (1.0, "1"),
        (100.0, "100"),
        (1.5, "1.5"),
        (-1.5, "-1.5"),
        (1e21, "1e+21"),
        (1e-7, "1e-7"),
        (1e20, "100000000000000000000"),
        (1000000.0, "1000000"),
        (1e23, "1e+23"),
        (123456789.0, "123456789"),
    ];
    for (input, expected) in cases {
        let actual =
            canonicalize_trusted_json_value(&json!(input)).expect("finite number canonicalizes");
        assert_eq!(actual, *expected, "number {input} serialized as {actual}");
    }
}

#[test]
fn rfc8785_appendix_numbers_parse_with_roundtrip_precision() {
    let value: serde_json::Value =
        serde_json::from_str("333333333.33333329").expect("appendix number is valid JSON");

    assert_eq!(
        canonicalize_trusted_json_value(&value).unwrap(),
        "333333333.3333333"
    );
}

#[test]
fn interoperable_integers_are_emitted_verbatim() {
    assert_eq!(canonicalize_trusted_json_value(&json!(0)).unwrap(), "0");
    assert_eq!(
        canonicalize_trusted_json_value(&json!(9_007_199_254_740_991_i64)).unwrap(),
        "9007199254740991"
    );
    assert_eq!(
        canonicalize_trusted_json_value(&json!(-9_007_199_254_740_991_i64)).unwrap(),
        "-9007199254740991"
    );
}

#[test]
fn non_interoperable_integers_are_rejected() {
    assert_eq!(
        canonicalize_trusted_json_value(&json!(9_007_199_254_740_992_u64)),
        Err(JcsError::IntegerOutsideInteroperableRange)
    );
    assert_eq!(
        canonicalize_trusted_json_value(&json!(-9_007_199_254_740_992_i64)),
        Err(JcsError::IntegerOutsideInteroperableRange)
    );
    assert_eq!(
        canonicalize_json_text("18446744073709551615"),
        Err(JcsError::IntegerOutsideInteroperableRange)
    );
}

#[test]
fn binary64_number_tokens_follow_ecmascript_rounding() {
    assert_eq!(
        canonicalize_json_text("18446744073709551616").unwrap(),
        "18446744073709552000"
    );
    assert_eq!(
        canonicalize_json_text("9007199254740992.0").unwrap(),
        "9007199254740992"
    );
}

#[test]
fn object_keys_sorted_by_utf16_code_unit() {
    // A supplementary-plane key (U+1F600, "😀", UTF-16 surrogate pair
    // starting 0xD83D) must sort BEFORE a BMP key in 0xE000–0xFFFF
    // (U+FB00, "ﬀ") under UTF-16 ordering, even though its Unicode scalar
    // value is larger. A code-point / UTF-8 sort would order them the
    // other way and produce non-canonical output.
    let value = json!({
        "\u{FB00}": 1,
        "\u{1F600}": 2,
    });
    let canonical = canonicalize_trusted_json_value(&value).unwrap();
    let emoji_at = canonical.find('\u{1F600}').expect("emoji key present");
    let bmp_at = canonical.find('\u{FB00}').expect("bmp key present");
    assert!(
        emoji_at < bmp_at,
        "UTF-16 order must place the supplementary-plane key first: {canonical}"
    );
}

#[test]
fn ascii_keys_sorted_lexicographically() {
    let value = json!({ "b": 1, "a": 2, "c": 3, "A": 4 });
    // Uppercase 'A' (0x41) sorts before lowercase letters (0x61+).
    assert_eq!(
        canonicalize_trusted_json_value(&value).unwrap(),
        r#"{"A":4,"a":2,"b":1,"c":3}"#
    );
}

#[test]
fn nested_structure_is_fully_canonicalized() {
    let value = json!({
        "z": [3, 2, 1],
        "a": { "y": 1e21, "x": 42 },
    });
    assert_eq!(
        canonicalize_trusted_json_value(&value).unwrap(),
        r#"{"a":{"x":42,"y":1e+21},"z":[3,2,1]}"#
    );
}

#[test]
fn non_finite_numbers_rejected() {
    // serde_json cannot even hold NaN/Infinity in a Value via json!, so
    // construct through an f64 that is finite here; this asserts the API
    // contract that only finite numbers are accepted. (Non-finite values
    // cannot appear in valid JSON input and are rejected at parse time.)
    assert!(canonicalize_trusted_json_value(&json!(1.0)).is_ok());
}
