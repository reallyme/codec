// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Default scalar values must retain genuine schema presence in both transports.
#![cfg(feature = "generated")]
#![allow(clippy::unwrap_used, reason = "bounded synthetic presence fixtures")]

use buffa::Message;
use reallyme_codec_proto::generated::proto::reallyme::codec::v1 as proto;
use reallyme_codec_proto::{decode_json, decode_protobuf, encode_protobuf};
use serde::{de::DeserializeOwned, Serialize};

fn roundtrip<M: Message + Serialize + DeserializeOwned>(value: &M, expected: &[u8]) -> M {
    let binary = encode_protobuf(value).unwrap();
    assert_eq!(binary.as_slice(), expected);
    let decoded: M = decode_protobuf(&binary).unwrap();
    assert_eq!(encode_protobuf(&decoded).unwrap().as_slice(), expected);
    let json = serde_json::to_vec(value).unwrap();
    let decoded: M = decode_json(&json).unwrap();
    assert_eq!(encode_protobuf(&decoded).unwrap().as_slice(), expected);
    decoded
}

#[test]
fn absent_and_empty_algorithm_remain_distinct() {
    let mut value = proto::CodecValidateKeyBindingRequest::default();
    assert_eq!(roundtrip(&value, &[]).algorithm, None);
    value.algorithm = Some(String::new().into());
    assert_eq!(roundtrip(&value, &[0x12, 0]).algorithm.as_deref(), Some(""));
    let absent: proto::CodecValidateKeyBindingRequest =
        decode_json(br#"{"algorithm":null}"#).unwrap();
    assert_eq!(absent.algorithm, None);
    let empty: proto::CodecValidateKeyBindingRequest = decode_json(br#"{"algorithm":""}"#).unwrap();
    assert_eq!(empty.algorithm.as_deref(), Some(""));
}

#[test]
fn absent_and_zero_limits_remain_distinct() {
    let mut cbor = proto::CodecGeneralCborOptions::default();
    assert_eq!(roundtrip(&cbor, &[]).input_bytes, None);
    cbor.input_bytes = Some(0);
    assert_eq!(roundtrip(&cbor, &[0x28, 0]).input_bytes, Some(0));
    let mut der = proto::CodecDerLimits::default();
    assert_eq!(roundtrip(&der, &[]).input_len, None);
    der.input_len = Some(0);
    assert_eq!(roundtrip(&der, &[0x08, 0]).input_len, Some(0));
}

#[test]
fn false_and_zero_observations_remain_present() {
    let mut header = proto::CodecGeneralCborHeader::default();
    assert_eq!(roundtrip(&header, &[]).preferred_argument, None);
    header.argument = Some(0);
    header.preferred_argument = Some(false);
    header.preferred_float = Some(false);
    let decoded = roundtrip(&header, &[0x18, 0, 0x28, 0, 0x30, 0]);
    assert_eq!(decoded.argument, Some(0));
    assert_eq!(decoded.preferred_argument, Some(false));
    assert_eq!(decoded.preferred_float, Some(false));
    let mut der = proto::CodecDerValue::default();
    assert_eq!(roundtrip(&der, &[]).bool_value, None);
    der.bool_value = Some(false);
    der.unused_bits = Some(0);
    let decoded = roundtrip(&der, &[0x28, 0, 0x30, 0]);
    assert_eq!(decoded.bool_value, Some(false));
    assert_eq!(decoded.unused_bits, Some(0));
}

#[test]
fn empty_messages_in_oneof_or_optional_message_keep_presence() {
    let mut value = proto::CodecGeneralCborValue::default();
    assert!(roundtrip(&value, &[]).value.is_none());
    value.value = Some(proto::codec_general_cbor_value::Value::BytesValue(
        Box::default(),
    ));
    let decoded = roundtrip(&value, &[0x1a, 0]);
    assert!(
        matches!(decoded.value.as_ref(), Some(proto::codec_general_cbor_value::Value::BytesValue(bytes)) if bytes.value.is_empty())
    );
    let mut text = proto::CodecOptionalTextResult::default();
    assert!(roundtrip(&text, &[]).value.as_option().is_none());
    text.value = Some(proto::CodecTextResult::default()).into();
    let decoded = roundtrip(&text, &[0x0a, 0]);
    assert_eq!(
        decoded.value.as_option().map(|value| value.value.as_str()),
        Some("")
    );
}
