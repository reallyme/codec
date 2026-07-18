// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

/// Signed 32-bit status code returned by every C ABI entry point
/// (`rm_codec_status_t` at the boundary); `0` is success and negatives are
/// deterministic, non-PII error classes.
pub type CodecStatus = i32;

/// Operation succeeded (`0`).
pub const CODEC_OK: CodecStatus = 0;
/// A pointer/length argument was invalid, null where required, or malformed (`-1`).
pub const CODEC_INVALID_ARGUMENT: CodecStatus = -1;
/// The caller-provided output buffer was too small for the result (`-5`).
pub const CODEC_BUFFER_TOO_SMALL: CodecStatus = -5;
/// An unexpected internal failure, including a caught panic (`-128`).
pub const CODEC_INTERNAL_ERROR: CodecStatus = -128;
