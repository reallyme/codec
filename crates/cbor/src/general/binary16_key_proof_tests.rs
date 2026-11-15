// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::compare_encoded_keys::check_previous_keys;
use super::limits::Budget;
use super::unique::{check_unique, KeyStarts};
use super::{
    frame_exact, read_header_prefix, validate_exact, CborError, ErrorReason, FloatWidth,
    LengthPolicy, Limits, MapKeyEquality, NanPolicy, Profile,
};

const BINARY16_HEAD: u8 = 0xf9;
const BINARY16_SIGN: u16 = 0x8000;
const BINARY16_FRACTION: u16 = 0x03ff;
const FIRST_KEY: usize = 1;
const SECOND_KEY: usize = 5;
const SECOND_VALUE: usize = 8;
const MAP_LENGTH: usize = 9;
const BINARY32_NAN_HEAD: u8 = 0xfa;

#[derive(Clone, Copy, Eq, PartialEq, kani::Arbitrary)]
enum NanKey {
    PositivePayloadOne,
    NegativePayloadOne,
    PositivePayloadTwo,
}

#[derive(Clone, Copy, Eq, PartialEq, kani::Arbitrary)]
enum NanAdmissionProfile {
    Received,
    Preferred,
    Cose,
}

impl NanKey {
    const fn bits(self) -> u16 {
        match self {
            Self::PositivePayloadOne => 0x7e01,
            Self::NegativePayloadOne => 0xfe01,
            Self::PositivePayloadTwo => 0x7e02,
        }
    }

    // Both signs share the quiet bit. The full fraction distinguishes the
    // two significands independently of the parser and comparison routines.
    const fn significand(self) -> u16 {
        self.bits() & BINARY16_FRACTION
    }
}

// The bound covers the 16 inline key indices and the byte-wise wipe of
// each protected header argument on this verifier target.
#[kani::proof]
#[kani::unwind(18)]
fn binary16_key_preserves_its_wire_bits_and_preferred_width() {
    let negative: bool = kani::any();
    let bits = zero_bits(negative);
    let bytes = scalar_wire(bits);
    let result = validate_exact(&bytes, Profile::preferred(), Limits::default());
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
    assert!(value.bits() == u64::from(bits));
    kani::cover!(negative, "negative zero has a preferred binary16 head");
    kani::cover!(!negative, "positive zero has a preferred binary16 head");
}

#[kani::proof]
#[kani::unwind(18)]
fn basic_model_checks_both_signs_as_one_key() {
    let (first, second) = kani::any();
    let wire = map_wire(zero_bits(first), zero_bits(second));
    assert_admission(check_basic_model(&wire), true);
    cover_sign_pairs(first, second);
}

#[kani::proof]
#[kani::unwind(18)]
fn encoded_scalar_check_compares_the_received_zero_signs() {
    let (first, second) = kani::any();
    let wire = map_wire(zero_bits(first), zero_bits(second));
    assert_admission(check_encoded_scalars(&wire), first == second);
    cover_sign_pairs(first, second);
}

#[kani::proof]
#[kani::unwind(18)]
fn binary16_nan_keys_retain_sign_and_significand() {
    let key: NanKey = kani::any();
    let encoded_scalars: bool = kani::any();
    let bytes = scalar_wire(key.bits());
    let profile = if encoded_scalars {
        let Ok(profile) = cose_profile() else {
            kani::assert(false, "the COSE profile is supported");
            return;
        };
        profile
    } else {
        Profile::preferred()
    };
    let Ok(item) = validate_exact(&bytes, profile, Limits::default()) else {
        kani::assert(false, "each fixture is an admitted binary16 NaN");
        return;
    };
    assert!(core::ptr::eq(
        item.frame().received_bytes(),
        bytes.as_slice()
    ));
    let Ok(value) = item.frame().float() else {
        kani::assert(false, "the admitted key is a float");
        return;
    };
    assert!(value.is_nan());
    assert!(value.width() == FloatWidth::Binary16);
    assert!(value.bits() == u64::from(key.bits()));

    use NanKey::{NegativePayloadOne, PositivePayloadOne, PositivePayloadTwo};
    kani::cover!(
        !encoded_scalars && key == PositivePayloadOne,
        "basic positive payload one"
    );
    kani::cover!(
        !encoded_scalars && key == NegativePayloadOne,
        "basic negative payload one"
    );
    kani::cover!(
        !encoded_scalars && key == PositivePayloadTwo,
        "basic positive payload two"
    );
    kani::cover!(
        encoded_scalars && key == PositivePayloadOne,
        "encoded positive payload one"
    );
    kani::cover!(
        encoded_scalars && key == NegativePayloadOne,
        "encoded negative payload one"
    );
    kani::cover!(
        encoded_scalars && key == PositivePayloadTwo,
        "encoded positive payload two"
    );
}

