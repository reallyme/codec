// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use proptest::prelude::{any, proptest};

use super::{
    bytes, encode, frame_exact, frame_prefix, read_header_prefix, reason, validate_exact,
    ArgumentWidth, ErrorReason, Float, FloatPolicy, LengthPolicy, Limits, MapEntry, MapOrdering,
    Profile, Serialization, ValueRef, MAX_BYTES, MAX_DEPTH, MAX_ENTRIES, MAX_NODES, MAX_WORK,
};

#[test]
fn all_resource_limits_are_caller_lowerable_and_enforced() {
    for limits in [
        Limits::default().with_input_bytes(MAX_BYTES + 1),
        Limits::default().with_output_bytes(MAX_BYTES + 1),
        Limits::default().with_depth(MAX_DEPTH + 1),
        Limits::default().with_nodes(MAX_NODES + 1),
        Limits::default().with_entries(MAX_ENTRIES + 1),
        Limits::default().with_byte_string_bytes(MAX_BYTES + 1),
        Limits::default().with_text_bytes(MAX_BYTES + 1),
        Limits::default().with_work(MAX_WORK + 1),
    ] {
        reason(limits, ErrorReason::InvalidLimit);
    }
    for (hex, limits, expected) in [
        (
            "00",
            Limits::default().with_input_bytes(0).unwrap(),
            ErrorReason::InputLimit,
        ),
        (
            "00",
            Limits::default().with_nodes(0).unwrap(),
            ErrorReason::NodeLimit,
        ),
        (
            "80",
            Limits::default().with_depth(0).unwrap(),
            ErrorReason::DepthLimit,
        ),
        (
            "8100",
            Limits::default().with_entries(0).unwrap(),
            ErrorReason::EntryLimit,
        ),
        (
            "8241004100",
            Limits::default().with_byte_string_bytes(1).unwrap(),
            ErrorReason::ByteStringLimit,
        ),
        (
            "8261616162",
            Limits::default().with_text_bytes(1).unwrap(),
            ErrorReason::TextLimit,
        ),
        (
            "00",
            Limits::default().with_work(0).unwrap(),
            ErrorReason::WorkLimit,
        ),
        (
            "9f0001ff",
            Limits::default().with_entries(1).unwrap(),
            ErrorReason::EntryLimit,
        ),
        (
            "5f4000ff",
            Limits::default().with_entries(0).unwrap(),
            ErrorReason::EntryLimit,
        ),
    ] {
        reason(
            validate_exact(&bytes(hex), Profile::received(), limits),
            expected,
        );
    }
    reason(
        encode(
            &ValueRef::Unsigned(0),
            Profile::preferred(),
            Limits::default().with_output_bytes(0).unwrap(),
        ),
        ErrorReason::OutputLimit,
    );
    reason(
        encode(
            &ValueRef::Bytes(&[0, 1]),
            Profile::preferred(),
            Limits::default().with_byte_string_bytes(1).unwrap(),
        ),
        ErrorReason::ByteStringLimit,
    );
    reason(
        encode(
            &ValueRef::Array(&[ValueRef::Null]),
            Profile::preferred(),
            Limits::default().with_nodes(1).unwrap(),
        ),
        ErrorReason::NodeLimit,
    );
    reason(
        Profile::new(
            Serialization::Preferred,
            LengthPolicy::AllowIndefinite,
            FloatPolicy::All,
            MapOrdering::Received,
        ),
        ErrorReason::UnsupportedPolicy,
    );
}

#[test]
fn maximum_input_depth_nodes_and_huge_declarations_are_bounded() {
    let mut input = vec![0x81; MAX_DEPTH];
    input.push(0);
    validate_exact(&input, Profile::preferred(), Limits::default()).unwrap();
    input.insert(0, 0x81);
    reason(
        validate_exact(&input, Profile::preferred(), Limits::default()),
        ErrorReason::DepthLimit,
    );
    reason(
        validate_exact(
            &bytes("9bffffffffffffffff"),
            Profile::received(),
            Limits::default(),
        ),
        ErrorReason::EntryLimit,
    );
    assert!(validate_exact(
        &bytes("5bffffffffffffffff"),
        Profile::received(),
        Limits::default()
    )
    .is_err());
    reason(
        validate_exact(&bytes("99ffff"), Profile::received(), Limits::default()),
        ErrorReason::EntryLimit,
    );
    reason(
        validate_exact(&bytes("a101"), Profile::received(), Limits::default()),
        ErrorReason::ContainerExceedsInput,
    );
    let large = vec![0; MAX_BYTES + 1];
    reason(
        frame_prefix(&large, Limits::default(), LengthPolicy::DefiniteOnly),
        ErrorReason::InputLimit,
    );
    let mut exact = vec![0x5a, 0x00, 0x0f, 0xff, 0xfb];
    exact.resize(MAX_BYTES, 0);
    let frame = frame_exact(&exact, Limits::default(), LengthPolicy::DefiniteOnly).unwrap();
    assert_eq!(frame.as_bytes().unwrap().len(), MAX_BYTES - 5);
}

