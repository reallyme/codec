// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::compare_encoded_keys::check_previous_keys;
use super::limits::Budget;
use super::unique::{check_unique, KeyStarts};
use super::{
    frame_exact, validate_exact, CborError, ErrorReason, FloatWidth, LengthPolicy, Limits,
    MapKeyEquality, NanPolicy, Profile,
};

const POSITIVE_ZERO: [u8; 3] = [0xf9, 0x00, 0x00];
const NEGATIVE_ZERO: [u8; 3] = [0xf9, 0x80, 0x00];
const FIRST_KEY: usize = 1;
const SECOND_KEY: usize = 5;
const SECOND_VALUE: usize = 8;
const MAP_LENGTH: usize = 9;

// The bound covers the 16 inline key indices and the byte-wise wipe of
// each protected header argument on this verifier target.
#[kani::proof]
#[kani::unwind(18)]
fn binary16_key_preserves_its_wire_bits_and_preferred_width() {
    let negative: bool = kani::any();
    let bytes = zero_bytes(negative);
    let result = validate_exact(bytes, Profile::preferred(), Limits::default());
    let Ok(item) = result else {
        kani::assert(false, "either binary16 signed zero has preferred width");
        return;
    };
    assert!(core::ptr::eq(
        item.frame().received_bytes(),
        bytes.as_slice()
    ));
    let Ok(value) = item.frame().float() else {
        kani::assert(false, "the preferred key is a float");
        return;
    };
    assert!(value.width() == FloatWidth::Binary16);
    assert!(value.bits() == if negative { 0x8000 } else { 0 });
    kani::cover!(negative, "negative zero has a preferred binary16 head");
    kani::cover!(!negative, "positive zero has a preferred binary16 head");
}

#[kani::proof]
#[kani::unwind(18)]
fn basic_model_checks_both_signs_as_one_key() {
    let (first, second) = kani::any();
    let wire = map_wire(first, second);
    let mut keys = KeyStarts::new();
    assert!(keys.push(FIRST_KEY).is_ok());
    assert!(keys.push(SECOND_KEY).is_ok());
    // Small maps take the inline pairwise path. The distinct values between
    // the key offsets cannot affect the keys' basic-model equality.
    let result = check_unique(&wire, &keys, &mut Budget::new(Limits::default()));
    assert_duplicate(result);
    cover_sign_pairs(first, second);
}

#[kani::proof]
#[kani::unwind(18)]
fn encoded_scalar_check_compares_the_received_zero_signs() {
    let (first, second) = kani::any();
    let wire = map_wire(first, second);
    let Ok(profile) = cose_profile() else {
        kani::assert(
            false,
            "encoded scalar keys and binary16 NaNs form a supported profile",
        );
        return;
    };
    let limits = Limits::default();
    let Ok(key) = frame_exact(
        &wire[SECOND_KEY..SECOND_VALUE],
        limits,
        LengthPolicy::DefiniteOnly,
    ) else {
        kani::assert(
            false,
            "the second binary16 head consumes exactly three bytes",
        );
        return;
    };
    assert!(core::ptr::eq(
        key.received_bytes(),
        &wire[SECOND_KEY..SECOND_VALUE]
    ));
    // The previous entry contains a key and its value. The second value is
    // outside the comparator's inputs, including on the duplicate path.
    let result = check_previous_keys(
        &wire[FIRST_KEY..SECOND_KEY],
        &key,
        &mut Budget::for_profile(limits, profile),
    );
    if first == second {
        assert_duplicate(result);
    } else {
        assert!(result.is_ok());
    }
    cover_sign_pairs(first, second);
}

fn assert_duplicate(result: Result<(), CborError>) {
    assert!(matches!(result, Err(error) if error.reason() == ErrorReason::DuplicateMapKey));
}

fn zero_bytes(negative: bool) -> &'static [u8; 3] {
    if negative {
        &NEGATIVE_ZERO
    } else {
        &POSITIVE_ZERO
    }
}

fn map_wire(first: bool, second: bool) -> [u8; MAP_LENGTH] {
    let [first_head, first_sign, first_tail] = *zero_bytes(first);
    let [second_head, second_sign, second_tail] = *zero_bytes(second);
    [
        0xa2,
        first_head,
        first_sign,
        first_tail,
        0x01,
        second_head,
        second_sign,
        second_tail,
        0x02,
    ]
}

fn cose_profile() -> Result<Profile, CborError> {
    Profile::preferred()
        .with_map_key_equality(MapKeyEquality::EncodedScalars)?
        .with_nan_policy(NanPolicy::Binary16Only)
}

fn cover_sign_pairs(first: bool, second: bool) {
    kani::cover!(!first && !second, "positive zero repeated");
    kani::cover!(first && second, "negative zero repeated");
    kani::cover!(!first && second, "positive then negative zero");
    kani::cover!(first && !second, "negative then positive zero");
}
