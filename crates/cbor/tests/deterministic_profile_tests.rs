// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Tests for deterministic generic-CBOR profile declarations.

use codec_cbor::{
    DeterministicCborInteger, DeterministicCborMapEntry, DeterministicCborMapKey,
    DeterministicCborProfileError, DeterministicCborValue, DETERMINISTIC_CBOR_NEGATIVE_MAX,
    DETERMINISTIC_CBOR_NEGATIVE_MIN, MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
    MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES, MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES,
    MAX_DETERMINISTIC_CBOR_INPUT_LEN, MAX_DETERMINISTIC_CBOR_NESTING_DEPTH,
    MAX_DETERMINISTIC_CBOR_NODES, MAX_DETERMINISTIC_CBOR_OUTPUT_LEN,
};
use zeroize::ZeroizeOnDrop;

const _: () = {
    // These values are the frozen Stage 2 semantic profile, not tuning
    // suggestions. Keep exact assertions here so changing any public limit is
    // an explicit profile-version decision rather than a silent refactor.
    assert!(MAX_DETERMINISTIC_CBOR_INPUT_LEN == 1_048_576);
    assert!(MAX_DETERMINISTIC_CBOR_OUTPUT_LEN == 1_048_576);
    assert!(MAX_DETERMINISTIC_CBOR_NESTING_DEPTH == 64);
    assert!(MAX_DETERMINISTIC_CBOR_NODES == 65_536);
    assert!(MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES == 16_384);
    assert!(MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES == 1_048_576);
    assert!(MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES == 1_048_576);
    assert!(DETERMINISTIC_CBOR_NEGATIVE_MIN == i64::MIN);
    assert!(DETERMINISTIC_CBOR_NEGATIVE_MAX == -1);
};

#[test]
fn negative_integer_constructor_rejects_non_negative_values() {
    let min = DeterministicCborInteger::negative(DETERMINISTIC_CBOR_NEGATIVE_MIN);
    assert!(matches!(&min, Ok(DeterministicCborInteger::Negative(_))));
    if let Ok(DeterministicCborInteger::Negative(value)) = &min {
        assert_eq!(value.value(), DETERMINISTIC_CBOR_NEGATIVE_MIN);
    }

    let max = DeterministicCborInteger::negative(DETERMINISTIC_CBOR_NEGATIVE_MAX);
    assert!(matches!(&max, Ok(DeterministicCborInteger::Negative(_))));
    if let Ok(DeterministicCborInteger::Negative(value)) = &max {
        assert_eq!(value.value(), DETERMINISTIC_CBOR_NEGATIVE_MAX);
    }

    assert!(matches!(
        DeterministicCborInteger::negative(0),
        Err(DeterministicCborProfileError::NegativeIntegerMustBeNegative)
    ));
    assert!(matches!(
        DeterministicCborInteger::negative(1),
        Err(DeterministicCborProfileError::NegativeIntegerMustBeNegative)
    ));
}

#[test]
fn map_entry_preserves_ordered_key_and_value_model() {
    let entry = DeterministicCborMapEntry::new(
        DeterministicCborMapKey::Integer(DeterministicCborInteger::unsigned(u64::MAX)),
        DeterministicCborValue::Bool(true),
    );

    assert!(matches!(
        entry.key(),
        DeterministicCborMapKey::Integer(DeterministicCborInteger::Unsigned(u64::MAX))
    ));
    assert!(matches!(entry.value(), DeterministicCborValue::Bool(true)));
}

#[test]
fn text_key_construction_preserves_exact_utf8_without_normalization() {
    let decomposed = "e\u{301}".to_owned();
    let composed = "é";
    assert_ne!(decomposed.as_bytes(), composed.as_bytes());

    let key = DeterministicCborMapKey::text(decomposed.clone());
    let text_bytes = match &key {
        DeterministicCborMapKey::Text(value) => Some(value.as_bytes()),
        _ => None,
    };
    assert_eq!(text_bytes, Some(decomposed.as_bytes()));
    assert_ne!(text_bytes, Some(composed.as_bytes()));
}

#[test]
fn debug_redacts_text_bytes_and_text_keys() {
    let unsigned_debug = format!("{:?}", DeterministicCborInteger::unsigned(987_654_321));
    assert!(unsigned_debug.contains("<redacted>"));
    assert!(!unsigned_debug.contains("987654321"));

    let negative = DeterministicCborInteger::negative(-987_654_321);
    if let Ok(negative) = negative {
        let negative_debug = format!("{negative:?}");
        assert!(negative_debug.contains("<redacted>"));
        assert!(!negative_debug.contains("-987654321"));
    }

    let bool_debug = format!("{:?}", DeterministicCborValue::Bool(true));
    assert!(bool_debug.contains("<redacted>"));
    assert!(!bool_debug.contains("true"));

    let integer_key =
        DeterministicCborMapKey::Integer(DeterministicCborInteger::unsigned(987_654_321));
    let integer_key_debug = format!("{integer_key:?}");
    assert!(integer_key_debug.contains("<redacted>"));
    assert!(!integer_key_debug.contains("987654321"));

    let entry = DeterministicCborMapEntry::new(
        DeterministicCborMapKey::text("passport-number".to_owned()),
        DeterministicCborValue::Text("P123456789".to_owned()),
    );
    let entry_debug = format!("{entry:?}");
    assert!(entry_debug.contains("<redacted>"));
    assert!(!entry_debug.contains("passport-number"));
    assert!(!entry_debug.contains("P123456789"));

    let value = DeterministicCborValue::Map(vec![
        DeterministicCborMapEntry::new(
            DeterministicCborMapKey::text("passport-number".to_owned()),
            DeterministicCborValue::Text("P123456789".to_owned()),
        ),
        DeterministicCborMapEntry::new(
            DeterministicCborMapKey::Integer(DeterministicCborInteger::unsigned(7)),
            DeterministicCborValue::Bytes(vec![1, 2, 3, 4]),
        ),
    ]);

    let debug = format!("{value:?}");

    assert!(debug.contains("len"));
    assert!(!debug.contains("passport-number"));
    assert!(!debug.contains("P123456789"));
    assert!(!debug.contains("[1, 2, 3, 4]"));
}

#[test]
fn every_recursive_native_owner_zeroizes_on_drop() {
    fn require_zeroize_on_drop<T: ZeroizeOnDrop>() {}

    require_zeroize_on_drop::<DeterministicCborInteger>();
    require_zeroize_on_drop::<codec_cbor::DeterministicCborNegativeInteger>();
    require_zeroize_on_drop::<DeterministicCborMapKey>();
    require_zeroize_on_drop::<DeterministicCborMapEntry>();
    require_zeroize_on_drop::<DeterministicCborValue>();
}
