// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

/// Error returned when base64 decoding fails.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Base64Error {
    /// The input was not valid base64.
    #[error("invalid base64")]
    Invalid,
}
