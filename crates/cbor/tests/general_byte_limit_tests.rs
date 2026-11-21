// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Binary budgets are independent of defaults, framing overhead, and text policy.
#![allow(
    missing_docs,
    clippy::unwrap_used,
    reason = "bounded byte-limit fixtures"
)]

use codec_cbor::general::{
    encode, encode_value, frame_exact, frame_prefix, validate_exact, CborError, Cursor,
    ErrorReason, LengthPolicy, Limits, Profile, ValueRef, DEFAULT_BYTES, MAX_BYTES, MAX_TEXT_BYTES,
};
use codec_proto::generated::proto::reallyme::codec::v1 as proto;
use zeroize::{ZeroizeOnDrop, Zeroizing};

const LARGE_STRING_HEADER_BYTES: usize = 5;

fn reason<T>(result: Result<T, CborError>, expected: ErrorReason) {
    assert_eq!(result.err().map(CborError::reason), Some(expected));
}

fn limits(input: usize, output: usize, bytes: usize) -> Limits {
    Limits::default()
        .with_input_bytes(input)
        .unwrap()
        .with_output_bytes(output)
        .unwrap()
        .with_byte_string_bytes(bytes)
        .unwrap()
}

fn byte_string(payload: usize) -> Zeroizing<Vec<u8>> {
    let mut encoded = Zeroizing::new(vec![0x5a]);
    encoded.extend_from_slice(&u32::try_from(payload).unwrap().to_be_bytes());
    encoded.resize(
        payload.checked_add(LARGE_STRING_HEADER_BYTES).unwrap(),
        0xa5,
    );
    encoded
}

#[test]
fn omitted_zero_exact_ceiling_and_unrepresentable_limits_keep_the_contract() {
    let default = Limits::try_from(&proto::CodecGeneralCborOptions::default()).unwrap();
    assert_eq!(DEFAULT_BYTES, 1_048_576);
    assert_eq!(MAX_BYTES, 2_097_152);
    assert_eq!(default.input_bytes(), DEFAULT_BYTES);
    assert_eq!(default.output_bytes(), DEFAULT_BYTES);
    assert_eq!(default.byte_string_bytes(), DEFAULT_BYTES);
    assert_eq!(default.text_bytes(), MAX_TEXT_BYTES);
    for value in [0, 1, DEFAULT_BYTES, MAX_BYTES] {
        let options = proto::CodecGeneralCborOptions {
            input_bytes: Some(u64::try_from(value).unwrap()),
            output_bytes: Some(u64::try_from(value).unwrap()),
            byte_string_bytes: Some(u64::try_from(value).unwrap()),
            ..Default::default()
        };
        assert_eq!(
            Limits::try_from(&options).unwrap(),
            limits(value, value, value)
        );
    }
    for value in [MAX_BYTES.checked_add(1).unwrap(), usize::MAX] {
        reason(default.with_input_bytes(value), ErrorReason::InvalidLimit);
        reason(default.with_output_bytes(value), ErrorReason::InvalidLimit);
        reason(
            default.with_byte_string_bytes(value),
            ErrorReason::InvalidLimit,
        );
    }
    for options in [
        proto::CodecGeneralCborOptions {
            input_bytes: Some(u64::MAX),
            ..Default::default()
        },
        proto::CodecGeneralCborOptions {
            output_bytes: Some(u64::MAX),
            ..Default::default()
        },
        proto::CodecGeneralCborOptions {
            byte_string_bytes: Some(u64::MAX),
            ..Default::default()
        },
    ] {
        reason(Limits::try_from(&options), ErrorReason::InvalidLimit);
    }
    reason(
        default.with_text_bytes(MAX_TEXT_BYTES.checked_add(1).unwrap()),
        ErrorReason::InvalidLimit,
    );
    reason(
        frame_exact(&[0x40], limits(0, 0, 0), LengthPolicy::DefiniteOnly),
        ErrorReason::InputLimit,
    );
    reason(
        encode(&ValueRef::Bytes(&[]), Profile::preferred(), limits(1, 0, 0)),
        ErrorReason::OutputLimit,
    );
    validate_exact(&[0x40], Profile::preferred(), limits(1, 1, 0)).unwrap();
}

