// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use half::f16;
use zeroize::{Zeroize, ZeroizeOnDrop};

use super::error::{error, CborError, ErrorReason};
use super::profile::FloatPolicy;

/// IEEE 754 representation width.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FloatWidth {
    /// Binary16.
    Binary16,
    /// Binary32.
    Binary32,
    /// Binary64.
    Binary64,
}

/// Owned floating-point bits, zeroized on drop without content-printing traits.
///
/// NaN payloads and signs are retained. Shortening a NaN preserves its
/// significand by right-side zero extension, rather than canonicalizing it.
pub struct Float {
    bits: FloatBits,
}

enum FloatBits {
    Binary16(u16),
    Binary32(u32),
    Binary64(u64),
}

impl Float {
    /// Preserve all binary16 bits.
    #[must_use]
    pub const fn binary16(bits: u16) -> Self {
        Self {
            bits: FloatBits::Binary16(bits),
        }
    }
    /// Preserve all binary32 bits.
    #[must_use]
    pub const fn binary32(bits: u32) -> Self {
        Self {
            bits: FloatBits::Binary32(bits),
        }
    }
    /// Preserve all binary64 bits.
    #[must_use]
    pub const fn binary64(bits: u64) -> Self {
        Self {
            bits: FloatBits::Binary64(bits),
        }
    }
    /// Preserve a Rust binary64 value's bits.
    #[must_use]
    pub fn from_f64(value: f64) -> Self {
        Self::binary64(value.to_bits())
    }
    /// Inspect representation width.
    #[must_use]
    pub const fn width(&self) -> FloatWidth {
        match self.bits {
            FloatBits::Binary16(_) => FloatWidth::Binary16,
            FloatBits::Binary32(_) => FloatWidth::Binary32,
            FloatBits::Binary64(_) => FloatWidth::Binary64,
        }
    }
    /// Explicitly extract the original bits; callers own any resulting copy.
    #[must_use]
    pub fn bits(&self) -> u64 {
        match self.bits {
            FloatBits::Binary16(value) => u64::from(value),
            FloatBits::Binary32(value) => u64::from(value),
            FloatBits::Binary64(value) => value,
        }
    }

    fn layout(&self) -> (u32, u64, u32) {
        match self.width() {
            FloatWidth::Binary16 => (10, 0x1f, 15),
            FloatWidth::Binary32 => (23, 0xff, 31),
            FloatWidth::Binary64 => (52, 0x7ff, 63),
        }
    }

    fn fraction(&self) -> u64 {
        let (fraction_bits, _, _) = self.layout();
        self.bits() & ((1_u64 << fraction_bits) - 1)
    }

    /// Inspect whether the represented value is a NaN.
    #[must_use]
    pub fn is_nan(&self) -> bool {
        !self.is_finite() && self.fraction() != 0
    }
    /// Inspect whether the represented value is finite.
    #[must_use]
    pub fn is_finite(&self) -> bool {
        let (fraction_bits, exponent_mask, _) = self.layout();
        (self.bits() >> fraction_bits) & exponent_mask != exponent_mask
    }

    /// Convert without losing a NaN's sign or significand.
    #[must_use]
    pub fn to_f64(&self) -> f64 {
        if self.is_nan() {
            let (fraction_bits, _, sign_bit) = self.layout();
            let sign = (self.bits() >> sign_bit) << 63;
            return f64::from_bits(
                sign | 0x7ff0_0000_0000_0000 | (self.fraction() << (52 - fraction_bits)),
            );
        }
        match self.bits {
            // The portable conversion avoids host FP16 assembly and gives
            // native, interpreted, and WASM lanes the same bit-level path.
            FloatBits::Binary16(value) => f16::from_bits(value).to_f64_const(),
            FloatBits::Binary32(value) => f64::from(f32::from_bits(value)),
            FloatBits::Binary64(value) => f64::from_bits(value),
        }
    }

    pub(super) fn check(&self, policy: FloatPolicy) -> Result<(), CborError> {
        match policy {
            FloatPolicy::Reject => Err(error(ErrorReason::FloatNotAllowed)),
            FloatPolicy::FiniteOnly if !self.is_finite() => Err(error(ErrorReason::NonFiniteFloat)),
            _ => Ok(()),
        }
    }

    pub(super) fn shortest(&self) -> FloatWidth {
        if self.is_nan() {
            let (fraction_bits, _, _) = self.layout();
            let normalized = self.fraction() << (64 - fraction_bits);
            if normalized & ((1_u64 << 54) - 1) == 0 {
                return FloatWidth::Binary16;
            }
            if normalized & ((1_u64 << 41) - 1) == 0 {
                return FloatWidth::Binary32;
            }
            return FloatWidth::Binary64;
        }
        let value = self.to_f64();
        if f16::from_f64_const(value).to_f64_const().to_bits() == value.to_bits() {
            return FloatWidth::Binary16;
        }
        // This narrowing is intentional; the round trip proves exactness and
        // includes the sign of zero. It is never used for lengths or offsets.
        if f64::from(value as f32).to_bits() == value.to_bits() {
            return FloatWidth::Binary32;
        }
        FloatWidth::Binary64
    }

    pub(super) fn at_width(&self, width: FloatWidth) -> Result<u64, CborError> {
        if width == self.width() {
            return Ok(self.bits());
        }
        if self.is_nan() {
            let (source_bits, _, sign_bit) = self.layout();
            let (target_bits, exponent, target_sign) = match width {
                FloatWidth::Binary16 => (10, 0x1f_u64, 15),
                FloatWidth::Binary32 => (23, 0xff, 31),
                FloatWidth::Binary64 => (52, 0x7ff, 63),
            };
            let normalized = self.fraction() << (64 - source_bits);
            let fraction = normalized >> (64 - target_bits);
            if fraction == 0 || fraction << (64 - target_bits) != normalized {
                return Err(error(ErrorReason::NonPreferredFloat));
            }
            return Ok(((self.bits() >> sign_bit) << target_sign)
                | (exponent << target_bits)
                | fraction);
        }
        let value = self.to_f64();
        match width {
            FloatWidth::Binary16 => Ok(u64::from(f16::from_f64_const(value).to_bits())),
            FloatWidth::Binary32 => Ok(u64::from((value as f32).to_bits())),
            FloatWidth::Binary64 => Ok(value.to_bits()),
        }
    }

    pub(super) fn equivalent(&self, other: &Self) -> bool {
        if self.is_nan() || other.is_nan() {
            if !self.is_nan() || !other.is_nan() {
                return false;
            }
            let (left, _, _) = self.layout();
            let (right, _, _) = other.layout();
            return self.fraction() << (64 - left) == other.fraction() << (64 - right);
        }
        self.to_f64() == other.to_f64()
    }
}

impl Zeroize for Float {
    fn zeroize(&mut self) {
        match &mut self.bits {
            FloatBits::Binary16(value) => value.zeroize(),
            FloatBits::Binary32(value) => value.zeroize(),
            FloatBits::Binary64(value) => value.zeroize(),
        }
    }
}
impl Drop for Float {
    fn drop(&mut self) {
        self.zeroize();
    }
}
impl ZeroizeOnDrop for Float {}
