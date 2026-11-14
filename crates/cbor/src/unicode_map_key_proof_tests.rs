// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::CborError;

pub(crate) const PRECOMPOSED: &str = "\u{00e9}";
pub(crate) const DECOMPOSED: &str = "e\u{0301}";
pub(crate) const CANONICAL: [u8; 12] = [
    0xa1, 0x60, 0xa2, 0x62, 0xc3, 0xa9, 0xf4, 0x63, 0x65, 0xcc, 0x81, 0xf5,
];
const REVERSED: [u8; 12] = [
    0xa1, 0x60, 0xa2, 0x63, 0x65, 0xcc, 0x81, 0xf5, 0x62, 0xc3, 0xa9, 0xf4,
];
// Keep the values distinct even in duplicate fixtures, so matching an
// entire key/value pair cannot pass the duplicate check.
const REPEATED_PRECOMPOSED: [u8; 11] = [
    0xa1, 0x60, 0xa2, 0x62, 0xc3, 0xa9, 0xf4, 0x62, 0xc3, 0xa9, 0xf5,
];
const REPEATED_DECOMPOSED: [u8; 13] = [
    0xa1, 0x60, 0xa2, 0x63, 0x65, 0xcc, 0x81, 0xf4, 0x63, 0x65, 0xcc, 0x81, 0xf5,
];

#[derive(Clone, Copy)]
pub(crate) enum Map {
    Canonical,
    Reversed,
    RepeatedPrecomposed,
    RepeatedDecomposed,
}

impl Map {
    pub(crate) fn symbolic() -> Self {
        let precomposed_first: bool = kani::any();
        let repeat: bool = kani::any();
        match (precomposed_first, repeat) {
            (true, false) => Self::Canonical,
            (false, false) => Self::Reversed,
            (true, true) => Self::RepeatedPrecomposed,
            (false, true) => Self::RepeatedDecomposed,
        }
    }

    pub(crate) const fn wire(self) -> &'static [u8] {
        match self {
            Self::Canonical => &CANONICAL,
            Self::Reversed => &REVERSED,
            Self::RepeatedPrecomposed => &REPEATED_PRECOMPOSED,
            Self::RepeatedDecomposed => &REPEATED_DECOMPOSED,
        }
    }

    pub(crate) const fn spans(self) -> [(usize, usize); 2] {
        match self {
            Self::Canonical => [(3, 6), (7, 11)],
            Self::Reversed => [(3, 7), (8, 11)],
            Self::RepeatedPrecomposed => [(3, 6), (7, 10)],
            Self::RepeatedDecomposed => [(3, 7), (8, 12)],
        }
    }

    pub(crate) const fn text(self, second: bool) -> &'static str {
        match (self, second) {
            (Self::Canonical, false) | (Self::Reversed, true) | (Self::RepeatedPrecomposed, _) => {
                PRECOMPOSED
            }
            _ => DECOMPOSED,
        }
    }

    pub(crate) const fn bit(self, second: bool) -> bool {
        match self {
            Self::Reversed => !second,
            _ => second,
        }
    }

    pub(crate) const fn second_error(self) -> Option<CborError> {
        match self {
            Self::Canonical => None,
            Self::Reversed => Some(CborError::MapKeysOutOfOrder),
            Self::RepeatedPrecomposed | Self::RepeatedDecomposed => {
                Some(CborError::DuplicateMapKey)
            }
        }
    }

    pub(crate) fn cover_all(self) {
        kani::cover!(
            matches!(self, Self::Canonical),
            "both spellings admitted in length order"
        );
        kani::cover!(
            matches!(self, Self::Reversed),
            "longer spelling comes first"
        );
        kani::cover!(
            matches!(self, Self::RepeatedPrecomposed),
            "precomposed spelling repeated"
        );
        kani::cover!(
            matches!(self, Self::RepeatedDecomposed),
            "decomposed spelling repeated"
        );
    }
}

pub(crate) fn assert_canonical(wire: &[u8]) {
    assert!(wire.len() == CANONICAL.len());
    let index: usize = kani::any();
    kani::assume(index < CANONICAL.len());
    assert!(wire[index] == CANONICAL[index]);
}
