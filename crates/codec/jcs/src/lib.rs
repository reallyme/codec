// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! JSON Canonicalization Scheme helpers.
//!
//! Object member ordering and finite floating-point formatting follow RFC
//! 8785. The raw-text API rejects duplicate object member names and malformed
//! JSON. Integer tokens that `serde_json` retains exactly as `i64` or `u64`
//! are additionally rejected outside `[-(2^53)+1, (2^53)-1]`; numbers parsed
//! through binary64, including decimal/exponent forms and magnitudes above
//! `u64::MAX`, follow RFC 8785's ECMAScript rounding semantics.

mod canonicalize;
mod error;
mod parse_json;

// Preserve the pre-0.2 symbol for downstream migration while ensuring every
// new call site sees the deprecation on use. The allowance is intentionally
// scoped to this compatibility re-export.
#[allow(deprecated)]
pub use canonicalize::canonicalize_json;
pub use canonicalize::{canonicalize_json_text, canonicalize_trusted_json_value};
pub use error::JcsError;

/// Maximum array/object nesting depth accepted by [`canonicalize_trusted_json_value`].
///
/// Defense in depth: `serde_json`'s parser already caps nesting, but a
/// caller may hand in a `Value` built by other means, so canonicalization
/// enforces its own bound rather than trusting the input's provenance.
pub const MAX_NESTING_DEPTH: usize = 128;
