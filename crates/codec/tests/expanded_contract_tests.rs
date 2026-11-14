// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Independent literals and compatibility checks for the expanded schema.
#![cfg(feature = "operation-contract")]
#![allow(
    clippy::unwrap_used,
    clippy::panic,
    reason = "assertions on bounded test fixtures"
)]

use codec_proto::generated::proto::reallyme::codec::v1 as proto;
use codec_proto::{decode_protobuf, encode_protobuf};
use proto::codec_api_operation_request::Operation;
use proto::codec_api_operation_result::Result as Output;
use reallyme_codec::operation_contract::{
    process_operation_response, process_operation_response_json,
};

fn response(operation: Operation) -> proto::CodecApiOperationResponse {
    let request = proto::CodecApiOperationRequest {
        operation: Some(operation),
    };
    decode_protobuf(&process_operation_response(
        &encode_protobuf(&request).unwrap(),
    ))
    .unwrap()
}
fn execute(operation: Operation) -> Output {
    let mut response = response(operation);
    let Some(proto::codec_api_operation_response::Outcome::Result(result)) =
        response.outcome.as_mut()
    else {
        panic!("expected operation success");
    };
    result.result.take().unwrap()
}
fn bytes(value: &[u8]) -> Box<proto::CodecBytesRequest> {
    Box::new(proto::CodecBytesRequest {
        value: value.to_vec().into(),
    })
}
fn text(value: &str) -> Box<proto::CodecTextRequest> {
    Box::new(proto::CodecTextRequest {
        value: value.to_owned().into(),
    })
}
fn source(value: &[u8]) -> Box<proto::CodecGeneralCborSourceRequest> {
    Box::new(proto::CodecGeneralCborSourceRequest {
        encoded: value.to_vec().into(),
        options: Default::default(),
    })
}
macro_rules! literal {
    ($operation:ident, $request:expr, $expected:expr) => {
        let Output::$operation(ref result) = execute(Operation::$operation($request)) else {
            panic!("wrong result variant");
        };
        assert_eq!(result.value, $expected);
    };
}

#[test]
fn scalar_contract_matches_independent_literals() {
    literal!(Base64Encode, bytes(&[0xfb, 0xff]), "+/8=");
    literal!(Base64Decode, text("+/8="), [0xfb, 0xff]);
    literal!(Base64urlEncode, bytes(&[0xfb, 0xff]), "-_8");
    literal!(Base64urlDecode, text("-_8"), [0xfb, 0xff]);
    literal!(Base64urlDecodeBytes, bytes(b"-_8"), [0xfb, 0xff]);
    literal!(LowerHexEncode, bytes(&[0xfb, 0xff]), "fbff");
    literal!(LowerHexDecode, text("fbff"), [0xfb, 0xff]);
    literal!(Base58btcEncode, bytes(b"Hello World"), "JxF12TrwUP45BMd");
    literal!(Base58btcDecode, text("JxF12TrwUP45BMd"), b"Hello World");
    literal!(
        MultibaseBase58btcEncode,
        bytes(b"Hello World"),
        "zJxF12TrwUP45BMd"
    );
    literal!(MultibaseBase64urlEncode, bytes(&[0xfb, 0xff]), "u-_8");
    literal!(MultibaseDecode, text("u-_8"), [0xfb, 0xff]);
    literal!(
        CanonicalizeJson,
        text("{\"b\":2,\"a\":1}"),
        "{\"a\":1,\"b\":2}"
    );
    literal!(
        MulticodecStripPrefix,
        bytes(&[vec![0xed, 0x01], vec![0x11; 32]].concat()),
        vec![0x11; 32]
    );
    literal!(
        RequireSupportedMulticodec,
        text("ed25519-pub"),
        "ed25519-pub"
    );
    literal!(
        BindingTypeMatchesCodec,
        Box::new(proto::CodecBindingTypeMatchesCodecRequest {
            binding_type: "Multikey".to_owned().into(),
            codec_name: "ed25519-pub".to_owned().into(),
        }),
        true
    );
    literal!(IsValidCidString, text("invalid"), false);
    literal!(DagCborCodecCode, Box::default(), 0x71);
    let Output::TryParseCid(ref result) = execute(Operation::TryParseCid(text("invalid"))) else {
        panic!("wrong variant");
    };
    assert!(result.value.as_option().is_none());
    let Output::ProviderInfo(ref result) = execute(Operation::ProviderInfo(Box::default())) else {
        panic!("wrong variant");
    };
    assert_eq!(result.abi_version, 6);
    assert_eq!(
        (
            result.package_major,
            result.package_minor,
            result.package_patch
        ),
        (0, 4, 0)
    );
}

