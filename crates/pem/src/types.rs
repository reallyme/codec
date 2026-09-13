// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use zeroize::Zeroizing;

use crate::PemLabel;

/// A decoded PEM document.
pub struct PemDocument {
    /// The exact label from the BEGIN/END boundaries.
    pub label: PemLabel,
    /// The decoded DER payload.
    pub der: Zeroizing<Vec<u8>>,
}
