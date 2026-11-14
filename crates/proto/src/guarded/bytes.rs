// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::{fmt, ops::Deref};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

/// Owned protobuf bytes. Drop wipes live bytes and the entire spare capacity.
///
/// Clone creates an independent wipe owner. Mutable access cannot grow the
/// allocation, so ordinary Vec growth cannot release an unwiped secret copy.
#[derive(Default, Eq)]
pub struct SecretBytes(Vec<u8>);

impl SecretBytes {
    /// Borrow the bytes without transferring ownership.
    #[must_use]
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Mutate the current bytes without exposing Vec growth or replacement.
    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.0
    }

    /// Inspect the allocated capacity without copying its contents.
    #[must_use]
    pub fn capacity(&self) -> usize {
        self.0.capacity()
    }

    /// Wipe the complete allocation while retaining its capacity.
    pub fn clear(&mut self) {
        self.zeroize();
    }

    /// Transfer allocation ownership to an explicit zeroizing transport owner.
    #[must_use]
    pub fn into_zeroizing(mut self) -> Zeroizing<Vec<u8>> {
        Zeroizing::new(core::mem::take(&mut self.0))
    }
}

impl From<Vec<u8>> for SecretBytes {
    fn from(value: Vec<u8>) -> Self {
        Self(value)
    }
}

impl Clone for SecretBytes {
    fn clone(&self) -> Self {
        // The owner exists before allocation or copying. Clone is infallible
        // in Buffa's representation contract, just like Vec's own Clone.
        let mut copy = Self::default();
        if copy.0.try_reserve_exact(self.0.len()).is_err() {
            std::alloc::handle_alloc_error(std::alloc::Layout::new::<u8>());
        }
        copy.0.extend_from_slice(&self.0);
        copy
    }
}

impl<T: AsRef<[u8]> + ?Sized> PartialEq<T> for SecretBytes {
    fn eq(&self, other: &T) -> bool {
        self.as_slice() == other.as_ref()
    }
}

impl Deref for SecretBytes {
    type Target = [u8];
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AsRef<[u8]> for SecretBytes {
    fn as_ref(&self) -> &[u8] {
        self.as_slice()
    }
}

impl fmt::Debug for SecretBytes {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretBytes([REDACTED])")
    }
}

impl Zeroize for SecretBytes {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl Drop for SecretBytes {
    fn drop(&mut self) {
        self.zeroize();
    }
}
impl ZeroizeOnDrop for SecretBytes {}
