// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Wrong-type diagnostics must never retain received scalar or container data.

#![cfg(feature = "generated")]
#![allow(clippy::unwrap_used, reason = "test assertions on fixed JSON fixtures")]

use buffa::json_helpers::{ProtoBytesJson, ProtoStringJson};
use reallyme_codec_proto::generated::proto::reallyme::codec::v1::{
    CodecBytesRequest, CodecTextRequest,
};
use reallyme_codec_proto::guarded::{SecretBytes, SecretString};

const WRONG_TYPES: &[&str] = &[
    "true",
    "false",
    "8675309123456789",
    "-8675309123456789",
    "18446744073709551615",
    "-9223372036854775808",
    "8675309.123456789",
    "8.675309123456789e42",
    r#"["private-input-canary"]"#,
    r#"{"private-input-canary":8675309123456789}"#,
];

fn assert_redacted(error: serde_json::Error) {
    let message = error.to_string();
    assert!(message.starts_with("invalid guarded JSON type"));
    assert!(!message.contains("8675309"));
    assert!(!message.contains("private-input-canary"));
    assert!(!message.contains("18446744073709551615"));
    assert!(!message.contains("9223372036854775808"));
}

#[test]
fn guarded_json_visitors_reject_wrong_types_without_received_values() {
    for input in WRONG_TYPES.iter().copied().chain(["null"]) {
        let mut bytes = serde_json::Deserializer::from_str(input);
        assert_redacted(SecretBytes::deserialize_json(&mut bytes).err().unwrap());
        let mut text = serde_json::Deserializer::from_str(input);
        assert_redacted(SecretString::deserialize_json(&mut text).err().unwrap());
    }
}

#[test]
fn generated_requests_use_the_redacting_visitors_and_preserve_valid_json() {
    for input in WRONG_TYPES {
        let json = format!(r#"{{"value":{input}}}"#);
        assert_redacted(
            serde_json::from_str::<CodecBytesRequest>(&json)
                .err()
                .unwrap(),
        );
        assert_redacted(
            serde_json::from_str::<CodecTextRequest>(&json)
                .err()
                .unwrap(),
        );
    }
    let bytes: CodecBytesRequest = serde_json::from_str(r#"{"value":"AQID"}"#).unwrap();
    assert_eq!(bytes.value.as_ref(), &[1, 2, 3]);
    let text: CodecTextRequest = serde_json::from_str(r#"{"value":"public-text"}"#).unwrap();
    assert_eq!(text.value.as_ref(), "public-text");
}
