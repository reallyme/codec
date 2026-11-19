// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! JSON Canonicalization Scheme helpers.
//!
//! Object member ordering and finite floating-point formatting follow RFC
//! 8785. The raw-text API rejects duplicate object member names and malformed
//! JSON. Integer tokens that `serde_json` retains exactly as `i64` or `u64`
//! are additionally rejected outside `[-(2^53)+1, (2^53)-1]`; integer-valued
//! binary64 numbers are subject to the same local policy. Non-integer binary64
//! numbers follow RFC 8785's ECMAScript rounding semantics.
//! If dependency feature unification enables `serde_json/arbitrary_precision`,
//! the raw-text entry point fails closed with
//! [`JcsError::UnsupportedNumberRepresentation`]. The feature changes the
//! dependency's number visitor contract. That feature combination remains
//! unsupported; use a separately compiled JCS lane.
//! The raw-text parser decodes strings into wiping owners and parses numbers
//! without a heap scratch buffer. Returned canonical text belongs to the caller.

mod canonicalize;
mod error;
mod parse_json;
mod read_json_string;
mod sensitive_json_value;

pub use canonicalize::{canonicalize_json_text, canonicalize_trusted_json_value};
pub use error::JcsError;

mod limits;
pub use limits::MAX_NESTING_DEPTH;