#[test]
fn scalar_invalid_input_has_a_caller_error_and_the_correct_family() {
    for operation in [
        Operation::Base64Decode(text("*")),
        Operation::Base64urlDecode(text("=")),
        Operation::Base64urlDecodeBytes(bytes(&[0xff])),
        Operation::LowerHexDecode(text("FF")),
        Operation::Base58btcDecode(text("0")),
        Operation::MultibaseDecode(text("x123")),
        Operation::CanonicalizeJson(text("{")),
        Operation::RequireSupportedMulticodec(text("unknown")),
    ] {
        let mut response = response(operation);
        let Some(proto::codec_api_operation_response::Outcome::Error(ref error)) =
            response.outcome.take()
        else {
            panic!("expected typed error");
        };
        assert_eq!(
            error.origin.as_known(),
            Some(proto::CodecErrorOrigin::CODEC_ERROR_ORIGIN_CALLER)
        );
        assert!(error.error.is_some());
    }
}

#[test]
fn legacy_and_expanded_envelopes_preserve_wire_bytes_and_results() {
    let old = proto::CodecOperationRequest {
        operation: Some(proto::codec_operation_request::Operation::MulticodecTable(
            Box::default(),
        )),
    };
    let expanded = proto::CodecApiOperationRequest {
        operation: Some(Operation::MulticodecTable(Box::default())),
    };
    let old_bytes = encode_protobuf(&old).unwrap();
    assert_eq!(
        old_bytes.as_slice(),
        encode_protobuf(&expanded).unwrap().as_slice()
    );
    let response = process_operation_response(&old_bytes);
    let old: proto::CodecOperationResponse = decode_protobuf(&response).unwrap();
    let expanded: proto::CodecApiOperationResponse = decode_protobuf(&response).unwrap();
    assert_eq!(
        encode_protobuf(&old).unwrap().as_slice(),
        encode_protobuf(&expanded).unwrap().as_slice()
    );
    let binary = process_operation_response(
        &encode_protobuf(&proto::CodecApiOperationRequest {
            operation: Some(Operation::Base64Encode(bytes(&[0xfb, 0xff]))),
        })
        .unwrap(),
    );
    let json = process_operation_response_json(br#"{"base64Encode":{"value":"+/8="}}"#);
    assert_eq!(binary.as_slice(), json.as_slice());
}

#[test]
fn cbor_receipts_keep_original_offsets_headers_and_boundaries() {
    let Output::GeneralCborReadHeader(ref header) =
        execute(Operation::GeneralCborReadHeader(source(&[0x18, 1])))
    else {
        panic!("wrong variant");
    };
    assert_eq!(
        (
            header.argument,
            header.encoded_length,
            header.preferred_argument
        ),
        (Some(1), 2, Some(false))
    );
    for operation in [
        Operation::GeneralCborFrameExact(source(&[0x18, 1])),
        Operation::GeneralCborFramePrefix(source(&[0x18, 1, 0])),
        Operation::GeneralCborValidateExact(source(&[0x18, 1])),
        Operation::GeneralCborValidatePrefix(source(&[0x18, 1, 0])),
    ] {
        let output = execute(operation);
        let frame = match &output {
            Output::GeneralCborFrameExact(frame)
            | Output::GeneralCborFramePrefix(frame)
            | Output::GeneralCborValidateExact(frame)
            | Output::GeneralCborValidatePrefix(frame) => frame,
            _ => panic!("wrong variant"),
        };
        let span = frame.received.as_option().unwrap();
        assert_eq!((span.offset, span.length), (0, 2));
        assert_eq!(
            frame.header.as_option().unwrap().width.as_known(),
            Some(proto::CodecGeneralCborArgumentWidth::One)
        );
    }
    let Output::GeneralCborCursor(ref frames) =
        execute(Operation::GeneralCborCursor(source(&[0, 0x18, 1])))
    else {
        panic!("wrong variant");
    };
    assert_eq!(frames.consumed_bytes, 3);
    assert_eq!(frames.values.len(), 2);
    assert_eq!(frames.values[1].received.as_option().unwrap().offset, 1);
    let request = proto::CodecGeneralCborEmbeddedRequest {
        encoded: vec![0xd8, 0x18, 0x41, 0].into(),
        outer_options: Default::default(),
        embedded_options: Default::default(),
    };
    let Output::GeneralCborValidateEmbedded(ref frame) =
        execute(Operation::GeneralCborValidateEmbedded(Box::new(request)))
    else {
        panic!("wrong variant");
    };
    assert_eq!(frame.received.as_option().unwrap().offset, 3);
    assert!(frame.validated);
}

#[test]
fn ieee_bits_and_full_negative_argument_survive_the_generated_lane() {
    for bits in [0x0000u64, 0x8000, 0x3c00, 0x7c00, 0x7e01, 0xfe01] {
        let float = proto::CodecGeneralCborFloat {
            width: proto::CodecGeneralCborFloatWidth::Binary16.into(),
            bits,
        };
        let value = proto::CodecGeneralCborValue {
            value: Some(proto::codec_general_cbor_value::Value::FloatValue(
                Box::new(float),
            )),
        };
        let request = proto::CodecGeneralCborEncodeRequest {
            value: Some(value).into(),
            options: Default::default(),
        };
        let Output::GeneralCborEncode(ref result) =
            execute(Operation::GeneralCborEncode(Box::new(request)))
        else {
            panic!("wrong variant");
        };
        let octets = u16::try_from(bits).unwrap().to_be_bytes();
        assert_eq!(result.value, [0xf9, octets[0], octets[1]]);
    }
    let request = proto::CodecGeneralCborInspectRequest {
        encoded: vec![0x3b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff].into(),
        options: Default::default(),
        inspection: proto::CodecGeneralCborInspection::CODEC_GENERAL_CBOR_INSPECTION_INTEGER.into(),
        validate: true,
    };
    let Output::GeneralCborInspect(ref mut result) =
        execute(Operation::GeneralCborInspect(Box::new(request)))
    else {
        panic!("wrong variant");
    };
    let Some(proto::codec_general_cbor_inspection_result::Value::IntegerValue(ref integer)) =
        result.value.take()
    else {
        panic!("wrong inspected type");
    };
    assert!(integer.negative);
    assert_eq!(integer.argument, u64::MAX);
}

#[test]
fn der_contract_shares_the_strict_borrowed_reader() {
    let pair = [0x30, 6, 2, 1, 1, 2, 1, 2];
    let request = proto::CodecDerPairRequest {
        encoded: pair.to_vec().into(),
        limits: Default::default(),
    };
    let Output::DerDecodeUnsignedIntegerPair(ref result) =
        execute(Operation::DerDecodeUnsignedIntegerPair(Box::new(request)))
    else {
        panic!("wrong variant");
    };
    assert_eq!(result.first, [1]);
    assert_eq!(result.second, [2]);
    let child = || proto::CodecDerSchema {
        kind: proto::CodecDerType::CODEC_DER_TYPE_UNSIGNED_INTEGER.into(),
        children: Vec::new().into(),
    };
    let request = proto::CodecDerDecodeRequest {
        encoded: pair.to_vec().into(),
        limits: Default::default(),
        schema: Some(proto::CodecDerSchema {
            kind: proto::CodecDerType::CODEC_DER_TYPE_SEQUENCE.into(),
            children: vec![child(), child()].into(),
        })
        .into(),
    };
    let Output::DerDecode(ref result) = execute(Operation::DerDecode(Box::new(request))) else {
        panic!("wrong variant");
    };
    assert_eq!(result.children[0].bytes_value, [1]);
    assert_eq!(result.children[0].received.as_option().unwrap().offset, 2);
    assert_eq!(result.children[1].contents.as_option().unwrap().offset, 7);
    let request = proto::CodecDerReadRequest {
        encoded: vec![2, 1, 1, 0].into(),
        limits: Default::default(),
        kind: proto::CodecDerType::CODEC_DER_TYPE_UNSIGNED_INTEGER.into(),
    };
    let Output::DerRead(ref result) = execute(Operation::DerRead(Box::new(request))) else {
        panic!("wrong variant");
    };
    assert_eq!(result.consumed_bytes, 3);
    assert_eq!(result.value.as_option().unwrap().bytes_value, [1]);
}

#[test]
fn new_families_preserve_precise_typed_failures() {
    let mut cbor = response(Operation::GeneralCborValidateExact(source(&[0x61, 0xff])));
    let Some(proto::codec_api_operation_response::Outcome::Error(ref mut error)) =
        cbor.outcome.take()
    else {
        panic!("expected error");
    };
    let Some(proto::codec_api_error::Error::GeneralCbor(ref error)) = error.error.take() else {
        panic!("wrong error family");
    };
    assert_eq!(
        error.detail.as_known(),
        Some(proto::CodecGeneralCborErrorReason::InvalidUtf8)
    );
    let request = proto::CodecDerPairRequest {
        encoded: vec![0x30, 0].into(),
        limits: Default::default(),
    };
    let mut der = response(Operation::DerDecodeUnsignedIntegerPair(Box::new(request)));
    let Some(proto::codec_api_operation_response::Outcome::Error(ref mut error)) =
        der.outcome.take()
    else {
        panic!("expected error");
    };
    let Some(proto::codec_api_error::Error::Der(ref error)) = error.error.take() else {
        panic!("wrong error family");
    };
    assert_eq!(
        error.detail.as_known(),
        Some(proto::CodecDerErrorReason::TruncatedInput)
    );
}

#[test]
fn generated_float_observations_and_width_validation_share_native_semantics() {
    let float = |bits| proto::CodecGeneralCborFloat {
        width: proto::CodecGeneralCborFloatWidth::Binary16.into(),
        bits,
    };
    let request = proto::CodecGeneralCborFloatPropertiesRequest {
        value: Some(float(0xfe01)).into(),
    };
    let Output::GeneralCborFloatProperties(ref result) =
        execute(Operation::GeneralCborFloatProperties(Box::new(request)))
    else {
        panic!("wrong result");
    };
    assert!(result.nan);
    assert!(!result.finite);
    assert_eq!(result.value.as_option().unwrap().bits, 0xfe01);
    assert_eq!(result.widened_binary64_bits, 0xfff8_0400_0000_0000);
    let request = proto::CodecGeneralCborFloatCompareRequest {
        left: Some(float(0x3c00)).into(),
        right: Some(proto::CodecGeneralCborFloat {
            width: proto::CodecGeneralCborFloatWidth::Binary64.into(),
            bits: 0x3ff0_0000_0000_0000,
        })
        .into(),
    };
    literal!(GeneralCborFloatEquivalent, Box::new(request), true);
    let request = proto::CodecGeneralCborFloatPropertiesRequest {
        value: Some(float(0x1_0000)).into(),
    };
    let mut response = response(Operation::GeneralCborFloatProperties(Box::new(request)));
    let Some(proto::codec_api_operation_response::Outcome::Error(ref mut error)) =
        response.outcome.take()
    else {
        panic!("expected error");
    };
    let Some(proto::codec_api_error::Error::GeneralCbor(ref error)) = error.error.take() else {
        panic!("wrong error");
    };
    assert_eq!(
        error.detail.as_known(),
        Some(proto::CodecGeneralCborErrorReason::WrongType)
    );
}

#[test]
fn generated_policy_unspecified_values_fail_closed_even_for_empty_sequences() {
    use codec_cbor::general::{Cursor, ErrorReason, LengthPolicy, Limits};
    let error = Cursor::new(
        &[],
        Limits::default(),
        LengthPolicy::CODEC_GENERAL_CBOR_LENGTH_POLICY_UNSPECIFIED,
    )
    .err()
    .unwrap();
    assert_eq!(error.reason(), ErrorReason::UnsupportedPolicy);
    let mut request = source(&[]);
    request.options = Some(proto::CodecGeneralCborOptions::default()).into();
    let mut response = response(Operation::GeneralCborCursor(request));
    let Some(proto::codec_api_operation_response::Outcome::Error(ref mut error)) =
        response.outcome.take()
    else {
        panic!("expected error");
    };
    let Some(proto::codec_api_error::Error::GeneralCbor(ref error)) = error.error.take() else {
        panic!("wrong family");
    };
    assert_eq!(
        error.detail.as_known(),
        Some(ErrorReason::UnsupportedPolicy)
    );
}

#[test]
fn all_generated_native_error_reasons_have_total_typed_mappings() {
    use buffa::Enumeration;
    use codec_proto::{codec_api_error, CodecWireError, CodecWireErrorOrigin};
    for reason in proto::CodecGeneralCborErrorReason::values()
        .iter()
        .copied()
        .filter(|reason| reason.to_i32() != 0)
    {
        let wire = CodecWireError::from_general_cbor_reason(reason);
        let mut envelope = codec_api_error(wire);
        let Some(proto::codec_api_error::Error::GeneralCbor(ref error)) = envelope.error.take()
        else {
            panic!("wrong family");
        };
        assert_eq!(error.detail.as_known(), Some(reason));
        assert_eq!(error.reason.as_known(), Some(wire.reason()));
        let expected = if matches!(
            reason,
            proto::CodecGeneralCborErrorReason::ArithmeticOverflow
                | proto::CodecGeneralCborErrorReason::AllocationFailure
        ) {
            CodecWireErrorOrigin::Provider
        } else {
            CodecWireErrorOrigin::Caller
        };
        assert_eq!(wire.origin(), expected);
    }
    for reason in proto::CodecDerErrorReason::values()
        .iter()
        .copied()
        .filter(|reason| reason.to_i32() != 0)
    {
        let wire = CodecWireError::from_der_reason(reason);
        let mut envelope = codec_api_error(wire);
        let Some(proto::codec_api_error::Error::Der(ref error)) = envelope.error.take() else {
            panic!("wrong family");
        };
        assert_eq!(error.detail.as_known(), Some(reason));
        assert_eq!(error.reason.as_known(), Some(wire.reason()));
        let expected = if matches!(
            reason,
            proto::CodecDerErrorReason::ArithmeticOverflow
                | proto::CodecDerErrorReason::AllocationFailure
        ) {
            CodecWireErrorOrigin::Provider
        } else {
            CodecWireErrorOrigin::Caller
        };
        assert_eq!(wire.origin(), expected);
    }
}

#[test]
fn cid_hash_and_multikey_compatibility_adapters_share_generated_operations() {
    let Output::DagCborComputeCid(ref cid) = execute(Operation::DagCborComputeCid(bytes(&[])))
    else {
        panic!("wrong result");
    };
    literal!(IsValidCidString, text(&cid.value), true);
    let Output::TryParseCid(ref parsed) = execute(Operation::TryParseCid(text(&cid.value))) else {
        panic!("wrong result");
    };
    assert_eq!(parsed.value.as_option().unwrap().value, cid.value);
    let digest = [
        0xe3, 0xb0, 0xc4, 0x42, 0x98, 0xfc, 0x1c, 0x14, 0x9a, 0xfb, 0xf4, 0xc8, 0x99, 0x6f, 0xb9,
        0x24, 0x27, 0xae, 0x41, 0xe4, 0x64, 0x9b, 0x93, 0x4c, 0xa4, 0x95, 0x99, 0x1b, 0x78, 0x52,
        0xb8, 0x55,
    ];
    literal!(DagCborSha256ContentHash, bytes(&[]), digest);
    literal!(
        DagCborMultihash,
        bytes(&[]),
        [&[0x12, 0x20][..], &digest[..]].concat()
    );
    let request = proto::CodecMultikeyEncodeRequest {
        codec_name: "ed25519-pub".to_owned().into(),
        public_key: vec![0x11; 32].into(),
    };
    let Output::MultikeyEncode(ref key) = execute(Operation::MultikeyEncode(Box::new(request)))
    else {
        panic!("wrong result");
    };
    let Output::MultikeyParse(ref parsed) = execute(Operation::MultikeyParse(Box::new(
        proto::CodecMultikeyParseRequest {
            multikey: key.value.clone(),
        },
    ))) else {
        panic!("wrong result");
    };
    assert_eq!(parsed.public_key, [0x11; 32]);
    let request = proto::CodecValidateKeyBindingRequest {
        binding_type: "Multikey".to_owned().into(),
        algorithm: None,
        multikey: key.value.clone(),
    };
    assert!(matches!(
        execute(Operation::ValidateKeyBinding(Box::new(request))),
        Output::ValidateKeyBinding(_)
    ));
}

#[test]
fn expanded_requests_inherit_preallocation_duplicate_field_guards() {
    let first = proto::CodecApiOperationRequest {
        operation: Some(Operation::Base64Encode(bytes(&[1]))),
    };
    let second = proto::CodecApiOperationRequest {
        operation: Some(Operation::LowerHexDecode(text("ff"))),
    };
    for request in [&first, &second] {
        let raw = [
            encode_protobuf(&first).unwrap().as_slice(),
            encode_protobuf(request).unwrap().as_slice(),
        ]
        .concat();
        let mut response: proto::CodecApiOperationResponse =
            decode_protobuf(&process_operation_response(&raw)).unwrap();
        let Some(proto::codec_api_operation_response::Outcome::Error(ref mut error)) =
            response.outcome.take()
        else {
            panic!("expected duplicate rejection");
        };
        let Some(proto::codec_api_error::Error::Boundary(ref error)) = error.error.take() else {
            panic!("wrong error family");
        };
        assert_eq!(
            error.reason.as_known(),
            Some(proto::CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MALFORMED_PROTOBUF)
        );
    }
}

#[test]
fn scalar_observations_keep_generated_zeroizing_owners() {
    use buffa::Message;
    fn protected<T: zeroize::ZeroizeOnDrop>() {}
    protected::<proto::CodecGeneralCborHeader>();
    protected::<proto::CodecGeneralCborTaggedBytes>();
    protected::<proto::CodecUint32Result>();
    let mut header = proto::CodecGeneralCborHeader::default();
    header.argument = Some(u64::MAX);
    assert!(format!("{header:?}").contains("[REDACTED]"));
    header.clear();
    assert_eq!(header.argument, None);
    let mut tagged = proto::CodecGeneralCborTaggedBytes {
        tag: u64::MAX,
        received: Default::default(),
        payload: Default::default(),
    };
    assert!(format!("{tagged:?}").contains("[REDACTED]"));
    tagged.clear();
    assert_eq!(tagged.tag, 0);
    let mut scalar = proto::CodecUint32Result { value: u32::MAX };
    assert!(format!("{scalar:?}").contains("[REDACTED]"));
    scalar.clear();
    assert_eq!(scalar.value, 0);
}

#[test]
fn protected_optional_arguments_keep_proto_json_integer_semantics() {
    let value: proto::CodecGeneralCborHeader = codec_proto::decode_json(
        br#"{"majorType":"CODEC_GENERAL_CBOR_MAJOR_TYPE_UNSIGNED","width":"CODEC_GENERAL_CBOR_ARGUMENT_WIDTH_EIGHT","argument":"18446744073709551615","encodedLength":"9"}"#,
    ).unwrap();
    assert_eq!(value.argument, Some(u64::MAX));
    assert_eq!(value.encoded_length, 9);
    let absent: proto::CodecGeneralCborHeader =
        codec_proto::decode_json(br#"{"encodedLength":"1"}"#).unwrap();
    assert_eq!(absent.argument, None);
    let null: proto::CodecGeneralCborHeader =
        codec_proto::decode_json(br#"{"argument":null}"#).unwrap();
    assert_eq!(null.argument, None);
}

#[test]
fn received_wire_fragments_restore_native_frame_proofs() {
    let received = |bytes: &[u8]| proto::CodecGeneralCborValue {
        value: Some(proto::codec_general_cbor_value::Value::ReceivedValue(
            Box::new(proto::CodecBytesResult {
                value: bytes.to_vec().into(),
            }),
        )),
    };
    let encode = |value| {
        Operation::GeneralCborEncode(Box::new(proto::CodecGeneralCborEncodeRequest {
            value: Some(value).into(),
            options: Default::default(),
        }))
    };
    literal!(
        GeneralCborEncode,
        Box::new(proto::CodecGeneralCborEncodeRequest {
            value: Some(received(&[0x18, 1])).into(),
            options: Default::default(),
        }),
        [0x18, 1]
    );
    for value in [
        received(&[0, 1]),
        proto::CodecGeneralCborValue {
            value: Some(proto::codec_general_cbor_value::Value::ArrayValue(
                Box::new(proto::CodecGeneralCborArray {
                    values: vec![received(&[0, 1])].into(),
                }),
            )),
        },
    ] {
        let mut response = response(encode(value));
        let Some(proto::codec_api_operation_response::Outcome::Error(ref mut error)) =
            response.outcome.take()
        else {
            panic!("expected trailing-data rejection");
        };
        let Some(proto::codec_api_error::Error::GeneralCbor(ref error)) = error.error.take() else {
            panic!("wrong error family");
        };
        assert_eq!(
            error.detail.as_known(),
            Some(proto::CodecGeneralCborErrorReason::TrailingData)
        );
    }
}

#[test]
fn receipt_collection_boundary_is_reachable_and_the_next_item_fails_closed() {
    let maximum = codec_cbor::general::Limits::default()
        .entries()
        .checked_mul(2)
        .unwrap();
    let Output::GeneralCborCursor(ref result) =
        execute(Operation::GeneralCborCursor(source(&vec![0; maximum])))
    else {
        panic!("wrong result");
    };
    assert_eq!(result.values.len(), maximum);
    let too_many = maximum.checked_add(1).unwrap();
    let mut response = response(Operation::GeneralCborCursor(source(&vec![0; too_many])));
    let Some(proto::codec_api_operation_response::Outcome::Error(ref mut error)) =
        response.outcome.take()
    else {
        panic!("expected entry limit");
    };
    let Some(proto::codec_api_error::Error::GeneralCbor(ref error)) = error.error.take() else {
        panic!("wrong family");
    };
    assert_eq!(
        error.detail.as_known(),
        Some(proto::CodecGeneralCborErrorReason::EntryLimit)
    );
}
