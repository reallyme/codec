// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::super::{rm_codec_process, CODEC_MULTICODEC_STRIP_PREFIX};
use crate::status::{CODEC_INVALID_MULTICODEC_PREFIX, CODEC_OK};

fn assert_strip_status(prefix: &[u8], payload: &[u8], accepted: bool) {
    let mut input = prefix.to_vec();
    input.extend_from_slice(payload);
    let mut output = vec![0xa5; payload.len().max(1)];
    let mut produced_len = usize::MAX;
    // SAFETY: Input, output, and the aligned length slot are disjoint, live
    // caller-owned storage. Each pointer covers its declared length. The other
    // two inputs are absent null/zero pairs; no ownership crosses the boundary.
    let status = unsafe {
        rm_codec_process(
            CODEC_MULTICODEC_STRIP_PREFIX,
            input.as_ptr(),
            input.len(),
            core::ptr::null(),
            0,
            core::ptr::null(),
            0,
            output.as_mut_ptr(),
            output.len(),
            &mut produced_len,
        )
    };
    if accepted {
        assert_eq!(status, CODEC_OK);
        assert_eq!(produced_len, payload.len());
        assert_eq!(output.get(..produced_len), Some(payload));
    } else {
        assert_eq!(status, CODEC_INVALID_MULTICODEC_PREFIX);
        assert_eq!(produced_len, 0);
        assert!(output.iter().all(|&byte| byte == 0xa5));
    }
}

#[test]
fn scalar_ffi_strip_rejects_bad_ec_tags_without_touching_output() {
    for (prefix, length) in [
        ([0x80, 0x24], 33),
        ([0x81, 0x24], 49),
        ([0x82, 0x24], 67),
        ([0xe7, 0x01], 33),
    ] {
        for tag in [0x02, 0x03, 0x04] {
            let mut payload = vec![0; length];
            payload[0] = tag;
            assert_strip_status(&prefix, &payload, tag != 0x04);
        }
    }
}

#[test]
fn scalar_ffi_strip_enforces_rsa_payload_cap_without_touching_output() {
    for length in [0, 1, 4_096, 4_097, 600_000] {
        assert_strip_status(
            &[0x85, 0x24],
            &vec![0x30; length],
            (1..=4_096).contains(&length),
        );
    }
}
