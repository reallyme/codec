// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{probed_output_capacity, MAX_CODEC_FFI_OUTPUT_BYTES};
use crate::status::{CODEC_BUFFER_TOO_SMALL, CODEC_INTERNAL_ERROR, CODEC_OK};

#[test]
fn generic_output_probe_is_strict_and_bounded() {
    assert_eq!(probed_output_capacity(CODEC_OK, 0), Some(0));
    assert_eq!(probed_output_capacity(CODEC_BUFFER_TOO_SMALL, 1), Some(1));
    assert_eq!(probed_output_capacity(CODEC_OK, 1), None);
    assert_eq!(probed_output_capacity(CODEC_BUFFER_TOO_SMALL, 0), None);
    assert_eq!(
        probed_output_capacity(CODEC_BUFFER_TOO_SMALL, MAX_CODEC_FFI_OUTPUT_BYTES + 1),
        None
    );
    assert_eq!(probed_output_capacity(CODEC_INTERNAL_ERROR, 1), None);
}
