// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Acceptance and limit boundaries of the COSE container preflight.
#![allow(clippy::unwrap_used, reason = "fixed conformance fixtures")]

use codec_cbor::general::{
    encode, frame_exact, validate_exact, validate_prefix, CborError, ErrorReason, Float,
    LengthPolicy, Limits, MapKeyEquality, MapOrdering, NanPolicy, Profile, ValueRef,
};

fn cose() -> Profile {
    Profile::preferred()
        .with_map_key_equality(MapKeyEquality::EncodedScalars)
        .unwrap()
        .with_nan_policy(NanPolicy::Binary16Only)
        .unwrap()
}

fn assert_reason<T>(result: Result<T, CborError>, reason: ErrorReason) {
    assert_eq!(result.err().map(CborError::reason), Some(reason));
}

#[test]
fn signed_zeros_and_nan_signs_are_distinct_under_cose_recursively() {
    for map in [
        &[0xa2, 0xf9, 0, 0, 1, 0xf9, 0x80, 0, 2][..],
        &[
            0xa2, 0x81, 0xd2, 0xf9, 0, 0, 1, 0x81, 0xd2, 0xf9, 0x80, 0, 2,
        ],
        &[0xa2, 0xa1, 0, 0xf9, 0, 0, 1, 0xa1, 0, 0xf9, 0x80, 0, 2],
        &[0xa2, 0xf9, 0x7e, 1, 1, 0xf9, 0xfe, 1, 2],
    ] {
        let limits = Limits::default();
        let borrowed = validate_exact(map, cose(), limits).unwrap();
        assert_eq!(borrowed.frame().received_bytes().as_ptr(), map.as_ptr());
        assert_eq!(borrowed.frame().received_bytes(), map);
        assert_eq!(
            borrowed.profile().map_key_equality(),
            Some(MapKeyEquality::EncodedScalars)
        );
        assert_reason(
            validate_exact(map, Profile::preferred(), limits),
            ErrorReason::DuplicateMapKey,
        );
    }
    let unsorted = [0xa2, 2, 0, 1, 0, 0xf6];
    let (item, rest) = validate_prefix(&unsorted, cose(), Limits::default()).unwrap();
    assert_eq!(item.frame().received_bytes(), &unsorted[..5]);
    assert_eq!(rest, [0xf6]);
    assert_reason(
        validate_exact(&unsorted, cose(), Limits::default()),
        ErrorReason::TrailingData,
    );
}

#[test]
fn duplicate_keys_and_structural_errors_keep_their_preflight_order() {
    for duplicate in [
        &[0xa2, 0x01, 0x00, 0x01][..], // the duplicate precedes a missing value
        &[0xa2, 0xf9, 0x80, 0, 0, 0xf9, 0x80, 0, 0],
        &[0xa2, 0xa2, 0x01, 0, 0x02, 1, 0, 0xa2, 0x02, 1, 0x01, 0, 0],
    ] {
        assert_reason(
            validate_exact(duplicate, cose(), Limits::default()),
            ErrorReason::DuplicateMapKey,
        );
    }
    for (bytes, reason) in [
        (&[0x18, 0][..], ErrorReason::NonPreferredArgument),
        (&[0x9f, 0xff], ErrorReason::IndefiniteNotAllowed),
        (&[0xff], ErrorReason::IndefiniteNotAllowed),
        (&[0xf8, 31], ErrorReason::InvalidSimpleValue),
        (&[0x61, 0xff], ErrorReason::InvalidUtf8),
        (&[0x81, 0x18], ErrorReason::Truncated),
    ] {
        assert_reason(validate_exact(bytes, cose(), Limits::default()), reason);
    }
}

