// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(clippy::unwrap_used, missing_docs)]

use codec_base64::{
    base64_to_bytes, base64_to_bytes_with_limits, bytes_to_base64, Base64DecodeLimits, Base64Error,
    MAX_BASE64_INPUT_LEN,
};
use codec_proto::generated::proto::reallyme::codec::v1::CodecErrorReason as R;
use zeroize::Zeroizing;

fn limits(input: u64, output: u64) -> Base64DecodeLimits {
    Base64DecodeLimits {
        encoded_input_bytes: Some(input),
        decoded_output_bytes: Some(output),
    }
}

#[test]
fn exact_small_budgets_account_for_padding_before_allocation() {
    for (input, expected) in [
        ("", &b""[..]),
        ("Zg==", b"f"),
        ("Zm8=", b"fo"),
        ("Zm9v", b"foo"),
    ] {
        let bounds = limits(
            u64::try_from(input.len()).unwrap(),
            u64::try_from(expected.len()).unwrap(),
        );
        let decoded: Zeroizing<Vec<u8>> = base64_to_bytes_with_limits(input, &bounds).unwrap();
        assert_eq!(decoded.as_slice(), expected);
    }
    assert!(matches!(
        base64_to_bytes_with_limits("Zg==", &limits(3, 1)),
        Err(Base64Error::InputTooLarge)
    ));
    assert!(matches!(
        base64_to_bytes_with_limits("Zg==", &limits(4, 0)),
        Err(Base64Error::OutputTooLarge)
    ));
    assert!(matches!(
        base64_to_bytes_with_limits("Zm8=", &limits(4, 1)),
        Err(Base64Error::OutputTooLarge)
    ));
}

#[test]
fn strict_alphabet_padding_and_pad_bits_are_unchanged() {
    for input in [
        "Zg",
        "Zg=",
        "Zg===",
        "Zh==",
        "Zm9=",
        "!!!!",
        "_w==",
        "Zg==\n",
        "Zm9v!!!!",
        "data:image/jpeg;base64,Zg==",
    ] {
        assert!(
            matches!(
                base64_to_bytes_with_limits(input, &limits(100, 100)),
                Err(Base64Error::Invalid)
            ),
            "{input}"
        );
    }
}

#[test]
fn portrait_exact_limits_and_one_byte_excess_are_independent() {
    const ENCODED: u64 = 13_981_016;
    const DECODED: usize = 10_485_760;
    let payload = Zeroizing::new(vec![0x5a; DECODED]);
    let encoded = bytes_to_base64(&payload);
    assert_eq!(u64::try_from(encoded.len()).unwrap(), ENCODED);
    let bounds = limits(ENCODED, u64::try_from(DECODED).unwrap());
    assert_eq!(
        base64_to_bytes_with_limits(&encoded, &bounds)
            .unwrap()
            .as_slice(),
        payload.as_slice()
    );
    assert!(matches!(
        base64_to_bytes(&encoded),
        Err(Base64Error::InputTooLarge)
    ));

    let mut input_excess = encoded.clone();
    input_excess.push('A');
    assert!(matches!(
        base64_to_bytes_with_limits(&input_excess, &bounds),
        Err(Base64Error::InputTooLarge)
    ));
    let mut output_excess = payload;
    output_excess.push(0x5a);
    let output_excess = bytes_to_base64(&output_excess);
    // One more decoded byte still occupies the same padded Base64 length.
    assert_eq!(u64::try_from(output_excess.len()).unwrap(), ENCODED);
    assert!(matches!(
        base64_to_bytes_with_limits(&output_excess, &bounds),
        Err(Base64Error::OutputTooLarge)
    ));
}

#[test]
fn omitted_limits_keep_defaults_and_unaddressable_limits_fail_closed() {
    let encoded = "A".repeat(MAX_BASE64_INPUT_LEN);
    assert!(base64_to_bytes(&encoded).is_ok());
    assert!(base64_to_bytes_with_limits(&encoded, &Base64DecodeLimits::default()).is_ok());
    assert!(matches!(
        base64_to_bytes_with_limits(&(encoded + "A"), &Base64DecodeLimits::default()),
        Err(Base64Error::InputTooLarge)
    ));
    for bounds in [limits(u64::MAX, 1), limits(4, u64::MAX)] {
        assert!(matches!(
            base64_to_bytes_with_limits("Zg==", &bounds),
            Err(Base64Error::InvalidLimits)
        ));
    }
}

#[test]
fn every_typed_failure_has_one_schema_reason() {
    for (error, reason) in [
        (
            Base64Error::Invalid,
            R::CODEC_ERROR_REASON_BASE_INVALID_BASE64,
        ),
        (
            Base64Error::InputTooLarge,
            R::CODEC_ERROR_REASON_BASE_INPUT_TOO_LARGE,
        ),
        (
            Base64Error::OutputTooLarge,
            R::CODEC_ERROR_REASON_BASE_OUTPUT_TOO_LARGE,
        ),
        (
            Base64Error::InvalidLimits,
            R::CODEC_ERROR_REASON_BASE_INVALID_LIMITS,
        ),
        (
            Base64Error::LengthOverflow,
            R::CODEC_ERROR_REASON_BASE_LENGTH_OVERFLOW,
        ),
        (
            Base64Error::AllocationFailure,
            R::CODEC_ERROR_REASON_BASE_ALLOCATION_FAILURE,
        ),
    ] {
        assert_eq!(error.reason(), reason);
        let wire = codec_proto::CodecWireError::try_new(
            codec_proto::CodecWireErrorBranch::BaseEncoding,
            reason,
        )
        .unwrap();
        let expected_origin = match error {
            Base64Error::AllocationFailure | Base64Error::LengthOverflow => {
                codec_proto::CodecWireErrorOrigin::Provider
            }
            _ => codec_proto::CodecWireErrorOrigin::Caller,
        };
        assert_eq!(wire.origin(), expected_origin);
    }
}
