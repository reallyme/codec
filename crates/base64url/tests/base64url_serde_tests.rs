// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct ByteDto {
    #[serde(with = "codec_base64url::serde_bytes")]
    data: Vec<u8>,
}

#[derive(Debug, Deserialize, PartialEq, Serialize)]
struct OptionalByteDto {
    #[serde(with = "codec_base64url::serde_option_bytes")]
    data: Option<Vec<u8>>,
}

#[test]
fn serde_bytes_serializes_unpadded_base64url() {
    let dto = ByteDto {
        data: vec![0x00, 0x01, 0x02, 0xfb, 0xff],
    };

    let json = serde_json::to_string(&dto).unwrap();

    assert_eq!(json, r#"{"data":"AAEC-_8"}"#);
}

#[test]
fn serde_bytes_deserializes_unpadded_base64url() {
    let dto: ByteDto = serde_json::from_str(r#"{"data":"AAEC-_8"}"#).unwrap();

    assert_eq!(
        dto,
        ByteDto {
            data: vec![0x00, 0x01, 0x02, 0xfb, 0xff],
        }
    );
}

#[test]
fn serde_bytes_rejects_invalid_base64url() {
    let decoded = serde_json::from_str::<ByteDto>(r#"{"data":"AAEC+_8"}"#);

    assert!(decoded.is_err());
}

#[test]
fn serde_option_bytes_serializes_some_and_none() {
    let some_json = serde_json::to_string(&OptionalByteDto {
        data: Some(vec![0x03, 0xee]),
    })
    .unwrap();
    let none_json = serde_json::to_string(&OptionalByteDto { data: None }).unwrap();

    assert_eq!(some_json, r#"{"data":"A-4"}"#);
    assert_eq!(none_json, r#"{"data":null}"#);
}

#[test]
fn serde_option_bytes_deserializes_some_and_none() {
    let some: OptionalByteDto = serde_json::from_str(r#"{"data":"A-4"}"#).unwrap();
    let none: OptionalByteDto = serde_json::from_str(r#"{"data":null}"#).unwrap();

    assert_eq!(
        some,
        OptionalByteDto {
            data: Some(vec![0x03, 0xee]),
        }
    );
    assert_eq!(none, OptionalByteDto { data: None });
}

#[test]
fn serde_option_bytes_rejects_invalid_base64url() {
    let decoded = serde_json::from_str::<OptionalByteDto>(r#"{"data":"A-4="}"#);

    assert!(decoded.is_err());
}
