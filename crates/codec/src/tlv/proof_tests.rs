// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::frame::{frame_from_header, require_exact_frame, value_end};
use super::{
    read_header, read_identifier, read_length, ErrorReason, Limits, Options, Profile, TlvError,
    MAX_IDENTIFIER_OCTETS, MAX_LENGTH_OCTETS,
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
#[kani::unwind(34)]
fn identifier_admission_matches_the_first_terminator_and_profile() {
    let bytes: [u8; MAX_IDENTIFIER_OCTETS] = kani::any();
    let supplied: u8 = kani::any();
    let cap: u8 = kani::any();
    let der: bool = kani::any();
    kani::assume(usize::from(supplied) <= bytes.len());
    kani::assume((1..=MAX_IDENTIFIER_OCTETS).contains(&usize::from(cap)));
    let input = &bytes[..usize::from(supplied)];
    let mut options = options(der, 8);
    options.limits = Some(Limits {
        identifier_octets: Some(u64::from(cap)),
        ..Limits::default()
    })
    .into();

    let actual = read_identifier(input, &options)
        .map(|identifier| {
            let end = identifier.consumed_bytes();
            assert!(core::ptr::eq(identifier.identifier_bytes(), &input[..end]));
            assert!(core::ptr::eq(
                identifier.supplied_remainder(),
                &input[end..]
            ));
            let receipt = identifier.contract().identifier.as_option().unwrap();
            assert_eq!(receipt.offset, 0);
            assert_eq!(usize::try_from(receipt.length), Ok(end));
            assert_eq!(
                usize::try_from(identifier.contract().consumed_bytes),
                Ok(end)
            );
            end
        })
        .map_err(TlvError::reason);
    let expected = identifier_reference(input, usize::from(cap), der);
    assert_eq!(actual, expected);
    kani::cover!(actual == Ok(1), "short identifier");
    kani::cover!(
        actual == Ok(MAX_IDENTIFIER_OCTETS),
        "longest accepted identifier"
    );
    kani::cover!(
        der && input == [0x5f, 0x01] && actual == Err(ErrorReason::NoncanonicalIdentifier),
        "DER rejects high-tag form for a low tag"
    );
    kani::cover!(
        !der && input == [0x5f, 0x01] && actual == Ok(2),
        "ISO 7816 admits its encoded tag"
    );
    kani::cover!(
        actual == Err(ErrorReason::IdentifierLimit),
        "the limit is reached before the terminator"
    );
    kani::cover!(
        actual == Err(ErrorReason::TruncatedIdentifier),
        "source ends within the limit"
    );
}

fn identifier_reference(input: &[u8], cap: usize, der: bool) -> Result<usize, ErrorReason> {
    let Some((&initial, rest)) = input.split_first() else {
        return Err(ErrorReason::EmptyInput);
    };
    if initial == 0 {
        return Err(ErrorReason::InvalidIdentifier);
    }
    if initial % 32 != 31 {
        return Ok(1);
    }

    // The search space excludes bytes beyond the identifier limit. This
    // specifies the precedence of the cap over any subsequent source byte.
    let admitted = &rest[..rest.len().min(cap.checked_sub(1).unwrap())];
    if admitted.first().is_some_and(|byte| byte % 128 == 0) {
        return Err(if der {
            ErrorReason::NoncanonicalIdentifier
        } else {
            ErrorReason::InvalidIdentifier
        });
    }
    match admitted.iter().position(|&byte| byte < 128) {
        Some(0) if der && admitted[0] < 31 => Err(ErrorReason::NoncanonicalIdentifier),
        Some(index) => Ok(index.checked_add(2).unwrap()),
        None if input.len() >= cap => Err(ErrorReason::IdentifierLimit),
        None => Err(ErrorReason::TruncatedIdentifier),
    }
}

#[kani::proof]
fn declared_length_admits_exactly_the_remaining_source() {
    let total: usize = kani::any();
    let start: usize = kani::any();
    let declared: u64 = kani::any();
    kani::assume(start <= total);
    let available = total.checked_sub(start).unwrap();
    let end = u128::from(u64::try_from(start).unwrap()) + u128::from(declared);
    let total_wide = u128::from(u64::try_from(total).unwrap());
    let actual = value_end(start, available, declared).map_err(TlvError::reason);
    let expected = if end <= total_wide {
        Ok(usize::try_from(end).unwrap())
    } else {
        Err(ErrorReason::TruncatedValue)
    };
    assert_eq!(actual, expected);
    kani::cover!(
        end == total_wide && declared > 0,
        "value consumes the entire source"
    );
    kani::cover!(
        end < total_wide && declared == 0,
        "zero-length value with trailing bytes"
    );
    kani::cover!(
        start > 0 && end > u128::from(u64::MAX),
        "declared size would overflow a uint64 offset"
    );
    kani::cover!(
        declared == u64::from(u32::MAX) && total == start,
        "large declaration with no value bytes"
    );
}

#[kani::proof]
#[kani::unwind(17)] // Also covers the generated option and receipt cleanup.
fn frame_receipt_and_exact_admission_preserve_the_source_partition() {
    const HEADER_BYTES: usize = 2;
    const OCTET_STRING_TAG: u8 = 0x04;
    let mut bytes: [u8; MAX_FRAME_INPUT_BYTES] = kani::any();
    let declared: u8 = kani::any();
    let supplied: u8 = kani::any();
    let der: bool = kani::any();
    kani::assume(declared < 128);
    kani::assume((HEADER_BYTES..=bytes.len()).contains(&usize::from(supplied)));
    (bytes[0], bytes[1]) = (OCTET_STRING_TAG, declared);
    let input = &bytes[..usize::from(supplied)];

    // Identifier and length admission are covered independently. Here the
    // two-octet header feeds the same constructor as the public prefix API.
    let header = read_header(input, &options(der, 8)).unwrap();
    assert_eq!(header.consumed_bytes(), HEADER_BYTES);
    let framed = frame_from_header(input, header);
    let end = HEADER_BYTES.checked_add(usize::from(declared)).unwrap();
    if end > input.len() {
        assert!(matches!(framed, Err(error) if error.reason() == ErrorReason::TruncatedValue));
    } else {
        let (frame, rest) = framed.unwrap();
        assert_eq!(frame.consumed_bytes(), end);
        assert!(core::ptr::eq(frame.received_bytes(), &input[..end]));
        assert!(core::ptr::eq(
            frame.value_bytes(),
            &input[HEADER_BYTES..end]
        ));
        assert!(core::ptr::eq(rest, &input[end..]));
        // Exact admission consumes the same frame used for the span checks.
        let exact = require_exact_frame((frame, rest));
        if rest.is_empty() {
            let receipt = exact.unwrap().into_contract();
            assert_eq!(usize::try_from(receipt.consumed_bytes), Ok(end));
            let received = receipt.received.as_option().unwrap();
            assert_eq!(received.offset, 0);
            assert_eq!(usize::try_from(received.length), Ok(end));
            let contents = receipt.contents.as_option().unwrap();
            assert_eq!(usize::try_from(contents.offset), Ok(HEADER_BYTES));
            assert_eq!(contents.length, u64::from(declared));
        } else {
            assert!(matches!(exact, Err(error) if error.reason() == ErrorReason::TrailingData));
        }
    }
    kani::cover!(end == input.len(), "exact frame serializes source spans");
    kani::cover!(
        end < input.len(),
        "exact API rejects trailing bytes after prefix admission"
    );
    kani::cover!(end > input.len(), "valid header with truncated value");
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
