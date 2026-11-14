// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Hardening must retain the pinned generator's accepted ProtoJSON forms.
#![cfg(feature = "generated")]
#![allow(clippy::unwrap_used, reason = "bounded synthetic ProtoJSON fixtures")]

use reallyme_codec_proto::generated::proto::reallyme::codec::v1 as proto;
use reallyme_codec_proto::{decode_json, encode_protobuf};

#[test]
fn generated_enum_fields_preserve_open_and_lenient_json_forms() {
    use buffa::json::{with_json_parse_options, JsonParseOptions};

    let unknown_number =
        serde_json::from_str::<proto::CodecTlvOptions>(r#"{"profile":7816234}"#).unwrap();
    assert_eq!(unknown_number.profile, buffa::EnumValue::Unknown(7_816_234));
    let null = serde_json::from_str::<proto::CodecTlvOptions>(r#"{"profile":null}"#).unwrap();
    assert_eq!(
        null.profile.as_known(),
        Some(proto::CodecTlvProfile::Unspecified)
    );

    let unknown_name = r#"{"profile":"synthetic-private-enum-name"}"#;
    assert!(serde_json::from_str::<proto::CodecTlvOptions>(unknown_name).is_err());
    let options = JsonParseOptions::new().ignore_unknown_enum_values(true);
    with_json_parse_options(&options, || {
        let ignored = serde_json::from_str::<proto::CodecTlvOptions>(unknown_name).unwrap();
        assert_eq!(
            ignored.profile.as_known(),
            Some(proto::CodecTlvProfile::Unspecified)
        );
        assert!(
            serde_json::from_str::<proto::CodecTlvProfile>(r#""synthetic-private-enum-name""#)
                .is_err()
        );
    });
}

#[test]
fn standalone_enum_json_rejects_unrecognized_values_without_echoing_them() {
    for (json, expected) in [
        (
            r#""CODEC_TLV_PROFILE_ISO7816_DEFINITE_BER""#,
            proto::CodecTlvProfile::Iso7816DefiniteBer,
        ),
        ("1", proto::CodecTlvProfile::Iso7816DefiniteBer),
        ("null", proto::CodecTlvProfile::Unspecified),
    ] {
        assert_eq!(
            serde_json::from_str::<proto::CodecTlvProfile>(json).unwrap(),
            expected
        );
    }
    for json in [
        r#""synthetic-private-enum-name""#,
        "7816234",
        "4294967297",
        "-4294967297",
        "93817.5",
    ] {
        let error = serde_json::from_str::<proto::CodecTlvProfile>(json).unwrap_err();
        assert!(error.is_data());
        assert!(!error.to_string().contains(json.trim_matches('"')));
    }
}

#[test]
fn null_strings_and_bools_keep_generated_default_semantics() {
    let text: proto::CodecTextRequest = decode_json(br#"{"value":null}"#).unwrap();
    assert!(text.value.is_empty());
    let boolean: proto::CodecBoolResult = decode_json(br#"{"value":null}"#).unwrap();
    assert!(!boolean.value);
    let received: proto::CodecGeneralCborFrame = decode_json(br#"{"validated":null}"#).unwrap();
    assert!(!received.validated);
}

#[test]
fn null_sensitive_lists_are_empty_but_null_elements_are_invalid() {
    let array: proto::CodecGeneralCborArray = decode_json(br#"{"values":null}"#).unwrap();
    assert!(array.values.is_empty());
    let schema: proto::CodecDerSchema = decode_json(br#"{"children":null}"#).unwrap();
    assert!(schema.children.is_empty());
    assert!(decode_json::<proto::CodecGeneralCborArray>(br#"{"values":[null]}"#).is_err());
    assert!(decode_json::<proto::CodecDerSchema>(br#"{"children":[null]}"#).is_err());
}

#[test]
fn quoted_uint32_and_null_keep_generated_integer_semantics() {
    let result: proto::CodecUint32Result = decode_json(br#"{"value":"4294967295"}"#).unwrap();
    assert_eq!(result.value, u32::MAX);
    let result: proto::CodecUint32Result = decode_json(br#"{"value":null}"#).unwrap();
    assert_eq!(result.value, 0);
    let result: proto::CodecDerValue = decode_json(br#"{"unusedBits":"0"}"#).unwrap();
    assert_eq!(result.unused_bits, Some(0));
    for json in [br#"{"value":"4294967296"}"#.as_slice(), br#"{"value":-1}"#] {
        assert!(decode_json::<proto::CodecUint32Result>(json).is_err());
    }
}

#[test]
fn empty_json_containers_preserve_oneof_and_native_default_bytes() {
    let request: proto::CodecApiOperationRequest =
        decode_json(br#"{"generalCborEncode":{"value":{"arrayValue":{"values":null}}}}"#).unwrap();
    let expected = proto::CodecApiOperationRequest {
        operation: Some(
            proto::codec_api_operation_request::Operation::GeneralCborEncode(Box::new(
                proto::CodecGeneralCborEncodeRequest {
                    value: Some(proto::CodecGeneralCborValue {
                        value: Some(proto::codec_general_cbor_value::Value::ArrayValue(
                            Box::default(),
                        )),
                    })
                    .into(),
                    options: Default::default(),
                },
            )),
        ),
    };
    assert_eq!(
        encode_protobuf(&request).unwrap(),
        encode_protobuf(&expected).unwrap()
    );
}
