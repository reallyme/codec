// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

/// Two canonical nonnegative INTEGER magnitudes borrowed from one DER SEQUENCE.
///
/// No width, subgroup-order, nonzero, or low-S condition is implied. Input
/// ownership and zeroization remain with the caller. This view never copies or
/// owns its payload and deliberately does not implement Debug or Clone.
pub struct DerUnsignedIntegerPair<'a> {
    pub(super) first: &'a [u8],
    pub(super) second: &'a [u8],
}

impl<'a> DerUnsignedIntegerPair<'a> {
    /// First canonical unsigned magnitude; zero is `[0]`.
    #[must_use]
    pub const fn first(&self) -> &'a [u8] {
        self.first
    }

    /// Second canonical unsigned magnitude; zero is `[0]`.
    #[must_use]
    pub const fn second(&self) -> &'a [u8] {
        self.second
    }
}

/// Borrowed canonical signed INTEGER contents in two's-complement form.
pub struct DerIntegerRef<'a> {
    pub(super) bytes: &'a [u8],
}

impl<'a> DerIntegerRef<'a> {
    /// Return the canonical signed encoding, including any required sign octet.
    #[must_use]
    pub const fn as_bytes(&self) -> &'a [u8] {
        self.bytes
    }
}

/// Borrowed canonical BIT STRING contents and unused-bit count.
pub struct DerBitStringRef<'a> {
    pub(super) bytes: &'a [u8],
    pub(super) unused_bits: u8,
}

impl<'a> DerBitStringRef<'a> {
    /// Return content bytes, excluding the encoded unused-bit-count octet.
    #[must_use]
    pub const fn as_bytes(&self) -> &'a [u8] {
        self.bytes
    }

    /// Return the number of unused low bits in the last byte, from zero to seven.
    #[must_use]
    pub const fn unused_bits(&self) -> u8 {
        self.unused_bits
    }
}

/// Borrowed canonical OBJECT IDENTIFIER value octets.
///
/// Arcs remain canonical base-128 octets; decoding does not restrict them to a
/// machine integer or copy them into a fixed-size OID buffer. The document input
/// limit bounds validation work even for very large arcs.
pub struct DerOidRef<'a> {
    pub(super) bytes: &'a [u8],
}

impl<'a> DerOidRef<'a> {
    /// Return the validated base-128 OID contents without tag or length octets.
    #[must_use]
    pub const fn as_bytes(&self) -> &'a [u8] {
        self.bytes
    }
}