#[test]
fn exact_encoded_budgets_count_headers_and_default_admission_stays_one_mib() {
    fn protected<T: ZeroizeOnDrop>() {}
    protected::<codec_cbor::general::EncodedCbor>();
    for selected in [
        DEFAULT_BYTES,
        DEFAULT_BYTES.checked_add(1).unwrap(),
        MAX_BYTES,
    ] {
        let payload = selected.checked_sub(LARGE_STRING_HEADER_BYTES).unwrap();
        let expected = byte_string(payload);
        let selected_limits = limits(selected, selected, payload);
        let encoded = encode(
            &ValueRef::Bytes(&expected[LARGE_STRING_HEADER_BYTES..]),
            Profile::preferred(),
            selected_limits,
        )
        .unwrap();
        assert_eq!(encoded.as_bytes(), expected.as_slice());
        let item = validate_exact(&expected, Profile::preferred(), selected_limits).unwrap();
        assert_eq!(item.frame().received_bytes().as_ptr(), expected.as_ptr());
        assert_eq!(item.frame().as_bytes().unwrap().len(), payload);
        frame_exact(&expected, selected_limits, LengthPolicy::DefiniteOnly).unwrap();
        reason(
            validate_exact(
                &expected,
                Profile::preferred(),
                selected_limits
                    .with_input_bytes(selected.checked_sub(1).unwrap())
                    .unwrap(),
            ),
            ErrorReason::InputLimit,
        );
        reason(
            encode(
                &ValueRef::Bytes(&expected[LARGE_STRING_HEADER_BYTES..]),
                Profile::preferred(),
                selected_limits
                    .with_output_bytes(selected.checked_sub(1).unwrap())
                    .unwrap(),
            ),
            ErrorReason::OutputLimit,
        );
        reason(
            validate_exact(
                &expected,
                Profile::preferred(),
                selected_limits
                    .with_byte_string_bytes(payload.checked_sub(1).unwrap())
                    .unwrap(),
            ),
            ErrorReason::ByteStringLimit,
        );
        reason(
            encode(
                &ValueRef::Bytes(&expected[LARGE_STRING_HEADER_BYTES..]),
                Profile::preferred(),
                selected_limits
                    .with_byte_string_bytes(payload.checked_sub(1).unwrap())
                    .unwrap(),
            ),
            ErrorReason::ByteStringLimit,
        );
        if selected > DEFAULT_BYTES {
            reason(
                frame_prefix(&expected, Limits::default(), LengthPolicy::DefiniteOnly),
                ErrorReason::InputLimit,
            );
            reason(
                encode(
                    &ValueRef::Bytes(&expected[LARGE_STRING_HEADER_BYTES..]),
                    Profile::preferred(),
                    Limits::default().with_byte_string_bytes(MAX_BYTES).unwrap(),
                ),
                ErrorReason::OutputLimit,
            );
        }
    }
    let over = byte_string(
        MAX_BYTES
            .checked_sub(LARGE_STRING_HEADER_BYTES)
            .unwrap()
            .checked_add(1)
            .unwrap(),
    );
    reason(
        frame_exact(
            &over,
            limits(MAX_BYTES, MAX_BYTES, MAX_BYTES),
            LengthPolicy::DefiniteOnly,
        ),
        ErrorReason::InputLimit,
    );
}

#[test]
fn received_fragments_obey_selected_input_limits_in_both_encoder_adapters() {
    let received = byte_string(DEFAULT_BYTES);
    let raised = limits(MAX_BYTES, MAX_BYTES, MAX_BYTES);
    let frame = frame_exact(&received, raised, LengthPolicy::DefiniteOnly).unwrap();
    let generated = proto::CodecGeneralCborValue::default().with_value(
        proto::codec_general_cbor_value::Value::ReceivedValue(Box::new(
            proto::CodecBytesResult::default().with_value(received.to_vec()),
        )),
    );
    for selected in [DEFAULT_BYTES, received.len()] {
        let selected_limits = raised.with_input_bytes(selected).unwrap();
        let native = encode(
            &ValueRef::Received(&frame),
            Profile::preferred(),
            selected_limits,
        );
        let wire = encode_value(&generated, Profile::preferred(), selected_limits);
        if selected == DEFAULT_BYTES {
            reason(native, ErrorReason::InputLimit);
            reason(wire, ErrorReason::InputLimit);
        } else {
            assert_eq!(native.unwrap().as_bytes(), received.as_slice());
            assert_eq!(wire.unwrap().as_bytes(), received.as_slice());
        }
    }
    // Constructed bytes have no received CBOR input; output remains independent.
    encode(
        &ValueRef::Bytes(&received[LARGE_STRING_HEADER_BYTES..]),
        Profile::preferred(),
        raised.with_input_bytes(0).unwrap(),
    )
    .unwrap();
}

