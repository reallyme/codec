// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::status::{CodecStatus, CODEC_BUFFER_TOO_SMALL, CODEC_OK};
use codec_proto::{MAX_CODEC_PROTO_MESSAGE_BYTES, MAX_CODEC_PROVIDER_OUTPUT_BYTES};

#[derive(Clone, Copy)]
pub(crate) enum OutputKind {
    Generic,
    Protobuf,
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) enum CopyFailure {
    Status(CodecStatus),
    LengthMismatch,
}

pub(crate) fn checked_input_total(used: usize, length: usize, maximum: usize) -> Option<usize> {
    let total = used.checked_add(length)?;
    (total <= maximum).then_some(total)
}

pub(crate) fn probed_output_capacity(
    kind: OutputKind,
    status: CodecStatus,
    produced_len: usize,
) -> Option<usize> {
    let maximum = match kind {
        OutputKind::Generic if status == CODEC_OK && produced_len == 0 => return Some(0),
        OutputKind::Generic => MAX_CODEC_PROVIDER_OUTPUT_BYTES,
        OutputKind::Protobuf => MAX_CODEC_PROTO_MESSAGE_BYTES,
    };
    (status == CODEC_BUFFER_TOO_SMALL && produced_len != 0 && produced_len <= maximum)
        .then_some(produced_len)
}

pub(crate) fn validate_output_copy(
    status: CodecStatus,
    copied_len: usize,
    capacity: usize,
) -> Result<(), CopyFailure> {
    if status != CODEC_OK {
        return Err(CopyFailure::Status(status));
    }
    if copied_len != capacity {
        return Err(CopyFailure::LengthMismatch);
    }
    Ok(())
}

#[cfg(kani)]
#[path = "jni_capacity_proof_tests.rs"]
mod proofs;