#[test]
fn exact_node_and_entry_ceilings_are_reachable_without_allocating_a_tree() {
    let mut input = vec![0x90]; // Sixteen child arrays.
    for index in 0..16 {
        let count = if index == 15 { 4094_usize } else { 4095 };
        input.extend_from_slice(if index == 15 {
            &[0x99, 0x0f, 0xfe]
        } else {
            &[0x99, 0x0f, 0xff]
        });
        input.extend(core::iter::repeat_n(0, count));
    }
    // 1 root + 16 arrays + 65,519 scalars = 65,536 nodes.
    validate_exact(&input, Profile::preferred(), Limits::default()).unwrap();
    reason(
        validate_exact(
            &input,
            Profile::preferred(),
            Limits::default().with_nodes(MAX_NODES - 1).unwrap(),
        ),
        ErrorReason::NodeLimit,
    );
    let mut entries = vec![0x99, 0x10, 0x00];
    entries.extend(core::iter::repeat_n(0, MAX_ENTRIES));
    validate_exact(&entries, Profile::preferred(), Limits::default()).unwrap();
    reason(
        validate_exact(
            &entries,
            Profile::preferred(),
            Limits::default().with_entries(MAX_ENTRIES - 1).unwrap(),
        ),
        ErrorReason::EntryLimit,
    );
}

#[test]
fn argument_width_transitions_are_observable_across_major_types() {
    for major in 0_u8..7 {
        for (hex, value, width) in [
            ("17", 23, ArgumentWidth::Inline),
            ("1818", 24, ArgumentWidth::One),
            ("18ff", 255, ArgumentWidth::One),
            ("190100", 256, ArgumentWidth::Two),
            ("19ffff", 65535, ArgumentWidth::Two),
            ("1a00010000", 65536, ArgumentWidth::Four),
            ("1affffffff", 4_294_967_295, ArgumentWidth::Four),
            ("1b0000000100000000", 4_294_967_296, ArgumentWidth::Eight),
            ("1bffffffffffffffff", u64::MAX, ArgumentWidth::Eight),
        ] {
            let mut input = bytes(hex);
            input[0] |= major << 5;
            let header = read_header_prefix(&input).unwrap();
            assert_eq!(header.argument(), Some(value));
            assert_eq!(header.argument_width(), width);
            assert_eq!(header.has_preferred_argument(), Some(true));
        }
    }
}

#[test]
fn long_key_comparison_cannot_bypass_the_work_budget() {
    let a = "a".repeat(1024);
    let mut b = a.clone();
    b.push('b');
    let entries = [
        MapEntry::new(ValueRef::Text(&a), ValueRef::Null),
        MapEntry::new(ValueRef::Text(&b), ValueRef::Null),
    ];
    let output = encode(
        &ValueRef::Map(&entries),
        Profile::preferred(),
        Limits::default(),
    )
    .unwrap();
    reason(
        validate_exact(
            output.as_bytes(),
            Profile::preferred(),
            Limits::default().with_work(2500).unwrap(),
        ),
        ErrorReason::WorkLimit,
    );
    validate_exact(output.as_bytes(), Profile::preferred(), Limits::default()).unwrap();
}

proptest! {
    #[test]
    fn arbitrary_binary32_bits_preserve_exact_value(bits in any::<u32>()) {
        let value = Float::binary32(bits);
        let output = encode(&ValueRef::Float(&value), Profile::preferred(), Limits::default()).unwrap();
        let item = validate_exact(output.as_bytes(), Profile::preferred(), Limits::default()).unwrap();
        assert_eq!(item.frame().float().unwrap().to_f64().to_bits(), value.to_f64().to_bits());
    }

    #[test]
    fn full_integer_arguments_round_trip(argument in any::<u64>(), negative in any::<bool>()) {
        let value = if negative { ValueRef::Negative(argument) } else { ValueRef::Unsigned(argument) };
        let output = encode(&value, Profile::preferred(), Limits::default()).unwrap();
        let item = validate_exact(output.as_bytes(), Profile::preferred(), Limits::default()).unwrap();
        let integer = item.frame().integer().unwrap();
        assert_eq!(integer.argument(), argument);
        assert_eq!(integer.is_negative(), negative);
    }

    #[test]
    fn arbitrary_binary64_bits_keep_exact_value_and_nan_payload(bits in any::<u64>()) {
        let value = Float::binary64(bits);
        let output = encode(&ValueRef::Float(&value), Profile::preferred(), Limits::default()).unwrap();
        let item = validate_exact(output.as_bytes(), Profile::preferred(), Limits::default()).unwrap();
        assert_eq!(item.frame().float().unwrap().to_f64().to_bits(), bits);
        let received = encode(&ValueRef::Float(&value), Profile::received(), Limits::default()).unwrap();
        assert_eq!(received.as_bytes().len(), 9);
        assert_eq!(validate_exact(received.as_bytes(), Profile::received(), Limits::default()).unwrap().frame().float().unwrap().bits(), bits);
    }

    #[test]
    fn hostile_bytes_never_escape_budgets_or_change_received_spans(input in proptest::collection::vec(any::<u8>(), 0..1024)) {
        if let Ok((frame, rest)) = frame_prefix(&input, Limits::default(), LengthPolicy::AllowIndefinite) {
            assert_eq!(frame.received_bytes().len() + rest.len(), input.len());
            assert_eq!(frame.received_bytes(), &input[..frame.received_bytes().len()]);
            if frame.validate(Profile::received(), Limits::default()).is_ok() {
                let output = encode(&ValueRef::Received(&frame), Profile::received(), Limits::default()).unwrap();
                assert_eq!(output.as_bytes(), frame.received_bytes());
            }
        }
    }
}
