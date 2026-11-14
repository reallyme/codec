// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

/// Maximum array/object nesting depth accepted by both JCS entry points.
///
/// The text parser applies this guard after disabling serde_json's recursion
/// limit, and trusted values apply it during canonicalization.
pub const MAX_NESTING_DEPTH: usize = 128;
