// SPDX-FileCopyrightText: 2026 ReallyMe LLC
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
//! deserializer's number visitor contract, so proceeding could produce
//! noncanonical bytes. Integrators that require that feature should use a
//! separately compiled JCS lane until an independent tokenizer is available.

mod canonicalize;
mod error;
mod parse_json;

pub use canonicalize::{canonicalize_json_text, canonicalize_trusted_json_value};
pub use error::JcsError;

/// Maximum array/object nesting depth accepted by both JCS entry points.
///
/// The text parser applies this guard after disabling serde_json's recursion
/// limit, and trusted values apply it during canonicalization.
pub const MAX_NESTING_DEPTH: usize = 128;
