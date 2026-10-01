// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use codec_proto::generated::proto::reallyme::codec::v1::CodecErrorReason;

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

/// Canonical lowercase hex was violated; sourced from the wire reason enum.
pub const CODEC_NON_CANONICAL_HEX: CodecStatus =
    -(CodecErrorReason::CODEC_ERROR_REASON_BASE_NON_CANONICAL_HEX as i32);
/// A multicodec prefix is not supported by the registered codec set.
pub const CODEC_INVALID_MULTICODEC_PREFIX: CodecStatus =
    -(CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_INVALID_MULTICODEC_PREFIX as i32);
/// A multicodec name is not supported by the registered codec set.
pub const CODEC_UNKNOWN_MULTICODEC: CodecStatus =
    -(CodecErrorReason::CODEC_ERROR_REASON_MULTIFORMAT_UNKNOWN_MULTICODEC as i32);
/// JSON text is valid but violates canonical object-member rules.
pub const CODEC_NON_CANONICAL_JSON: CodecStatus =
    -(CodecErrorReason::CODEC_ERROR_REASON_CANONICAL_NON_CANONICAL_JSON as i32);
