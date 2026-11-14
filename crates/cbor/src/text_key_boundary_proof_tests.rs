// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

// The payloads sort in the opposite order from their encoded lengths.
const SHORT_PAYLOAD: [u8; 23] = [b'z'; 23];
const UTF8_PAYLOAD: [u8; 24] = {
    let mut payload = [b'a'; 24];
    payload[22] = 0xc3;
    payload[23] = 0xa9;
    payload
};
pub(crate) const SHORT_TEXT: &str = "zzzzzzzzzzzzzzzzzzzzzzz";
pub(crate) const WIRE_LEN: usize = 55;

#[derive(Clone, Copy)]
pub(crate) struct Fixture {
    pub(crate) long_text: &'static str,
    pub(crate) canonical: &'static [u8; WIRE_LEN],
    pub(crate) longer_first: &'static [u8; WIRE_LEN],
}

pub(crate) const ASCII: Fixture = Fixture {
    long_text: "aaaaaaaaaaaaaaaaaaaaaaaa",
    canonical: &wire(false, &[b'a'; 24]),
    longer_first: &wire(true, &[b'a'; 24]),
};

pub(crate) const UTF8: Fixture = Fixture {
    long_text: "aaaaaaaaaaaaaaaaaaaaaaé",
    canonical: &wire(false, &UTF8_PAYLOAD),
    longer_first: &wire(true, &UTF8_PAYLOAD),
};

const fn wire(long_first: bool, long_payload: &[u8; 24]) -> [u8; WIRE_LEN] {
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
    while index < long_payload.len() {
        bytes[long_start + 2 + index] = long_payload[index];
        index += 1;
    }
    bytes[long_start + 2 + long_payload.len()] = 0xf5;
    bytes
}

pub(crate) fn assert_canonical_wire(bytes: &[u8], fixture: Fixture) {
    assert!(bytes.len() == WIRE_LEN);
    // A symbolic index checks every byte without a 55-iteration comparison.
    let index: usize = kani::any();
    kani::assume(index < WIRE_LEN);
    assert!(bytes[index] == fixture.canonical[index]);
}
