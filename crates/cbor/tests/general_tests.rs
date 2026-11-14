// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(missing_docs, clippy::unwrap_used, clippy::expect_used)]

use codec_cbor::general::{
    encode, frame_exact, frame_prefix, read_header_prefix, validate_exact, validate_prefix,
    ArgumentWidth, CborError, Cursor, ErrorReason, Float, FloatPolicy, FloatWidth, LengthPolicy,
    Limits, MajorType, MapEntry, MapOrdering, Profile, Serialization, ValueRef, MAX_BYTES,
    MAX_DEPTH, MAX_ENTRIES, MAX_NODES, MAX_WORK,
};

fn bytes(hex: &str) -> Vec<u8> {
    hex.split_ascii_whitespace()
        .flat_map(|part| {
            let (pairs, remainder) = part.as_bytes().as_chunks::<2>();
            assert!(remainder.is_empty());
            pairs.iter()
        })
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn reason<T>(result: Result<T, CborError>, expected: ErrorReason) {
    assert_eq!(result.err().map(CborError::reason), Some(expected));
}

#[test]
fn independent_rfc_integer_vectors_cover_both_complete_argument_domains() {
    for (hex, negative, argument, value) in [
        ("00", false, 0, 0),
        ("17", false, 23, 23),
        ("1818", false, 24, 24),
        ("18ff", false, 255, 255),
        ("190100", false, 256, 256),
        ("19ffff", false, 65535, 65535),
        ("1a00010000", false, 65536, 65536),
        ("1affffffff", false, 4_294_967_295, 4_294_967_295),
        ("1b0000000100000000", false, 4_294_967_296, 4_294_967_296),
        (
            "1bffffffffffffffff",
            false,
            u64::MAX,
            18_446_744_073_709_551_615,
        ),
        ("20", true, 0, -1),
        ("37", true, 23, -24),
        ("3818", true, 24, -25),
        (
            "3bffffffffffffffff",
            true,
            u64::MAX,
            -18_446_744_073_709_551_616,
        ),
    ] {
        let input = bytes(hex);
        let item = validate_exact(&input, Profile::preferred(), Limits::default()).unwrap();
        let integer = item.frame().integer().unwrap();
        assert_eq!(integer.argument(), argument);
        assert_eq!(integer.is_negative(), negative);
        assert_eq!(integer.to_i128(), value);
        let encoded = encode(
            &if negative {
                ValueRef::Negative(argument)
            } else {
                ValueRef::Unsigned(argument)
            },
            Profile::preferred(),
            Limits::default(),
        )
        .unwrap();
        assert_eq!(encoded.as_bytes(), input);
    }
}

#[test]
fn received_tag_and_length_widths_remain_exact_and_borrowed() {
    let input = bytes("da00000018 590003 a10102");
    let frame = frame_exact(&input, Limits::default(), LengthPolicy::DefiniteOnly).unwrap();
    let tagged = frame.tagged_bytes().unwrap();
    assert_eq!(tagged.tag(), 24);
    assert_eq!(tagged.received_bytes(), input);
    assert_eq!(tagged.received_bytes().as_ptr(), input.as_ptr());
    assert_eq!(tagged.payload_bytes(), &[0xa1, 1, 2]);
    assert_eq!(tagged.payload_bytes().as_ptr(), input[8..].as_ptr());
    assert_eq!(frame.header().argument_width(), ArgumentWidth::Four);
    assert_eq!(frame.header().has_preferred_argument(), Some(false));
    tagged
        .validate_embedded(Profile::preferred(), Limits::default())
        .unwrap();
    validate_exact(&input, Profile::received(), Limits::default()).unwrap();
    reason(
        validate_exact(&input, Profile::preferred(), Limits::default()),
        ErrorReason::NonPreferredArgument,
    );
    let output = encode(
        &ValueRef::Received(&frame),
        Profile::received(),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(output.as_bytes(), input);
    reason(
        encode(
            &ValueRef::Received(&frame),
            Profile::preferred(),
            Limits::default(),
        ),
        ErrorReason::NonPreferredArgument,
    );
}

#[test]
fn framing_and_tag_semantics_are_distinct() {
    for (hex, embedded_error) in [
        ("d81840", ErrorReason::EmptyEmbeddedItem),
        ("d818420001", ErrorReason::TrailingData),
        ("d81841ff", ErrorReason::UnexpectedBreak),
    ] {
        let input = bytes(hex);
        let frame = frame_exact(&input, Limits::default(), LengthPolicy::DefiniteOnly).unwrap();
        validate_exact(&input, Profile::preferred(), Limits::default()).unwrap();
        reason(
            frame
                .tagged_bytes()
                .unwrap()
                .validate_embedded(Profile::received(), Limits::default()),
            embedded_error,
        );
    }
    let input = bytes("dbffffffffffffffff4100");
    let frame = frame_exact(&input, Limits::default(), LengthPolicy::DefiniteOnly).unwrap();
    let tag = frame.tagged_bytes().unwrap();
    assert_eq!(tag.tag(), u64::MAX);
    reason(
        tag.validate_embedded(Profile::preferred(), Limits::default()),
        ErrorReason::WrongTag,
    );
    reason(frame.as_bytes(), ErrorReason::WrongType);
    let input = bytes("d8185f4100ff");
    let frame = frame_exact(&input, Limits::default(), LengthPolicy::AllowIndefinite).unwrap();
    reason(frame.tagged_bytes(), ErrorReason::NonContiguousString);
}

#[test]
fn prefix_exact_and_cursor_consumption_are_explicit() {
    let input = bytes("820102 d8184100 03");
    let (first, rest) =
        frame_prefix(&input, Limits::default(), LengthPolicy::DefiniteOnly).unwrap();
    assert_eq!(first.received_bytes(), &[0x82, 1, 2]);
    assert_eq!(rest, &input[3..]);
    let (valid, rest) = validate_prefix(&input, Profile::preferred(), Limits::default()).unwrap();
    assert_eq!(valid.frame().received_bytes(), first.received_bytes());
    assert_eq!(rest, &input[3..]);
    reason(
        frame_exact(&input, Limits::default(), LengthPolicy::DefiniteOnly),
        ErrorReason::TrailingData,
    );
    reason(
        validate_exact(&input, Profile::preferred(), Limits::default()),
        ErrorReason::TrailingData,
    );
    let mut cursor = Cursor::new(&input, Limits::default(), LengthPolicy::DefiniteOnly).unwrap();
    assert!(cursor.consumed_bytes().is_empty());
    for expected in [&input[..3], &input[3..7], &input[7..]] {
        assert_eq!(cursor.next().unwrap().unwrap().received_bytes(), expected);
    }
    assert!(cursor.remaining_bytes().is_empty());
    assert_eq!(cursor.consumed_bytes(), input);
    assert!(cursor.next().is_none());
    let mut children = first.children().unwrap();
    assert_eq!(
        children
            .next()
            .unwrap()
            .unwrap()
            .integer()
            .unwrap()
            .to_i128(),
        1
    );
    assert_eq!(
        children
            .next()
            .unwrap()
            .unwrap()
            .integer()
            .unwrap()
            .to_i128(),
        2
    );
    assert!(children.next().is_none());
}

#[test]
fn cursor_errors_are_fused_and_limits_accumulate() {
    let mut cursor =
        Cursor::new(&[0, 0x18], Limits::default(), LengthPolicy::DefiniteOnly).unwrap();
    assert!(cursor.next().unwrap().is_ok());
    reason(cursor.next().unwrap(), ErrorReason::Truncated);
    assert_eq!(cursor.consumed_bytes(), &[0]);
    assert_eq!(cursor.remaining_bytes(), &[0x18]);
    assert!(cursor.next().is_none());
    let mut cursor = Cursor::new(
        &[0, 1],
        Limits::default().with_nodes(1).unwrap(),
        LengthPolicy::DefiniteOnly,
    )
    .unwrap();
    assert!(cursor.next().unwrap().is_ok());
    reason(cursor.next().unwrap(), ErrorReason::NodeLimit);
    assert!(
        Cursor::new(&[], Limits::default(), LengthPolicy::DefiniteOnly)
            .unwrap()
            .next()
            .is_none()
    );
    reason(
        frame_exact(&[], Limits::default(), LengthPolicy::DefiniteOnly),
        ErrorReason::EmptyInput,
    );
}

#[test]
fn unsorted_maps_are_valid_and_encoder_keeps_order() {
    let entries = [
        MapEntry::new(ValueRef::Text("z"), ValueRef::Unsigned(1)),
        MapEntry::new(ValueRef::Text("a"), ValueRef::Unsigned(2)),
    ];
    let expected = bytes("a2617a01616102");
    let output = encode(
        &ValueRef::Map(&entries),
        Profile::preferred(),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(output.as_bytes(), expected);
    validate_exact(&expected, Profile::preferred(), Limits::default()).unwrap();
    for ordering in [MapOrdering::LengthFirst, MapOrdering::Bytewise] {
        let profile = Profile::preferred().with_map_ordering(ordering);
        reason(
            validate_exact(&expected, profile, Limits::default()),
            ErrorReason::MapKeyOrder,
        );
        reason(
            encode(&ValueRef::Map(&entries), profile, Limits::default()),
            ErrorReason::MapKeyOrder,
        );
    }
    // Bytewise and length-first ordering are distinct, explicitly selected rules.
    let input = bytes("a2 40 00 1818 01");
    validate_exact(
        &input,
        Profile::preferred().with_map_ordering(MapOrdering::LengthFirst),
        Limits::default(),
    )
    .unwrap();
    reason(
        validate_exact(
            &input,
            Profile::preferred().with_map_ordering(MapOrdering::Bytewise),
            Limits::default(),
        ),
        ErrorReason::MapKeyOrder,
    );
}

#[test]
fn semantic_equivalent_keys_are_rejected_without_sorting() {
    for ordering in [
        MapOrdering::Received,
        MapOrdering::LengthFirst,
        MapOrdering::Bytewise,
    ] {
        reason(
            validate_exact(
                &[0xa2, 0, 0, 0, 1],
                Profile::preferred().with_map_ordering(ordering),
                Limits::default(),
            ),
            ErrorReason::DuplicateMapKey,
        );
    }
    for hex in [
        "a2 00 00 1800 01",               // Integer widths do not change integer identity.
        "a2 8100 00 811800 01",           // Ordered array keys compare recursively.
        "a2 6161 00 7f6161ff 01",         // Chunking does not change text identity.
        "a2 426162 00 5f41614162ff 01",   // Or byte-string identity.
        "a2 a201020304 00 a203040102 01", // Map keys are unordered pair sets.
        "a2 c100 00 c11800 01",           // Same tag and equivalent child.
        "a2 f90000 00 f98000 01",         // Both float zero signs are equivalent.
        "a2 f93c00 00 fa3f800000 01",     // Exact floats across widths.
        "a2 f97e00 00 fa7fc00000 01",     // Right-zero-extended NaN significands.
        "a2 f97e00 00 f9fe00 01",         // NaN sign does not change basic-model identity.
    ] {
        let input = bytes(hex);
        frame_exact(&input, Limits::default(), LengthPolicy::AllowIndefinite).unwrap();
        reason(
            validate_exact(&input, Profile::received(), Limits::default()),
            ErrorReason::DuplicateMapKey,
        );
    }
}

#[test]
fn compound_key_comparison_respects_the_maximum_nesting_boundary() {
    let mut input = vec![0xa2];
    for value in [0, 1] {
        input.extend(core::iter::repeat_n(0x81, MAX_DEPTH - 1));
        input.push(0);
        input.push(value);
    }
    reason(
        validate_exact(&input, Profile::preferred(), Limits::default()),
        ErrorReason::DuplicateMapKey,
    );
    reason(
        validate_exact(
            &input,
            Profile::preferred(),
            Limits::default().with_work(128).unwrap(),
        ),
        ErrorReason::WorkLimit,
    );
}

#[test]
fn keys_with_different_basic_types_or_nan_payloads_remain_distinct() {
    for hex in [
        "a2 00 00 f90000 01",
        "a2 4161 00 6161 01",
        "a2 c100 00 00 01",
        "a2 c100 00 c200 01",
        "a2 f97e00 00 f97e01 01",
        "a2 820001 00 820100 01",
    ] {
        validate_exact(&bytes(hex), Profile::received(), Limits::default()).unwrap();
    }
}

#[test]
fn indefinite_containers_and_chunks_have_precise_framing() {
    for hex in [
        "9fff",
        "bfff",
        "5fff",
        "7fff",
        "9f018202039f0405ffff",
        "bf61610161629f0203ffff",
        "5f42010243030405ff",
        "7f657374726561646d696e67ff",
    ] {
        let input = bytes(hex);
        validate_exact(&input, Profile::received(), Limits::default()).unwrap();
        reason(
            frame_exact(&input, Limits::default(), LengthPolicy::DefiniteOnly),
            ErrorReason::IndefiniteNotAllowed,
        );
        reason(
            validate_exact(&input, Profile::preferred(), Limits::default()),
            ErrorReason::IndefiniteNotAllowed,
        );
    }
    let input = bytes("5f42010243030405ff");
    let frame = frame_exact(&input, Limits::default(), LengthPolicy::AllowIndefinite).unwrap();
    let chunks: Vec<&[u8]> = frame.string_chunks().unwrap().map(Result::unwrap).collect();
    assert_eq!(chunks, [&[1, 2][..], &[3, 4, 5][..]]);
    reason(frame.as_bytes(), ErrorReason::NonContiguousString);
    for (hex, expected) in [
        ("5f60ff", ErrorReason::InvalidChunk),
        ("7f40ff", ErrorReason::InvalidChunk),
        ("5f5fffff", ErrorReason::InvalidChunk),
        ("bf00ff", ErrorReason::UnexpectedBreak),
        ("9f", ErrorReason::Truncated),
        ("5f4100", ErrorReason::Truncated),
        ("81ff", ErrorReason::UnexpectedBreak),
    ] {
        reason(
            frame_exact(
                &bytes(hex),
                Limits::default(),
                LengthPolicy::AllowIndefinite,
            ),
            expected,
        );
    }
}

#[test]
fn utf8_is_basic_validity_not_structural_framing() {
    for hex in ["61ff", "62c080", "63eda080", "7f61c361a9ff"] {
        let input = bytes(hex);
        frame_exact(&input, Limits::default(), LengthPolicy::AllowIndefinite).unwrap();
        reason(
            validate_exact(&input, Profile::received(), Limits::default()),
            ErrorReason::InvalidUtf8,
        );
    }
    let input = bytes("62c3a9");
    let item = validate_exact(&input, Profile::preferred(), Limits::default()).unwrap();
    assert_eq!(item.frame().as_text().unwrap(), "é");
    reason(
        frame_exact(&[0x61, 0xff], Limits::default(), LengthPolicy::DefiniteOnly)
            .unwrap()
            .as_text(),
        ErrorReason::InvalidUtf8,
    );
}

#[test]
fn reserved_and_truncated_headers_reject_for_every_major_type() {
    for major in 0_u8..8 {
        for info in 28..=30 {
            reason(
                read_header_prefix(&[major << 5 | info]),
                ErrorReason::ReservedAdditionalInformation,
            );
        }
        for (info, length) in [(24, 2), (25, 3), (26, 5), (27, 9)] {
            let mut input = vec![0; length];
            input[0] = major << 5 | info;
            for length in 1..length {
                reason(read_header_prefix(&input[..length]), ErrorReason::Truncated);
            }
        }
    }
    for initial in [0x1f, 0x3f, 0xdf] {
        reason(
            read_header_prefix(&[initial]),
            ErrorReason::InvalidIndefiniteArgument,
        );
    }
    reason(
        frame_exact(&[0xff], Limits::default(), LengthPolicy::AllowIndefinite),
        ErrorReason::UnexpectedBreak,
    );
    for code in 0..32 {
        reason(
            read_header_prefix(&[0xf8, code]),
            ErrorReason::InvalidSimpleValue,
        );
    }
    for hex in ["4100", "6100", "8100", "a10000", "d81800"] {
        let input = bytes(hex);
        for length in 1..input.len() {
            assert!(frame_exact(
                &input[..length],
                Limits::default(),
                LengthPolicy::AllowIndefinite
            )
            .is_err());
        }
    }
}

#[test]
fn header_observation_does_not_claim_a_complete_payload() {
    let input = bytes("5bffffffffffffffff");
    let header = read_header_prefix(&input).unwrap();
    assert_eq!(header.major_type(), MajorType::Bytes);
    assert_eq!(header.argument(), Some(u64::MAX));
    assert_eq!(header.has_preferred_argument(), Some(true));
    assert!(frame_exact(
        header.received_bytes(),
        Limits::default(),
        LengthPolicy::DefiniteOnly
    )
    .is_err());
    let header = read_header_prefix(&[0xfb, 0, 0, 0, 0, 0, 0, 0, 0]).unwrap();
    assert_eq!(header.argument(), None);
    assert_eq!(header.has_preferred_argument(), None);
    assert!(header.is_float());
}

#[test]
fn float_known_vectors_and_selected_acceptance_policies() {
    for (hex, bits) in [
        ("f90000", 0.0_f64.to_bits()),
        ("f98000", (-0.0_f64).to_bits()),
        ("f93c00", 1.0_f64.to_bits()),
        ("f93e00", 1.5_f64.to_bits()),
        ("f97bff", 65504.0_f64.to_bits()),
        ("f90001", (2.0_f64.powi(-24)).to_bits()),
        ("fa47c35000", 100000.0_f64.to_bits()),
        ("fb3ff199999999999a", 1.1_f64.to_bits()),
        ("f97c00", f64::INFINITY.to_bits()),
        ("f9fc00", f64::NEG_INFINITY.to_bits()),
    ] {
        let input = bytes(hex);
        let item = validate_exact(&input, Profile::preferred(), Limits::default()).unwrap();
        assert_eq!(item.frame().float().unwrap().to_f64().to_bits(), bits);
        reason(
            validate_exact(
                &input,
                Profile::received().with_floats(FloatPolicy::Reject),
                Limits::default(),
            ),
            ErrorReason::FloatNotAllowed,
        );
        if !f64::from_bits(bits).is_finite() {
            reason(
                validate_exact(
                    &input,
                    Profile::received().with_floats(FloatPolicy::FiniteOnly),
                    Limits::default(),
                ),
                ErrorReason::NonFiniteFloat,
            );
        }
    }
    for hex in [
        "fa3f800000",
        "fb3ff0000000000000",
        "fa80000000",
        "fb8000000000000000",
        "fa7f800000",
        "fb7ff0000000000000",
        "fa7fc00000",
        "fb7ff8000000000000",
    ] {
        let input = bytes(hex);
        validate_exact(&input, Profile::received(), Limits::default()).unwrap();
        reason(
            validate_exact(&input, Profile::preferred(), Limits::default()),
            ErrorReason::NonPreferredFloat,
        );
    }
    // A non-zero low payload bit prevents shortening this NaN. Preferred CBOR
    // preserves its significand rather than replacing it with a canonical NaN.
    validate_exact(
        &bytes("fb7ff8000000000001"),
        Profile::preferred(),
        Limits::default(),
    )
    .unwrap();
}

#[test]
fn every_binary16_pattern_preserves_sign_and_nan_significand() {
    for bits in u16::MIN..=u16::MAX {
        let value = Float::binary16(bits);
        let encoded = encode(
            &ValueRef::Float(&value),
            Profile::preferred(),
            Limits::default(),
        )
        .unwrap();
        let expected = bits.to_be_bytes();
        assert_eq!(encoded.as_bytes(), &[0xf9, expected[0], expected[1]]);
        let item =
            validate_exact(encoded.as_bytes(), Profile::preferred(), Limits::default()).unwrap();
        assert_eq!(item.frame().float().unwrap().bits(), u64::from(bits));
        assert_eq!(item.frame().float().unwrap().width(), FloatWidth::Binary16);
    }
}

#[test]
fn encode_general_values_matches_independent_bytes() {
    assert!(
        validate_exact(&[0xf5], Profile::preferred(), Limits::default())
            .unwrap()
            .frame()
            .as_bool()
            .unwrap()
    );
    assert!(
        !validate_exact(&[0xf4], Profile::preferred(), Limits::default())
            .unwrap()
            .frame()
            .as_bool()
            .unwrap()
    );
    reason(
        validate_exact(&[0xf6], Profile::preferred(), Limits::default())
            .unwrap()
            .frame()
            .as_bool(),
        ErrorReason::WrongType,
    );
    let values = [
        ValueRef::Bool(false),
        ValueRef::Bool(true),
        ValueRef::Null,
        ValueRef::Undefined,
        ValueRef::Simple(0),
        ValueRef::Simple(255),
        ValueRef::Text("IETF"),
        ValueRef::Bytes(&[1, 2, 3]),
    ];
    let output = encode(
        &ValueRef::Array(&values),
        Profile::preferred(),
        Limits::default(),
    )
    .unwrap();
    assert_eq!(
        output.as_bytes(),
        bytes("88f4f5f6f7e0f8ff644945544643010203")
    );
    let payload = ValueRef::Bytes(&[0xa1, 1, 2]);
    assert_eq!(
        encode(
            &ValueRef::Tag(24, &payload),
            Profile::preferred(),
            Limits::default()
        )
        .unwrap()
        .as_bytes(),
        bytes("d81843a10102")
    );
    for code in 24..32 {
        reason(
            encode(
                &ValueRef::Simple(code),
                Profile::received(),
                Limits::default(),
            ),
            ErrorReason::InvalidSimpleValue,
        );
    }
    let duplicate = [
        MapEntry::new(ValueRef::Unsigned(1), ValueRef::Null),
        MapEntry::new(ValueRef::Unsigned(1), ValueRef::Bool(false)),
    ];
    reason(
        encode(
            &ValueRef::Map(&duplicate),
            Profile::preferred(),
            Limits::default(),
        ),
        ErrorReason::DuplicateMapKey,
    );
}

#[path = "general/resource_tests.rs"]
mod resource_tests;