#[kani::proof]
#[kani::unwind(18)]
fn basic_model_nan_keys_compare_significands() {
    let (first, second): (NanKey, NanKey) = kani::any();
    let wire = map_wire(first.bits(), second.bits());
    assert_admission(
        check_basic_model(&wire),
        first.significand() == second.significand(),
    );
    cover_nan_pairs(first, second);
}

#[kani::proof]
#[kani::unwind(18)]
fn encoded_scalar_nan_keys_compare_full_encodings() {
    let (first, second): (NanKey, NanKey) = kani::any();
    let wire = map_wire(first.bits(), second.bits());
    assert_admission(check_encoded_scalars(&wire), first == second);
    cover_nan_pairs(first, second);
}

#[kani::proof]
#[kani::unwind(18)]
fn received_nan_keys_compare_significands_across_widths() {
    let different_significand: bool = kani::any();
    let half_wire = scalar_wire(NanKey::PositivePayloadOne.bits());
    let wide_wire = wide_nan_wire(different_significand);
    let (Ok(half_header), Ok(wide_header)) = (
        read_header_prefix(&half_wire),
        read_header_prefix(&wide_wire),
    ) else {
        kani::assert(false, "both NaN headers are complete");
        return;
    };
    assert!(core::ptr::eq(
        half_header.received_bytes(),
        half_wire.as_slice()
    ));
    assert!(core::ptr::eq(
        wide_header.received_bytes(),
        wide_wire.as_slice()
    ));
    let (Ok(Some(half)), Ok(Some(wide))) = (half_header.float(), wide_header.float()) else {
        kani::assert(false, "both headers carry NaN bits");
        return;
    };
    // The inline comparison uses equivalent. Larger maps group keys by
    // equivalence_key before testing each potential duplicate.
    assert!(half.equivalent(&wide) == !different_significand);
    assert!(wide.equivalent(&half) == !different_significand);
    assert!((half.equivalence_key() == wide.equivalence_key()) == !different_significand);
    kani::cover!(!different_significand, "exact widening");
    kani::cover!(different_significand, "extra fraction bit");
}

#[kani::proof]
#[kani::unwind(18)]
fn binary32_nan_width_admission_uses_the_selected_profile() {
    use NanAdmissionProfile::{Cose, Preferred, Received};
    let selected: NanAdmissionProfile = kani::any();
    let different_significand: bool = kani::any();
    let bytes = wide_nan_wire(different_significand);
    let profile = match selected {
        Received => Profile::received(),
        Preferred => Profile::preferred(),
        Cose => {
            let Ok(profile) = cose_profile() else {
                kani::assert(false, "the COSE profile is supported");
                return;
            };
            profile
        }
    };

    let result = validate_exact(&bytes, profile, Limits::default());
    // The former significand is 0x201 shifted left 13 places. The extra
    // low bit in the latter has no exact binary16 representation.
    if selected == Cose || (selected == Preferred && !different_significand) {
        assert!(matches!(result, Err(error) if error.reason() == ErrorReason::NonPreferredFloat));
    } else {
        let Ok(item) = result else {
            kani::assert(false, "the profile admits this binary32 NaN");
            return;
        };
        assert!(core::ptr::eq(
            item.frame().received_bytes(),
            bytes.as_slice()
        ));
        let Ok(value) = item.frame().float() else {
            kani::assert(false, "the admitted key is a float");
            return;
        };
        assert!(value.is_nan());
        assert!(value.width() == FloatWidth::Binary32);
        assert!(value.bits() == 0x7fc0_2000 + u64::from(different_significand));
    }

    kani::cover!(
        selected == Received && !different_significand,
        "received exact widening"
    );
    kani::cover!(
        selected == Received && different_significand,
        "received extra fraction bit"
    );
    kani::cover!(
        selected == Preferred && !different_significand,
        "preferred rejects exact widening"
    );
    kani::cover!(
        selected == Preferred && different_significand,
        "preferred admits extra fraction bit"
    );
    kani::cover!(
        selected == Cose && !different_significand,
        "COSE rejects exact widening"
    );
    kani::cover!(
        selected == Cose && different_significand,
        "COSE rejects extra fraction bit"
    );
}