#[test]
fn preferred_floats_match_cose_nan_and_width_rules() {
    let limits = Limits::default();
    for value in [
        &[0xfa, 0x7f, 0xc0, 0, 1][..],
        &[0xfb, 0xff, 0xf8, 0, 0, 0, 0, 0, 1],
    ] {
        assert!(validate_exact(value, Profile::preferred(), limits).is_ok());
        assert_reason(
            validate_exact(value, cose(), limits),
            ErrorReason::NonPreferredFloat,
        );
        let frame = frame_exact(value, limits, LengthPolicy::DefiniteOnly).unwrap();
        assert_reason(
            encode(&ValueRef::Received(&frame), cose(), limits),
            ErrorReason::NonPreferredFloat,
        );
    }
    for value in [[0xf9, 0x7c, 1], [0xf9, 0xfe, 1], [0xf9, 0x7c, 0]] {
        assert!(validate_exact(&value, cose(), limits).is_ok());
    }
    assert_reason(
        encode(
            &ValueRef::Float(&Float::binary32(0x7fc0_0000)),
            cose(),
            limits,
        ),
        ErrorReason::NonPreferredFloat,
    );
    assert_reason(
        validate_exact(&[0xfa, 0x3f, 0x80, 0, 0], cose(), limits),
        ErrorReason::NonPreferredFloat,
    );
    assert!(validate_exact(&[0xfa, 0x3f, 0x80, 0, 1], cose(), limits).is_ok());
    assert_reason(
        Profile::received().with_nan_policy(NanPolicy::CODEC_GENERAL_CBOR_NAN_POLICY_UNSPECIFIED),
        ErrorReason::UnsupportedPolicy,
    );
}

#[test]
fn nan_width_and_scalar_key_equality_can_be_selected_independently() {
    let limits = Limits::default();
    let encoded = Profile::received()
        .with_map_key_equality(MapKeyEquality::EncodedScalars)
        .unwrap();
    let nan = Profile::received()
        .with_nan_policy(NanPolicy::Binary16Only)
        .unwrap();
    let zeros = [0xa2, 0xf9, 0, 0, 1, 0xf9, 0x80, 0, 2];
    let wide_nan = [0xfa, 0x7f, 0xc0, 0, 1];
    assert!(validate_exact(&zeros, encoded, limits).is_ok());
    assert!(validate_exact(&wide_nan, encoded, limits).is_ok());
    assert_reason(
        validate_exact(&zeros, nan, limits),
        ErrorReason::DuplicateMapKey,
    );
    assert_reason(
        validate_exact(&wide_nan, nan, limits),
        ErrorReason::NonPreferredFloat,
    );
    assert!(validate_exact(&[0xa2, 0, 0, 0x18, 0, 0], encoded, limits).is_ok());
    assert_reason(
        validate_exact(
            &[0xa2, 2, 0, 1, 0],
            encoded.with_map_ordering(MapOrdering::Bytewise),
            limits,
        ),
        ErrorReason::MapKeyOrder,
    );

    // Compound framing is ignored even when one side uses indefinite length.
    for duplicate in [
        &[0xa2, 0x81, 1, 0, 0x9f, 1, 0xff, 0][..],
        &[0xa2, 0xa1, 1, 2, 0, 0xbf, 1, 2, 0xff, 0],
        &[0xa2, 0xc0, 1, 0, 0xd8, 0, 1, 0],
    ] {
        assert_reason(
            validate_exact(duplicate, encoded, limits),
            ErrorReason::DuplicateMapKey,
        );
    }
    assert!(validate_exact(&[0xa2, 0xbf, 0xff, 0, 0xa1, 1, 2, 0], encoded, limits).is_ok());
}

#[test]
fn cose_depth_nodes_entries_and_input_are_bounded() {
    let limits = Limits::default()
        .with_depth(32)
        .unwrap()
        .with_entries(1_024)
        .unwrap();
    let mut nested = vec![0xc0; 32];
    nested.push(0x80); // An empty container is itself a node at depth 32.
    assert!(validate_exact(&nested, cose(), limits).is_ok());
    nested[32] = 0x81;
    nested.push(0);
    assert_reason(
        validate_exact(&nested, cose(), limits),
        ErrorReason::DepthLimit,
    );

    let mut entries = vec![0x99, 0x04, 0];
    entries.resize(1_027, 0);
    assert!(validate_exact(&entries, cose(), limits).is_ok());
    entries[2] = 1;
    entries.push(0);
    assert_reason(
        validate_exact(&entries, cose(), limits),
        ErrorReason::EntryLimit,
    );

    let full_array = [&[0x99, 0x04, 0][..], &[0; 1_024]].concat();
    let mut nodes = vec![0x98, 64];
    for _ in 0..63 {
        nodes.extend_from_slice(&full_array);
    }
    nodes.extend_from_slice(&[0x99, 0x03, 0xbf]); // 959 children plus 64 array nodes and the root
    nodes.extend_from_slice(&[0; 959]);
    assert!(validate_exact(&nodes, cose(), limits).is_ok());
    let last_count = nodes.len().checked_sub(960).unwrap();
    nodes[last_count] = 0xc0;
    nodes.push(0);
    assert_reason(
        validate_exact(&nodes, cose(), limits),
        ErrorReason::NodeLimit,
    );

    let mut input = vec![0x5a, 0, 0x0f, 0xff, 0xfb];
    input.resize(1_048_576, 0);
    assert!(validate_exact(&input, cose(), limits).is_ok());
    input.push(0);
    assert_reason(
        validate_exact(&input, cose(), limits),
        ErrorReason::InputLimit,
    );
    assert_reason(
        validate_exact(&[0x81, 0], cose(), limits.with_nodes(1).unwrap()),
        ErrorReason::NodeLimit,
    );
}

