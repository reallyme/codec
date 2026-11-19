// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! String token boundaries, Unicode admission, and independent number parity.
#![allow(clippy::unwrap_used, reason = "assertions on bounded JSON fixtures")]

use codec_jcs::{canonicalize_json_text, canonicalize_trusted_json_value, JcsError};

#[test]
fn escaped_and_borrowed_unicode_tokens_preserve_json_semantics() {
    for input in [
        r#""\"\\\/\b\f\n\r\t\u0000""#,
        r#"{"\ud83d\ude00":"é漢字","\u0061":"\u00e9"}"#,
        r#"["a\\","\ud800\udc00","\udbff\udffd"]"#,
        r#"{"a":"quote: \" slash: /"}"#,
    ] {
        let value = serde_json::from_str(input).unwrap();
        assert_eq!(
            canonicalize_json_text(input),
            canonicalize_trusted_json_value(&value)
        );
    }
    assert_eq!(
        canonicalize_json_text(r#"{"a":0,"\u0061":1}"#),
        Err(JcsError::DuplicateProperty)
    );
}

#[test]
fn malformed_tokens_fail_with_stable_errors() {
    for input in [
        "",
        " ",
        "nul",
        "truefalse",
        "+1",
        "01",
        "-01",
        ".1",
        "1.",
        "1e",
        "1e+",
        "--1",
        "[1,]",
        "{\"x\":1,}",
        "{1:0}",
        "\"\n\"",
        "\"\u{0000}\"",
        "\u{000c}null",
        r#""\uD800""#,
        r#""\uDC00""#,
        r#""\uD800\u0041""#,
        r#""\u12x4""#,
        r#""\uD800\\uDC00""#,
        r#""\x20""#,
        "\"unterminated\\",
        "[null true]",
    ] {
        assert_eq!(canonicalize_json_text(input), Err(JcsError::InvalidJson));
    }
}

#[test]
fn binary64_rounding_matches_independent_serde_number_parsing() {
    for sign in ["", "-"] {
        for digits in [
            "0",
            "1",
            "123456789012345678901234567890",
            "9007199254740991",
        ] {
            for exponent in [-350, -324, -100, -1, 0, 1, 100, 308, 350] {
                let input = format!("{sign}{digits}.123456789012345678901234567890e{exponent}");
                let expected = match serde_json::from_str(&input) {
                    Ok(value) => canonicalize_trusted_json_value(&value),
                    Err(_) => Err(JcsError::InvalidJson),
                };
                assert_eq!(canonicalize_json_text(&input), expected);
            }
        }
    }
}
