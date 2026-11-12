// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Validated navigation, header float observations, and specific rejection
//! reasons for oversized lengths and empty embedded items.

#![allow(missing_docs, clippy::unwrap_used)]

use codec_cbor::general::{
    frame_exact, read_header_prefix, validate_exact, CborError, ErrorReason, LengthPolicy, Limits,
    Profile,
};

fn reason<T>(result: Result<T, CborError>) -> Option<ErrorReason> {
    result.err().map(CborError::reason)
}

#[test]
fn validated_children_keep_the_validated_receipt_and_exact_spans() {
    let input = [
        0xa2, 0x01, 0x82, 0x02, 0x03, 0x61, 0x61, 0xd8, 0x18, 0x41, 0x00,
    ];
    let item = validate_exact(&input, Profile::preferred(), Limits::default()).unwrap();
    let children: Vec<_> = item.children().unwrap().map(Result::unwrap).collect();
    let spans: Vec<&[u8]> = children
        .iter()
        .map(|child| child.frame().received_bytes())
        .collect();
    assert_eq!(
        spans,
        [&input[1..2], &input[2..5], &input[5..7], &input[7..]]
    );
    for child in &children {
        assert_eq!(child.profile(), Profile::preferred());
    }
    let nested: Vec<&[u8]> = children[1]
        .children()
        .unwrap()
        .map(|child| child.unwrap().frame().received_bytes())
        .collect();
    assert_eq!(nested, [&input[3..4], &input[4..5]]);
    let tagged: Vec<_> = children[3]
        .children()
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(tagged.len(), 1);
    assert_eq!(tagged[0].frame().received_bytes(), &input[9..]);
    assert!(children[0].children().is_err());
}

#[test]
fn header_observations_report_float_preference() {
    for (bytes, preferred) in [
        (&[0xf9, 0x3c, 0x00][..], Some(true)),
        (&[0xfa, 0x3f, 0x80, 0x00, 0x00], Some(false)),
        (&[0xfa, 0x7f, 0xc0, 0x00, 0x00], Some(false)),
        (&[0xfa, 0x7f, 0x80, 0x00, 0x01], Some(true)),
        (
            &[0xfb, 0x3f, 0xf1, 0x99, 0x99, 0x99, 0x99, 0x99, 0x9a],
            Some(true),
        ),
        (&[0x18, 0x01], None),
    ] {
        let header = read_header_prefix(bytes).unwrap();
        assert_eq!(header.has_preferred_float().unwrap(), preferred);
    }
}

#[test]
fn oversized_string_lengths_report_truncation() {
    for input in [
        &[0x5b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff][..],
        &[0x7b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff],
        &[0x5a, 0x7f, 0xff, 0xff, 0xff],
        &[0x42, 0x01],
    ] {
        assert_eq!(
            reason(frame_exact(
                input,
                Limits::default(),
                LengthPolicy::DefiniteOnly
            )),
            Some(ErrorReason::Truncated)
        );
        assert_eq!(
            reason(validate_exact(
                input,
                Profile::received(),
                Limits::default()
            )),
            Some(ErrorReason::Truncated)
        );
    }
}

#[test]
fn empty_tag_24_payloads_frame_but_are_not_embedded_items() {
    let input = [0xd8, 0x18, 0x40];
    let item = validate_exact(&input, Profile::preferred(), Limits::default()).unwrap();
    let tagged = item.frame().tagged_bytes().unwrap();
    assert!(tagged.payload_bytes().is_empty());
    assert_eq!(
        reason(tagged.validate_embedded(Profile::preferred(), Limits::default())),
        Some(ErrorReason::EmptyEmbeddedItem)
    );
}
