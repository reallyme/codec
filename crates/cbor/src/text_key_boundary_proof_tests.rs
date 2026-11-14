// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

// The payloads sort in the opposite order from their encoded lengths.
const SHORT_PAYLOAD: [u8; 23] = [b'z'; 23];
const LONG_PAYLOAD: [u8; 24] = [b'a'; 24];
pub(crate) const SHORT_TEXT: &str = "zzzzzzzzzzzzzzzzzzzzzzz";
pub(crate) const LONG_TEXT: &str = "aaaaaaaaaaaaaaaaaaaaaaaa";
pub(crate) const WIRE_LEN: usize = 55;
pub(crate) const CANONICAL_WIRE: [u8; WIRE_LEN] = wire(false);
pub(crate) const LONGER_FIRST_WIRE: [u8; WIRE_LEN] = wire(true);

const fn wire(long_first: bool) -> [u8; WIRE_LEN] {
    let mut bytes = [0; WIRE_LEN];
    bytes[0] = 0xa1;
    bytes[1] = 0x60;
    bytes[2] = 0xa2;
    let (short_start, long_start) = if long_first { (30, 3) } else { (3, 28) };
    bytes[short_start] = 0x77;
    let mut index = 0;
    while index < SHORT_PAYLOAD.len() {
        bytes[short_start + 1 + index] = SHORT_PAYLOAD[index];
        index += 1;
    }
    bytes[short_start + 1 + SHORT_PAYLOAD.len()] = 0xf4;
    bytes[long_start] = 0x78;
    bytes[long_start + 1] = 0x18;
    index = 0;
    while index < LONG_PAYLOAD.len() {
        bytes[long_start + 2 + index] = LONG_PAYLOAD[index];
        index += 1;
    }
    bytes[long_start + 2 + LONG_PAYLOAD.len()] = 0xf5;
    bytes
}

pub(crate) fn assert_canonical_wire(bytes: &[u8]) {
    assert!(bytes.len() == WIRE_LEN);
    // A symbolic index checks every byte without a 55-iteration comparison.
    let index: usize = kani::any();
    kani::assume(index < WIRE_LEN);
    assert!(bytes[index] == CANONICAL_WIRE[index]);
}
