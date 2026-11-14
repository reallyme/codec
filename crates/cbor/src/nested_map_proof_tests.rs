// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

/// Wire shape shared by the decoder and encoder obligations. Distinct
/// boolean leaves make it possible to detect a swapped key/value association.
pub(crate) struct TwoEntryMap {
    pub(crate) first: u8,
    pub(crate) second: u8,
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
