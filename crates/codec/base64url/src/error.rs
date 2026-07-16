// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

/// Error returned when base64url decoding fails.
#[derive(Debug, Error)]
pub enum Base64UrlError {
    /// The input was not valid base64url.
    #[error("invalid base64url")]
    Invalid,
}
