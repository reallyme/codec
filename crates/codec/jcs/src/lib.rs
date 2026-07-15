// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! JSON Canonicalization Scheme helpers.
//!
//! Object member ordering and finite floating-point formatting follow RFC
//! 8785. Integer values represented exactly by `serde_json` as `i64`/`u64`
//! are emitted verbatim, including values outside the ES6 safe-integer range;
//! callers that require strict I-JSON interoperability should reject those
//! integers before canonicalization.

mod canonicalize;
mod error;

pub use canonicalize::canonicalize_json;
pub use error::JcsError;

/// Maximum array/object nesting depth accepted by [`canonicalize_json`].
///
/// Defense in depth: `serde_json`'s parser already caps nesting, but a
/// caller may hand in a `Value` built by other means, so canonicalization
/// enforces its own bound rather than trusting the input's provenance.
pub const MAX_NESTING_DEPTH: usize = 128;