fn wide_nan_wire(different_significand: bool) -> [u8; 5] {
    [
        BINARY32_NAN_HEAD,
        0x7f,
        0xc0,
        0x20,
        u8::from(different_significand),
    ]
}

fn check_basic_model(wire: &[u8; MAP_LENGTH]) -> Result<(), CborError> {
    let mut keys = KeyStarts::new();
    assert!(keys.push(FIRST_KEY).is_ok());
    assert!(keys.push(SECOND_KEY).is_ok());
    // Small maps take the inline pairwise path. The distinct values between
    // the key offsets cannot affect the keys' basic-model equality.
    check_unique(wire, &keys, &mut Budget::new(Limits::default()))
}

fn check_encoded_scalars(wire: &[u8; MAP_LENGTH]) -> Result<(), CborError> {
    let profile = cose_profile()?;
    let limits = Limits::default();
    let key = frame_exact(
        &wire[SECOND_KEY..SECOND_VALUE],
        limits,
        LengthPolicy::DefiniteOnly,
    )?;
    assert!(core::ptr::eq(
        key.received_bytes(),
        &wire[SECOND_KEY..SECOND_VALUE]
    ));
    // The previous entry contains a key and its value. The second value is
    // outside the comparator's inputs, including on the duplicate path.
    check_previous_keys(
        &wire[FIRST_KEY..SECOND_KEY],
        &key,
        &mut Budget::for_profile(limits, profile),
    )
}

fn assert_admission(result: Result<(), CborError>, duplicate: bool) {
    if duplicate {
        assert!(matches!(result, Err(error) if error.reason() == ErrorReason::DuplicateMapKey));
    } else {
        assert!(result.is_ok());
    }
}

fn zero_bits(negative: bool) -> u16 {
    if negative {
        BINARY16_SIGN
    } else {
        0
    }
}

fn scalar_wire(bits: u16) -> [u8; 3] {
    let [high, low] = bits.to_be_bytes();
    [BINARY16_HEAD, high, low]
}

fn map_wire(first: u16, second: u16) -> [u8; MAP_LENGTH] {
    let [first_head, first_high, first_low] = scalar_wire(first);
    let [second_head, second_high, second_low] = scalar_wire(second);
    [
        0xa2,
        first_head,
        first_high,
        first_low,
        0x01,
        second_head,
        second_high,
        second_low,
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

fn cover_nan_pairs(first: NanKey, second: NanKey) {
    use NanKey::{NegativePayloadOne, PositivePayloadOne, PositivePayloadTwo};
    kani::cover!(
        first == PositivePayloadOne && second == PositivePayloadOne,
        "positive payload one repeated"
    );
    kani::cover!(
        first == NegativePayloadOne && second == NegativePayloadOne,
        "negative payload one repeated"
    );
    kani::cover!(
        first == PositivePayloadTwo && second == PositivePayloadTwo,
        "positive payload two repeated"
    );
    kani::cover!(
        first == PositivePayloadOne && second == NegativePayloadOne,
        "positive then negative payload one"
    );
    kani::cover!(
        first == NegativePayloadOne && second == PositivePayloadOne,
        "negative then positive payload one"
    );
    kani::cover!(
        first == PositivePayloadOne && second == PositivePayloadTwo,
        "positive payload one then positive payload two"
    );
    kani::cover!(
        first == PositivePayloadTwo && second == PositivePayloadOne,
        "positive payload two then positive payload one"
    );
    kani::cover!(
        first == NegativePayloadOne && second == PositivePayloadTwo,
        "negative payload one then positive payload two"
    );
    kani::cover!(
        first == PositivePayloadTwo && second == NegativePayloadOne,
        "positive payload two then negative payload one"
    );
}