#[test]
fn aggregate_bytes_accumulate_across_containers_and_cursors() {
    let payload = Zeroizing::new(vec![0xa5; DEFAULT_BYTES / 2]);
    let values = [ValueRef::Bytes(&payload), ValueRef::Bytes(&payload)];
    let total = payload.len().checked_mul(2).unwrap();
    let raised = limits(MAX_BYTES, MAX_BYTES, total);
    let encoded = encode(&ValueRef::Array(&values), Profile::preferred(), raised).unwrap();
    let item = validate_exact(encoded.as_bytes(), Profile::preferred(), raised).unwrap();
    assert_eq!(item.children().unwrap().count(), 2);
    let lower = raised
        .with_byte_string_bytes(total.checked_sub(1).unwrap())
        .unwrap();
    reason(
        encode(&ValueRef::Array(&values), Profile::preferred(), lower),
        ErrorReason::ByteStringLimit,
    );
    reason(
        validate_exact(encoded.as_bytes(), Profile::preferred(), lower),
        ErrorReason::ByteStringLimit,
    );
    let sequence = &encoded.as_bytes()[1..];
    let mut cursor = Cursor::new(sequence, lower, LengthPolicy::DefiniteOnly).unwrap();
    cursor.next().unwrap().unwrap();
    reason(cursor.next().unwrap(), ErrorReason::ByteStringLimit);
    assert!(cursor.next().is_none());
    let mut cursor = Cursor::new(sequence, raised, LengthPolicy::DefiniteOnly).unwrap();
    assert_eq!(cursor.by_ref().map(Result::unwrap).count(), 2);
    assert_eq!(cursor.consumed_bytes(), sequence);
}

#[test]
fn embedded_and_outer_budgets_are_separate_and_received_bytes_are_unchanged() {
    let inner = byte_string(DEFAULT_BYTES);
    let mut outer = Zeroizing::new(vec![0xd8, 0x18, 0x5a]);
    outer.extend_from_slice(&u32::try_from(inner.len()).unwrap().to_be_bytes());
    outer.extend_from_slice(&inner);
    let raised = limits(MAX_BYTES, MAX_BYTES, MAX_BYTES);
    let outer_item = validate_exact(&outer, Profile::preferred(), raised).unwrap();
    let tagged = outer_item.frame().tagged_bytes().unwrap();
    assert_eq!(tagged.received_bytes(), outer.as_slice());
    assert_eq!(tagged.payload_bytes(), inner.as_slice());
    reason(
        tagged.validate_embedded(Profile::preferred(), Limits::default()),
        ErrorReason::InputLimit,
    );
    let embedded = tagged
        .validate_embedded(Profile::preferred(), raised)
        .unwrap();
    assert_eq!(embedded.frame().received_bytes(), inner.as_slice());
    reason(
        validate_exact(
            &outer,
            Profile::preferred(),
            raised
                .with_byte_string_bytes(inner.len().checked_sub(1).unwrap())
                .unwrap(),
        ),
        ErrorReason::ByteStringLimit,
    );
    reason(
        tagged.validate_embedded(
            Profile::preferred(),
            raised
                .with_byte_string_bytes(DEFAULT_BYTES.checked_sub(1).unwrap())
                .unwrap(),
        ),
        ErrorReason::ByteStringLimit,
    );
}

#[test]
fn raised_binary_limits_do_not_weaken_text_or_structural_checks() {
    let raised = limits(MAX_BYTES, MAX_BYTES, MAX_BYTES);
    for (input, expected) in [
        (&[0x42, 0x01][..], ErrorReason::Truncated),
        (&[0x61, 0xff][..], ErrorReason::InvalidUtf8),
        (&[0x00, 0x00][..], ErrorReason::TrailingData),
        (
            &[0xa2, 0x00, 0x01, 0x00, 0x02][..],
            ErrorReason::DuplicateMapKey,
        ),
    ] {
        reason(
            validate_exact(input, Profile::preferred(), raised),
            expected,
        );
    }
    let text = Zeroizing::new("a".repeat(MAX_TEXT_BYTES.checked_add(1).unwrap()));
    reason(
        encode(&ValueRef::Text(&text), Profile::preferred(), raised),
        ErrorReason::TextLimit,
    );
    reason(
        validate_exact(
            &[0x81, 0x00],
            Profile::preferred(),
            raised.with_nodes(1).unwrap(),
        ),
        ErrorReason::NodeLimit,
    );
    reason(
        validate_exact(
            &[0x81, 0x00],
            Profile::preferred(),
            raised.with_entries(0).unwrap(),
        ),
        ErrorReason::EntryLimit,
    );
    reason(
        validate_exact(
            &[0x81, 0x00],
            Profile::preferred(),
            raised.with_depth(0).unwrap(),
        ),
        ErrorReason::DepthLimit,
    );
    reason(
        validate_exact(&[0x00], Profile::preferred(), raised.with_work(0).unwrap()),
        ErrorReason::WorkLimit,
    );
}
