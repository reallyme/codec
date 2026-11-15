// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use crate::jni_capacity::{probed_output_capacity, validate_output_copy, CopyFailure, OutputKind};
use crate::status::{CODEC_BUFFER_TOO_SMALL, CODEC_INTERNAL_ERROR, CODEC_OK};
use codec_proto::{MAX_CODEC_PROTO_MESSAGE_BYTES, MAX_CODEC_PROVIDER_OUTPUT_BYTES};

#[test]
fn generic_output_probe_is_strict_and_bounded() {
    let generic = OutputKind::Generic;
    assert_eq!(probed_output_capacity(generic, CODEC_OK, 0), Some(0));
    assert_eq!(
        probed_output_capacity(generic, CODEC_BUFFER_TOO_SMALL, 1),
        Some(1)
    );
    assert_eq!(probed_output_capacity(generic, CODEC_OK, 1), None);
    assert_eq!(
        probed_output_capacity(generic, CODEC_BUFFER_TOO_SMALL, 0),
        None
    );
    assert_eq!(
        probed_output_capacity(
            generic,
            CODEC_BUFFER_TOO_SMALL,
            MAX_CODEC_PROVIDER_OUTPUT_BYTES + 1
        ),
        None
    );
    assert_eq!(
        probed_output_capacity(generic, CODEC_INTERNAL_ERROR, 1),
        None
    );
}

#[test]
fn operation_probe_and_copy_require_independent_nonempty_receipts() {
    let protobuf = OutputKind::Protobuf;
    assert_eq!(probed_output_capacity(protobuf, CODEC_OK, 0), None);
    let capacity = probed_output_capacity(
        protobuf,
        CODEC_BUFFER_TOO_SMALL,
        MAX_CODEC_PROTO_MESSAGE_BYTES,
    );
    assert_eq!(capacity, Some(MAX_CODEC_PROTO_MESSAGE_BYTES));
    assert_eq!(
        probed_output_capacity(
            protobuf,
            CODEC_BUFFER_TOO_SMALL,
            MAX_CODEC_PROTO_MESSAGE_BYTES + 1
        ),
        None
    );
    // If the copy call leaves its length output untouched, the zero
    // initialization cannot accidentally reuse the successful probe.
    assert_eq!(
        validate_output_copy(CODEC_OK, 0, MAX_CODEC_PROTO_MESSAGE_BYTES),
        Err(CopyFailure::LengthMismatch)
    );
    assert_eq!(
        validate_output_copy(
            CODEC_OK,
            MAX_CODEC_PROTO_MESSAGE_BYTES,
            MAX_CODEC_PROTO_MESSAGE_BYTES
        ),
        Ok(())
    );
    assert_eq!(
        validate_output_copy(CODEC_INTERNAL_ERROR, 0, MAX_CODEC_PROTO_MESSAGE_BYTES),
        Err(CopyFailure::Status(CODEC_INTERNAL_ERROR))
    );
}
