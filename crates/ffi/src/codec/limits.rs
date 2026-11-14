// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

/// Maximum aggregate caller-controlled input accepted by the generic C ABI.
///
/// This check runs before decoding or copying so every operation, including
/// future dispatch additions, inherits a finite allocation budget.
pub(crate) const MAX_CODEC_FFI_INPUT_BYTES: usize = codec_proto::MAX_CODEC_PROVIDER_INPUT_BYTES;

/// Maximum generic result accepted by SDK adapters after an FFI size probe.
///
/// Structured outputs can expand substantially at SDK boundaries, so this is
/// deliberately larger than the input budget. Keeping it finite prevents a
/// corrupted or mismatched provider from turning the two-pass ABI into an
/// unbounded allocation request in Swift or JNI callers.
pub(crate) const MAX_CODEC_FFI_OUTPUT_BYTES: usize = codec_proto::MAX_CODEC_PROVIDER_OUTPUT_BYTES;
