// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::{fmt, ops::Deref};
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

/// Owned protobuf UTF-8 text with redacted diagnostics and allocation cleanup.
/// This is a storage owner, not a validated application identifier.
#[derive(Default, Eq)]
pub struct SecretString(String);

impl SecretString {
    /// Borrow text without transferring its allocation.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Wipe the complete allocation while retaining its capacity.
    pub fn clear(&mut self) {
        self.zeroize();
    }

    /// Transfer allocation ownership to an explicit zeroizing transport owner.
    #[must_use]
    pub fn into_zeroizing(mut self) -> Zeroizing<String> {
        Zeroizing::new(core::mem::take(&mut self.0))
    }
}

impl From<String> for SecretString {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for SecretString {
    fn from(value: &str) -> Self {
        let mut copy = Self::default();
        if copy.0.try_reserve_exact(value.len()).is_err() {
            std::alloc::handle_alloc_error(std::alloc::Layout::new::<u8>());
        }
        copy.0.push_str(value);
        copy
    }
}

impl Clone for SecretString {
    fn clone(&self) -> Self {
        Self::from(self.as_str())
    }
}

impl<T: AsRef<str> + ?Sized> PartialEq<T> for SecretString {
    fn eq(&self, other: &T) -> bool {
        self.as_str() == other.as_ref()
    }
}

impl Deref for SecretString {
    type Target = str;
    fn deref(&self) -> &str {
        &self.0
    }
}

impl AsRef<str> for SecretString {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretString([REDACTED])")
    }
}

impl Zeroize for SecretString {
    fn zeroize(&mut self) {
        self.0.zeroize();
    }
}

impl Drop for SecretString {
    fn drop(&mut self) {
        self.zeroize();
    }
}
impl ZeroizeOnDrop for SecretString {}
