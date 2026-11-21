// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Explicit large-container admission retains every independent resource budget.
#![allow(clippy::unwrap_used, reason = "bounded conformance fixtures")]

use codec_cbor::general::{
    encode, encode_value, validate_exact, CborError, ErrorReason, Limits, Profile, ValueRef,
    DEFAULT_BYTES, DEFAULT_ENTRIES, MAX_DEPTH, MAX_ENTRIES, MAX_NODES, MAX_TEXT_BYTES, MAX_WORK,
};
use codec_proto::generated::proto::reallyme::codec::v1 as proto;

fn assert_reason<T>(result: Result<T, CborError>, expected: ErrorReason) {
    assert_eq!(result.err().map(CborError::reason), Some(expected));
}

fn array_wire(octets: &[u8], indefinite: bool) -> Vec<u8> {
    let mut wire = Vec::new();
    if indefinite {
        wire.push(0x9f);
    } else {
        match octets.len() {
            0..=23 => wire.push(0x80 | u8::try_from(octets.len()).unwrap()),
            24..=255 => wire.extend_from_slice(&[0x98, u8::try_from(octets.len()).unwrap()]),
            256..=65_535 => {
                wire.push(0x99);
                wire.extend_from_slice(&u16::try_from(octets.len()).unwrap().to_be_bytes());
            }
            _ => {
                wire.push(0x9a);
                wire.extend_from_slice(&u32::try_from(octets.len()).unwrap().to_be_bytes());
            }
        }
    }
    for &octet in octets {
        if octet < 24 {
            wire.push(octet);
        } else {
            wire.extend_from_slice(&[0x18, octet]);
        }
    }
    if indefinite {
        wire.push(0xff);
    }
    wire
}

fn array_values(octets: &[u8]) -> Vec<ValueRef<'_>> {
    octets
        .iter()
        .map(|&octet| ValueRef::Unsigned(u64::from(octet)))
        .collect()
}

#[test]
fn defaults_and_all_other_limit_fields_remain_unchanged() {
    let defaults = Limits::default();
    assert_eq!(DEFAULT_ENTRIES, 4_096);
    assert_eq!(MAX_ENTRIES, 65_536);
    assert_eq!(defaults.entries(), DEFAULT_ENTRIES);
    assert_eq!(defaults.input_bytes(), DEFAULT_BYTES);
    assert_eq!(defaults.output_bytes(), DEFAULT_BYTES);
    assert_eq!(defaults.depth(), MAX_DEPTH);
    assert_eq!(defaults.nodes(), MAX_NODES);
    assert_eq!(defaults.byte_string_bytes(), DEFAULT_BYTES);
    assert_eq!(defaults.text_bytes(), MAX_TEXT_BYTES);
    assert_eq!(defaults.work(), 16_777_216);
    assert_eq!(MAX_WORK, 67_108_864);

    let explicit = defaults.with_entries(65_536).unwrap();
    assert_eq!(explicit.entries(), MAX_ENTRIES);
    // Equality covers every field, including any budget added in the future.
    assert_eq!(explicit.with_entries(DEFAULT_ENTRIES).unwrap(), defaults);
    assert_reason(
        defaults.with_entries(MAX_ENTRIES.checked_add(1).unwrap()),
        ErrorReason::InvalidLimit,
    );

    let options = proto::CodecGeneralCborOptions {
        entries: Some(65_536),
        ..Default::default()
    };
    assert_eq!(Limits::try_from(&options).unwrap(), explicit);
    assert_eq!(
        Limits::try_from(&proto::CodecGeneralCborOptions::default()).unwrap(),
        defaults
    );
    assert_reason(
        Limits::try_from(&proto::CodecGeneralCborOptions {
            entries: Some(65_537),
            ..Default::default()
        }),
        ErrorReason::InvalidLimit,
    );
}

#[test]
fn unsigned_octet_arrays_above_the_default_round_trip_below_64_kib() {
    let limits = Limits::default().with_entries(65_536).unwrap();
    for count in [4_097, 32_768] {
        let octets: Vec<u8> = (0..=u8::MAX).cycle().take(count).collect();
        let values = array_values(&octets);
        let expected = array_wire(&octets, false);
        assert!(expected.len() < 65_536);
        let encoded = encode(&ValueRef::Array(&values), Profile::preferred(), limits).unwrap();
        assert_eq!(encoded.as_bytes(), expected);

        for indefinite in [false, true] {
            let wire = array_wire(&octets, indefinite);
            let item = validate_exact(&wire, Profile::received(), limits).unwrap();
            let mut children = item.children().unwrap();
            for &octet in &octets {
                let child = children.next().unwrap().unwrap();
                let integer = child.frame().integer().unwrap();
                assert!(!integer.is_negative());
                assert_eq!(integer.argument(), u64::from(octet));
            }
            assert!(children.next().is_none());
        }
    }
}

