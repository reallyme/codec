// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use zeroize::{ZeroizeOnDrop, Zeroizing};

use crate::PemLabel;

/// A decoded PEM document.
pub struct PemDocument {
    /// The exact label from the BEGIN/END boundaries.
    pub label: PemLabel,
    /// The decoded DER payload.
    pub der: Zeroizing<Vec<u8>>,
}

// The DER owner already wipes its full capacity on drop. Exposing the marker
// lets generic secret-owner code enforce that lifetime guarantee at compile
// time without making the document Clone or Debug.
impl ZeroizeOnDrop for PemDocument {}
