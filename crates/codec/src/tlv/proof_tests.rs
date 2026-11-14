// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{
    frame_exact, frame_prefix, read_header, read_length, ErrorReason, Limits, Options, Profile,
    MAX_LENGTH_OCTETS,
};

const MAX_LENGTH_FIELD_BYTES: usize = 1 + MAX_LENGTH_OCTETS;
const MAX_FRAME_INPUT_BYTES: usize = 12;

#[kani::proof]
#[kani::unwind(17)] // Also covers the generated option and receipt cleanup.
fn definite_length_matches_wire_format() {
    let bytes: [u8; MAX_LENGTH_FIELD_BYTES] = kani::any();
    let available: usize = kani::any();
    let maximum: u8 = kani::any();
    let der: bool = kani::any();
    kani::assume(available <= bytes.len());
    kani::assume((1..=MAX_LENGTH_OCTETS).contains(&usize::from(maximum)));
    let input = &bytes[..available];
    let options = options(der, maximum);

    let discriminator = bytes[0];
    let count = usize::from(discriminator & 0x7f);
    let expected = match discriminator {
        _ if available == 0 => Err(ErrorReason::TruncatedLength),
        0..=0x7f => Ok((1, u64::from(discriminator))),
        0x80 => Err(ErrorReason::UnsupportedIndefiniteLength),
        0xff => Err(ErrorReason::InvalidLength),
        _ if count > usize::from(maximum) => Err(ErrorReason::LengthLimit),
        _ if count >= available => Err(ErrorReason::TruncatedLength),
        _ => {
            let end = 1 + count;
            let mut argument = [0; MAX_LENGTH_OCTETS];
            argument[MAX_LENGTH_OCTETS - count..].copy_from_slice(&input[1..end]);
            let declared = u64::from_be_bytes(argument);
            if der && (argument[MAX_LENGTH_OCTETS - count] == 0 || declared <= 127) {
                Err(ErrorReason::NoncanonicalLength)
            } else {
                Ok((end, declared))
            }
        }
    };
    let actual = read_length(input, &options)
        .map(|length| {
            let end = length.consumed_bytes();
            assert_eq!(input.get(..end), Some(length.length_octets()));
            assert_eq!(input.get(end..), Some(length.supplied_remainder()));
            (end, length.declared_value_length())
        })
        .map_err(super::TlvError::reason);
    kani::cover!(actual == Ok((9, u64::MAX)), "full uint64 length");
    kani::cover!(
        actual == Err(ErrorReason::NoncanonicalLength),
        "DER rejects overlong length"
    );
    kani::cover!(
        !der && bytes[0] == 0x81 && actual == Ok((2, 1)),
        "BER admits overlong length"
    );
    assert_eq!(actual, expected);
}

#[kani::proof]
#[kani::unwind(17)] // Also covers the generated option and receipt cleanup.
fn complete_frame_and_receipt_partition_the_supplied_source() {
    let bytes: [u8; MAX_FRAME_INPUT_BYTES] = kani::any();
    let available: usize = kani::any();
    let der: bool = kani::any();
    kani::assume(available <= bytes.len());
    let input = &bytes[..available];
    let options = options(der, 8);
    let header = read_header(input, &options);
    let prefix = frame_prefix(input, &options);
    let exact = frame_exact(input, &options).map(|frame| frame.consumed_bytes());

    kani::cover!(
        matches!(&prefix, Ok((_, rest)) if rest.is_empty()),
        "exact frame"
    );
    kani::cover!(
        matches!(&prefix, Ok((_, rest)) if !rest.is_empty()),
        "frame with trailing bytes"
    );
    kani::cover!(
        matches!(&prefix, Err(err) if err.reason() == ErrorReason::TruncatedValue),
        "header with incomplete value"
    );

    match (header, prefix) {
        (Err(header_error), Err(frame_error)) => {
            assert_eq!(frame_error, header_error);
            assert_eq!(exact, Err(header_error));
        }
        (Ok(header), Err(frame_error)) => {
            assert_eq!(frame_error.reason(), ErrorReason::TruncatedValue);
            assert!(
                header.declared_value_length()
                    > u64::try_from(header.supplied_remainder().len()).unwrap()
            );
            assert_eq!(exact, Err(frame_error));
        }
        (Ok(header), Ok((frame, remainder))) => {
            let start = header.consumed_bytes();
            let end = frame.consumed_bytes();
            let length = frame.value_bytes().len();
            assert!(start > 0 && end <= input.len());
            assert_eq!(start.checked_add(length), Some(end));
            assert_eq!(u64::try_from(length), Ok(header.declared_value_length()));
            assert_eq!(input.get(..end), Some(frame.received_bytes()));
            assert_eq!(input.get(start..end), Some(frame.value_bytes()));
            assert_eq!(input.get(end..), Some(remainder));
            if remainder.is_empty() {
                assert_eq!(exact, Ok(end));
            } else {
                assert!(matches!(exact, Err(err) if err.reason() == ErrorReason::TrailingData));
            }

            let receipt = frame.into_contract();
            assert_eq!(u64::try_from(end), Ok(receipt.consumed_bytes));
            let received = receipt.received.as_option().unwrap();
            assert_eq!(received.offset, 0);
            assert_eq!(u64::try_from(end), Ok(received.length));
            let contents = receipt.contents.as_option().unwrap();
            assert_eq!(u64::try_from(start), Ok(contents.offset));
            assert_eq!(u64::try_from(length), Ok(contents.length));
        }
        (Err(_), Ok(_)) => panic!("a complete frame requires a valid header"),
    }
}

fn options(der: bool, length_octets: u8) -> Options {
    let profile = if der {
        Profile::CODEC_TLV_PROFILE_ASN1_DER_FRAMING
    } else {
        Profile::CODEC_TLV_PROFILE_ISO7816_DEFINITE_BER
    };
    Options {
        profile: profile.into(),
        limits: Some(Limits {
            length_octets: Some(u64::from(length_octets)),
            ..Limits::default()
        })
        .into(),
    }
}
