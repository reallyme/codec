// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Preserve borrowed bytes and bounded extents for arbitrary framing input.
#![no_main]

use libfuzzer_sys::fuzz_target;
use reallyme_codec::tlv::{self, Limits, Options, Profile};

fuzz_target!(|input: &[u8]| {
    for profile in [
        Profile::CODEC_TLV_PROFILE_ISO7816_DEFINITE_BER,
        Profile::CODEC_TLV_PROFILE_ASN1_DER_FRAMING,
    ] {
        let options = Options {
            profile: profile.into(),
            limits: Some(Limits {
                identifier_octets: Some(32),
                length_octets: Some(8),
                ..Default::default()
            })
            .into(),
            ..Default::default()
        };
        if let Ok(header) = tlv::read_header(input, &options) {
            assert!(header.consumed_bytes() <= input.len());
            assert_eq!(header.received_bytes(), &input[..header.consumed_bytes()]);
            assert_eq!(
                header.supplied_remainder(),
                &input[header.consumed_bytes()..]
            );
        }
        if let Ok(identifier) = tlv::read_identifier(input, &options) {
            assert_eq!(
                identifier.identifier_bytes(),
                &input[..identifier.consumed_bytes()]
            );
            assert_eq!(
                identifier.supplied_remainder(),
                &input[identifier.consumed_bytes()..]
            );
        }
        if let Ok(length) = tlv::read_length(input, &options) {
            assert_eq!(length.length_octets(), &input[..length.consumed_bytes()]);
            assert_eq!(
                length.supplied_remainder(),
                &input[length.consumed_bytes()..]
            );
        }
        if let Ok((frame, remaining)) = tlv::frame_prefix(input, &options) {
            assert_eq!(frame.received_bytes(), &input[..frame.consumed_bytes()]);
            assert_eq!(remaining, &input[frame.consumed_bytes()..]);
            assert_eq!(
                u64::try_from(frame.value_bytes().len()).ok(),
                Some(frame.header().declared_value_length())
            );
            assert_eq!(
                tlv::frame_exact(input, &options).is_ok(),
                remaining.is_empty()
            );
        }
    }
});