#[test]
fn encoded_key_profile_encodes_the_same_depth_boundary_it_validates() {
    let profile = cose();
    let limits = Limits::default().with_depth(0).unwrap();
    let empty_array = [0x80];
    let frame = frame_exact(&empty_array, Limits::default(), LengthPolicy::DefiniteOnly).unwrap();
    for (value, expected) in [
        (ValueRef::Array(&[]), [0x80]),
        (ValueRef::Map(&[]), [0xa0]),
        (ValueRef::Received(&frame), [0x80]),
    ] {
        let output = encode(&value, profile, limits).unwrap();
        assert_eq!(output.as_bytes(), expected);
        validate_exact(output.as_bytes(), profile, limits).unwrap();
    }
    assert_reason(
        encode(&ValueRef::Array(&[ValueRef::Null]), profile, limits),
        ErrorReason::DepthLimit,
    );
    let frame = frame_exact(&[0x81, 0xf6], Limits::default(), LengthPolicy::DefiniteOnly).unwrap();
    assert_reason(
        encode(&ValueRef::Received(&frame), profile, limits),
        ErrorReason::DepthLimit,
    );
}

#[test]
fn oversized_string_declarations_have_architecture_independent_reasons() {
    let limits = Limits::default();
    for prefix in [0x5b, 0x7b] {
        let mut string = [prefix, 0, 0, 0, 1, 0, 0, 0, 0]; // 2^32 bytes
        assert_reason(
            validate_exact(&string, cose(), limits),
            ErrorReason::Truncated,
        );
        string[1..].fill(0xff); // The CBOR length plus its header overflows u64.
        assert_reason(
            validate_exact(&string, cose(), limits),
            ErrorReason::InputLimit,
        );
    }
}

#[test]
fn cose_has_independent_scan_and_comparison_budgets() {
    // A large previous value exhausts scan nodes while the short keys stay
    // well below the comparison-byte ceiling.
    let mut scans = vec![0xb8, 0xf8, 0, 0x84];
    for _ in 0..4 {
        scans.extend_from_slice(&[0x99, 0x04, 0]);
        scans.extend_from_slice(&[0; 1_024]);
    }
    for key in 1_u8..248 {
        if key <= 23 {
            scans.push(key);
        } else {
            scans.extend_from_slice(&[0x18, key]);
        }
        scans.push(0);
    }
    let limit = Limits::default();
    // Use a shorter large value for the accepted boundary fixture.
    let final_member = 4 + 3 * 1_027;
    let mut within = scans.clone();
    within[3] = 0x83;
    within.drain(final_member..final_member + 1_027);
    assert!(validate_exact(&within, cose(), limit).is_ok());
    assert_reason(
        validate_exact(&scans, cose(), limit),
        ErrorReason::WorkLimit,
    );

    // Three pair comparisons between 180,000-byte keys exceed the
    // comparison-byte ceiling while remaining within the input limit.
    let mut comparisons = vec![0xa2];
    for key in 0..2 {
        comparisons.extend_from_slice(&[0x5a, 0, 0x02, 0xbf, 0x20]);
        comparisons.extend_from_slice(&vec![key; 180_000]);
        comparisons.push(0);
    }
    assert!(validate_exact(&comparisons, cose(), Limits::default()).is_ok());
    comparisons[0] = 0xa3;
    comparisons.extend_from_slice(&[0x5a, 0, 0x02, 0xbf, 0x20]);
    comparisons.extend_from_slice(&vec![2; 180_000]);
    comparisons.push(0);
    assert_reason(
        validate_exact(&comparisons, cose(), Limits::default()),
        ErrorReason::WorkLimit,
    );
    assert_reason(
        validate_exact(&[0], cose(), Limits::default().with_work(1).unwrap()),
        ErrorReason::WorkLimit,
    );
}
