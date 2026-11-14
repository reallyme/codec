// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

/// Wire shape shared by the decoder and encoder obligations. Distinct
/// boolean leaves make it possible to detect a swapped key/value association.
pub(crate) struct TwoEntryMap {
    pub(crate) first: u8,
    pub(crate) second: u8,
}

// Lexically, "aa" precedes "z". Keep distinct leaves attached to each key
// so an encoder that sorts just the keys cannot satisfy byte equality.
pub(crate) const MIXED_LENGTH_CANONICAL: [u8; 10] =
    [0xa1, 0x60, 0xa2, 0x61, b'z', 0xf4, 0x62, b'a', b'a', 0xf5];
pub(crate) const MIXED_LENGTH_REVERSED: [u8; 10] =
    [0xa1, 0x60, 0xa2, 0x62, b'a', b'a', 0xf5, 0x61, b'z', 0xf4];

pub(crate) fn assert_mixed_length_wire(bytes: &[u8]) {
    assert!(matches!(
        bytes,
        [0xa1, 0x60, 0xa2, 0x61, b'z', 0xf4, 0x62, b'a', b'a', 0xf5]
    ));
}

// The first byte of the unsigned key sorts before the text header, but
// the unsigned key is one byte longer.
pub(crate) const TEXT_KEY_ENCODING: &[u8] = b"\x61z";
pub(crate) const UNSIGNED_KEY_ENCODING: &[u8] = b"\x19\x01\x00";
pub(crate) const UNSIGNED_KEY: u64 = 256;
pub(crate) const MIXED_KEY_CANONICAL: [u8; 10] =
    [0xa1, 0x60, 0xa2, 0x61, b'z', 0xf4, 0x19, 0x01, 0x00, 0xf5];
pub(crate) const MIXED_KEY_BYTEWISE: [u8; 10] =
    [0xa1, 0x60, 0xa2, 0x19, 0x01, 0x00, 0xf5, 0x61, b'z', 0xf4];

pub(crate) fn assert_mixed_key_wire(bytes: &[u8]) {
    assert!(matches!(
        bytes,
        [0xa1, 0x60, 0xa2, 0x61, b'z', 0xf4, 0x19, 0x01, 0x00, 0xf5]
    ));
}

impl TwoEntryMap {
    pub(crate) const WIRE_LENGTH: usize = 9;

    pub(crate) fn symbolic() -> Self {
        let [first, second]: [u8; 2] = kani::any();
        Self {
            first: first & 0x7f,
            second: second & 0x7f,
        }
    }

    pub(crate) fn wire(&self) -> [u8; Self::WIRE_LENGTH] {
        [
            0xa1,
            0x60,
            0xa2,
            0x61,
            self.first,
            0xf4,
            0x61,
            self.second,
            0xf5,
        ]
    }

    pub(crate) fn assert_wire(&self, bytes: &[u8]) {
        assert!(matches!(
            bytes,
            [0xa1, 0x60, 0xa2, 0x61, first, 0xf4, 0x61, second, 0xf5]
                if *first == self.first && *second == self.second
        ));
    }
}

#[path = "three_entry_map_proof_tests.rs"]
pub(crate) mod three_entry;

#[path = "text_key_boundary_proof_tests.rs"]
pub(crate) mod text_key_boundary;