#[test]
fn default_and_explicit_entry_limits_reject_in_both_directions() {
    let count = DEFAULT_ENTRIES.checked_add(1).unwrap();
    let octets = vec![0; count];
    let values = array_values(&octets);
    let exact = Limits::default().with_entries(count).unwrap();
    encode(&ValueRef::Array(&values), Profile::preferred(), exact).unwrap();

    for limits in [
        Limits::default(),
        exact.with_entries(count.checked_sub(1).unwrap()).unwrap(),
        exact.with_entries(0).unwrap(),
    ] {
        assert_reason(
            encode(&ValueRef::Array(&values), Profile::preferred(), limits),
            ErrorReason::EntryLimit,
        );
        for indefinite in [false, true] {
            assert_reason(
                validate_exact(
                    &array_wire(&octets, indefinite),
                    Profile::received(),
                    limits,
                ),
                ErrorReason::EntryLimit,
            );
        }
    }
    for indefinite in [false, true] {
        validate_exact(&array_wire(&octets, indefinite), Profile::received(), exact).unwrap();
    }
}

#[test]
fn larger_arrays_still_charge_the_container_against_the_total_node_budget() {
    let limits = Limits::default().with_entries(65_536).unwrap();
    let count = MAX_NODES.checked_sub(1).unwrap();
    let octets = vec![0; count];
    let values = array_values(&octets);
    encode(&ValueRef::Array(&values), Profile::preferred(), limits).unwrap();
    for indefinite in [false, true] {
        validate_exact(
            &array_wire(&octets, indefinite),
            Profile::received(),
            limits,
        )
        .unwrap();
    }
    let tighter = limits.with_nodes(count).unwrap();
    assert_reason(
        encode(&ValueRef::Array(&values), Profile::preferred(), tighter),
        ErrorReason::NodeLimit,
    );
    assert_reason(
        validate_exact(&array_wire(&octets, false), Profile::preferred(), tighter),
        ErrorReason::NodeLimit,
    );

    // All children fit the entry limit, but the array itself is also a node.
    let too_many = vec![0; MAX_NODES];
    assert_reason(
        encode(
            &ValueRef::Array(&array_values(&too_many)),
            Profile::preferred(),
            limits,
        ),
        ErrorReason::NodeLimit,
    );
    for indefinite in [false, true] {
        assert_reason(
            validate_exact(
                &array_wire(&too_many, indefinite),
                Profile::received(),
                limits,
            ),
            ErrorReason::NodeLimit,
        );
    }
}

#[test]
fn larger_sibling_arrays_share_one_node_budget() {
    const CHILDREN: usize = 4_097;
    // 1 root + 2 child arrays + 2 * 4,097 scalar elements.
    const NODES: usize = 8_197;
    let octets = vec![0; CHILDREN];
    let values = array_values(&octets);
    let siblings = [ValueRef::Array(&values), ValueRef::Array(&values)];
    let limits = Limits::default()
        .with_entries(65_536)
        .unwrap()
        .with_nodes(NODES)
        .unwrap();
    let encoded = encode(&ValueRef::Array(&siblings), Profile::preferred(), limits).unwrap();
    validate_exact(encoded.as_bytes(), Profile::preferred(), limits).unwrap();
    let tighter = limits.with_nodes(NODES.checked_sub(1).unwrap()).unwrap();
    assert_reason(
        encode(&ValueRef::Array(&siblings), Profile::preferred(), tighter),
        ErrorReason::NodeLimit,
    );
    assert_reason(
        validate_exact(encoded.as_bytes(), Profile::preferred(), tighter),
        ErrorReason::NodeLimit,
    );
}

#[test]
fn explicit_entries_preserve_byte_depth_and_work_enforcement() {
    let octets = vec![0; 4_097];
    let values = array_values(&octets);
    let wire = array_wire(&octets, false);
    let limits = Limits::default().with_entries(65_536).unwrap();
    assert_reason(
        validate_exact(
            &wire,
            Profile::preferred(),
            limits
                .with_input_bytes(wire.len().checked_sub(1).unwrap())
                .unwrap(),
        ),
        ErrorReason::InputLimit,
    );
    for (limits, expected) in [
        (limits.with_depth(0).unwrap(), ErrorReason::DepthLimit),
        (limits.with_work(1).unwrap(), ErrorReason::WorkLimit),
    ] {
        assert_reason(
            encode(&ValueRef::Array(&values), Profile::preferred(), limits),
            expected,
        );
        assert_reason(
            validate_exact(&wire, Profile::preferred(), limits),
            expected,
        );
    }
    assert_reason(
        encode(
            &ValueRef::Array(&values),
            Profile::preferred(),
            limits
                .with_output_bytes(wire.len().checked_sub(1).unwrap())
                .unwrap(),
        ),
        ErrorReason::OutputLimit,
    );
}

#[test]
fn generated_array_encoding_uses_the_same_explicit_entry_budget() {
    let octets = vec![0; 4_097];
    let generated = proto::CodecGeneralCborValue {
        value: Some(proto::codec_general_cbor_value::Value::ArrayValue(
            Box::new(proto::CodecGeneralCborArray {
                values: octets
                    .iter()
                    .map(|_| proto::CodecGeneralCborValue {
                        value: Some(proto::codec_general_cbor_value::Value::UnsignedValue(
                            Box::new(proto::CodecGeneralCborUnsigned { argument: 0 }),
                        )),
                    })
                    .collect(),
            }),
        )),
    };
    let limits = Limits::try_from(&proto::CodecGeneralCborOptions {
        entries: Some(65_536),
        ..Default::default()
    })
    .unwrap();
    let encoded = encode_value(&generated, Profile::preferred(), limits).unwrap();
    assert_eq!(encoded.as_bytes(), array_wire(&octets, false));
    assert_reason(
        encode_value(&generated, Profile::preferred(), Limits::default()),
        ErrorReason::EntryLimit,
    );
}
