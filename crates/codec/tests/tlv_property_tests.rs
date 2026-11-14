// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Resource, truncation, and byte-preservation properties for borrowed TLV views.
#![cfg(feature = "tlv")]
#![allow(clippy::unwrap_used, reason = "public synthetic framing properties")]

use proptest::prelude::{any, prop_assert, prop_assert_eq, proptest};
use reallyme_codec::tlv::{self, ErrorReason, Limits, Options, Profile};

fn options(profile: Profile) -> Options {
    Options {
        profile: profile.into(),
        limits: buffa::MessageField::none(),
    }
}

fn ber() -> Options {
    options(Profile::CODEC_TLV_PROFILE_ISO7816_DEFINITE_BER)
}

#[test]
fn end_of_contents_and_zero_initial_high_tag_groups_are_not_definite_values() {
    for profile in [
        Profile::CODEC_TLV_PROFILE_ISO7816_DEFINITE_BER,
        Profile::CODEC_TLV_PROFILE_ASN1_DER_FRAMING,
    ] {
        let options = options(profile);
        for input in [&[0, 0][..], &[0x5f, 0, 0], &[0x5f, 0x80, 1, 0]] {
            assert!(tlv::read_header(input, &options).is_err());
        }
    }
}

#[test]
fn actual_input_limits_include_trailing_bytes_and_accept_the_exact_maximum() {
    let mut input = vec![0u8; tlv::DEFAULT_INPUT_BYTES];
    input[..2].copy_from_slice(&[4, 0]);
    assert_eq!(
        tlv::read_header(&input, &ber())
            .unwrap()
            .declared_value_length(),
        0
    );
    assert_eq!(
        tlv::frame_prefix(&input, &ber()).unwrap().1.len(),
        input.len() - 2
    );
    input.push(0);
    assert_eq!(
        tlv::read_header(&input, &ber()).err().unwrap().reason(),
        ErrorReason::InputLimit
    );
}

#[test]
fn all_framing_reads_respect_the_explicit_sixteen_mibibyte_source_limit() {
    let mut input = vec![0; tlv::MAX_INPUT_BYTES + 1];
    input[0] = 4;
    let source = &input[..tlv::MAX_INPUT_BYTES];
    let mut options = ber();
    options.limits = Some(Limits {
        input_bytes: Some(u64::try_from(tlv::MAX_INPUT_BYTES).unwrap()),
        ..Limits::default()
    })
    .into();
    assert_eq!(
        tlv::read_header(source, &options).unwrap().received_bytes(),
        &[4, 0]
    );
    assert_eq!(
        tlv::read_identifier(source, &options)
            .unwrap()
            .identifier_bytes(),
        &[4]
    );
    assert_eq!(
        tlv::read_length(source, &options)
            .unwrap()
            .declared_value_length(),
        4
    );
    assert_eq!(
        tlv::frame_prefix(source, &options).unwrap().1.len(),
        source.len() - 2
    );
    assert_eq!(
        tlv::frame_exact(source, &options).err().unwrap().reason(),
        ErrorReason::TrailingData
    );
    for error in [
        tlv::read_identifier(&input, &options).err(),
        tlv::read_length(&input, &options).err(),
        tlv::read_header(&input, &options).err(),
        tlv::frame_prefix(&input, &options).err(),
        tlv::frame_exact(&input, &options).err(),
    ] {
        assert_eq!(error.unwrap().reason(), ErrorReason::InputLimit);
    }
    // Five header octets plus 0xFFFFFB content octets occupy exactly 16 MiB.
    let header = [4, 0x83, 0xff, 0xff, 0xfb];
    input[..header.len()].copy_from_slice(&header);
    let frame = tlv::frame_exact(&input[..tlv::MAX_INPUT_BYTES], &options).unwrap();
    assert_eq!(frame.consumed_bytes(), tlv::MAX_INPUT_BYTES);
    assert_eq!(
        frame.value_bytes().len(),
        tlv::MAX_INPUT_BYTES - header.len()
    );
}

#[test]
fn every_truncated_boundary_is_rejected_by_complete_frames() {
    let input = [0xdf, 0xff, 0x7f, 0x82, 0, 3, 1, 2, 3];
    for length in 0..input.len() {
        let prefix = &input[..length];
        let header = tlv::read_header(prefix, &ber());
        assert_eq!(header.is_ok(), length >= 6);
        assert!(tlv::frame_prefix(prefix, &ber()).is_err());
        assert!(tlv::frame_exact(prefix, &ber()).is_err());
    }
    assert_eq!(
        tlv::frame_exact(&input, &ber()).unwrap().value_bytes(),
        &[1, 2, 3]
    );
}

