// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

use crate::base58btc::Base58Error;

/// Error returned when decoding a multibase string fails.
#[derive(Debug, Error)]
pub enum MultibaseError {
    /// The input is too short to contain a multibase prefix and payload.
    #[error("invalid multibase string: too short")]
    TooShort,

    // Deliberately carries no payload: the rejected prefix comes from
    // untrusted input, and error variants must not echo it back.
    /// The leading multibase prefix denotes an unsupported base encoding.
    #[error("unsupported multibase prefix")]
    UnsupportedPrefix,

    /// The base58btc payload failed to decode.
    #[error(transparent)]
    Base58(#[from] Base58Error),

    /// The base64url payload failed to decode.
    #[error("invalid base64url")]
    Base64Url,
}
