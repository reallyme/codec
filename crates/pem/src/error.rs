// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use thiserror::Error;

/// Error returned when PEM text armor cannot be parsed or encoded.
#[derive(Debug, Error, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PemError {
    /// The input text exceeded the configured maximum size.
    #[error("pem: input too large")]
    InputTooLarge,
    /// The decoded DER body exceeded the configured maximum size.
    #[error("pem: der body too large")]
    DerTooLarge,
    /// The PEM document did not contain a BEGIN boundary.
    #[error("pem: missing begin boundary")]
    MissingBegin,
    /// The PEM document did not contain a matching END boundary.
    #[error("pem: missing end boundary")]
    MissingEnd,
    /// The BEGIN and END labels did not match.
    #[error("pem: label mismatch")]
    LabelMismatch,
    /// The label is not part of the configured allowlist.
    #[error("pem: unsupported label")]
    UnsupportedLabel,
    /// A boundary line was malformed.
    #[error("pem: invalid boundary")]
    InvalidBoundary,
    /// The base64 body was malformed.
    #[error("pem: invalid base64")]
    InvalidBase64,
    /// The body was empty or contained invalid armor data.
    #[error("pem: invalid body")]
    InvalidBody,
    /// The encode options were invalid.
    #[error("pem: invalid options")]
    InvalidOptions,
}