#[test]
fn complete_headers_do_not_convert_declared_lengths_to_platform_offsets() {
    let mut options = ber();
    options.limits = Some(Limits {
        length_octets: Some(8),
        input_bytes: None,
        identifier_octets: None,
    })
    .into();
    let input = [4, 0x88, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
    assert_eq!(
        tlv::read_header(&input, &options)
            .unwrap()
            .declared_value_length(),
        u64::MAX
    );
    assert_eq!(
        tlv::frame_prefix(&input, &options).err().unwrap().reason(),
        ErrorReason::TruncatedValue
    );
    assert_eq!(
        tlv::frame_exact(&input, &options).err().unwrap().reason(),
        ErrorReason::TruncatedValue
    );
}

#[test]
fn minimum_positive_limits_are_valid_and_unknown_policy_data_fails_closed() {
    let mut options = ber();
    options.limits = Some(Limits {
        input_bytes: Some(2),
        identifier_octets: Some(1),
        length_octets: Some(1),
    })
    .into();
    assert!(tlv::frame_exact(&[4, 0], &options).is_ok());
    assert!(buffa::DecodeOptions::new()
        .reject_unknown_fields()
        .decode_from_slice::<Options>(&[8, 1, 0x98, 6, 1])
        .is_err());
    assert!(buffa::DecodeOptions::new()
        .reject_unknown_fields()
        .decode_from_slice::<Options>(&[8, 1, 18, 3, 0x98, 6, 1])
        .is_err());
}

proptest! {
    #[test]
    fn arbitrary_input_never_produces_unbounded_receipts(input in proptest::collection::vec(any::<u8>(), 0..1024)) {
        for profile in [Profile::CODEC_TLV_PROFILE_ISO7816_DEFINITE_BER, Profile::CODEC_TLV_PROFILE_ASN1_DER_FRAMING] {
            let options = options(profile);
            if let Ok(header) = tlv::read_header(&input, &options) {
                prop_assert!(header.consumed_bytes() <= input.len());
                prop_assert_eq!(header.received_bytes(), &input[..header.consumed_bytes()]);
                prop_assert_eq!(header.supplied_remainder(), &input[header.consumed_bytes()..]);
                let identifier = tlv::read_identifier(header.identifier_bytes(), &options).unwrap();
                prop_assert_eq!(identifier.identifier_bytes(), header.identifier_bytes());
                prop_assert!(identifier.supplied_remainder().is_empty());
                let length = tlv::read_length(header.length_octets(), &options).unwrap();
                prop_assert!(length.supplied_remainder().is_empty());
                prop_assert_eq!(length.declared_value_length(), header.declared_value_length());
                for span in [&header.contract().identifier, &header.contract().length_octets, &header.contract().header_bytes] {
                    let span = span.as_option().unwrap();
                    prop_assert!(span.offset.checked_add(span.length).unwrap() <= u64::try_from(input.len()).unwrap());
                }
            }
            if let Ok((frame, rest)) = tlv::frame_prefix(&input, &options) {
                prop_assert_eq!(frame.received_bytes(), &input[..frame.consumed_bytes()]);
                prop_assert_eq!(rest, &input[frame.consumed_bytes()..]);
                prop_assert_eq!(u64::try_from(frame.value_bytes().len()).unwrap(), frame.header().declared_value_length());
                prop_assert_eq!(tlv::frame_exact(&input, &options).is_ok(), rest.is_empty());
            }
        }
    }

    #[test]
    fn ber_wire_identifiers_and_redundant_lengths_survive_without_normalization(
        groups in proptest::collection::vec(0u8..128, 0..8), last in 0u8..128,
        value in proptest::collection::vec(any::<u8>(), 0..128), suffix in proptest::collection::vec(any::<u8>(), 0..32),
        zero_octets in 0usize..7,
    ) {
        let mut identifier = vec![0x5f];
        identifier.extend(groups.iter().map(|byte| byte | 0x80));
        identifier.push(last);
        // Valid high-tag identifiers start with a nonzero base-128 group.
        if identifier[1] & 0x7f == 0 { identifier[1] |= 1; }
        let mut input = identifier.clone();
        let count = zero_octets.checked_add(1).unwrap();
        input.push(0x80 | u8::try_from(count).unwrap());
        input.extend(std::iter::repeat_n(0, zero_octets));
        input.push(u8::try_from(value.len()).unwrap());
        let header_end = input.len();
        input.extend_from_slice(&value);
        let frame_end = input.len();
        input.extend_from_slice(&suffix);
        let mut options = ber();
        options.limits = Some(Limits { identifier_octets: Some(32), length_octets: Some(8), input_bytes: None }).into();
        let header = tlv::read_header(&input, &options).unwrap();
        prop_assert_eq!(header.identifier_bytes(), identifier);
        prop_assert_eq!(header.consumed_bytes(), header_end);
        let (frame, rest) = tlv::frame_prefix(&input, &options).unwrap();
        prop_assert_eq!(frame.value_bytes(), value);
        prop_assert_eq!(frame.received_bytes(), &input[..frame_end]);
        prop_assert_eq!(rest, suffix);
    }
}
