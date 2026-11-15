// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::CborValue;

pub(crate) const FIRST_KEY_OFFSET: usize = 3;
pub(crate) const MAX_WIRE_LEN: usize = FIRST_KEY_OFFSET + 3 * 4;
pub(crate) const CANONICAL_WIRE: [u8; 13] = [
    0xa1, 0x60, 0xa3, 0x61, b'a', 0xf4, 0x61, b'z', 0xf5, 0x62, b'a', b'a', 0xf6,
];

#[derive(Clone, Copy, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum Key {
    // The reference ranks are fixed by the profile, independently of the
    // production comparator. The two short keys differ in their payload.
    A,
    Z,
    Aa,
}

pub(crate) const CANONICAL_KEYS: [Key; 3] = [Key::A, Key::Z, Key::Aa];

impl Key {
    pub(crate) fn symbolic() -> Self {
        let index: u8 = kani::any();
        kani::assume(index < 3);
        match index {
            0 => Self::A,
            1 => Self::Z,
            _ => Self::Aa,
        }
    }

    pub(crate) const fn text(self) -> &'static str {
        match self {
            Self::A => "a",
            Self::Z => "z",
            Self::Aa => "aa",
        }
    }

    pub(crate) const fn key_bytes(self) -> &'static [u8] {
        match self {
            Self::A => b"\x61a",
            Self::Z => b"\x61z",
            Self::Aa => b"\x62aa",
        }
    }

    pub(crate) const fn value_byte(self) -> u8 {
        match self {
            Self::A => 0xf4,
            Self::Z => 0xf5,
            Self::Aa => 0xf6,
        }
    }

    pub(crate) const fn value(self) -> CborValue {
        match self {
            Self::A => CborValue::Bool(false),
            Self::Z => CborValue::Bool(true),
            Self::Aa => CborValue::Null,
        }
    }

    pub(crate) fn owned_entry(self) -> (String, CborValue) {
        (String::from(self.text()), self.value())
    }
}

pub(crate) fn wire(keys: [Key; 3]) -> ([u8; MAX_WIRE_LEN], usize) {
    let mut wire = [0; MAX_WIRE_LEN];
    wire[..FIRST_KEY_OFFSET].copy_from_slice(&[0xa1, 0x60, 0xa3]);
    let mut start = FIRST_KEY_OFFSET;
    for key in keys {
        let encoded = key.key_bytes();
        let end = start + encoded.len();
        wire[start..end].copy_from_slice(encoded);
        wire[end] = key.value_byte();
        start = end + 1;
    }
    (wire, start)
}

pub(crate) fn assert_canonical_wire(wire: &[u8]) {
    assert!(matches!(
        wire,
        [0xa1, 0x60, 0xa3, 0x61, b'a', 0xf4, 0x61, b'z', 0xf5, 0x62, b'a', b'a', 0xf6]
    ));
}

pub(crate) fn cover_permutations(keys: [Key; 3]) {
    use Key::{Aa, A, Z};
    kani::cover!(keys == [A, Z, Aa], "a, z, aa");
    kani::cover!(keys == [A, Aa, Z], "a, aa, z");
    kani::cover!(keys == [Z, A, Aa], "z, a, aa");
    kani::cover!(keys == [Z, Aa, A], "z, aa, a");
    kani::cover!(keys == [Aa, A, Z], "aa, a, z");
    kani::cover!(keys == [Aa, Z, A], "aa, z, a");
}
