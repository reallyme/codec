// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(clippy::panic)]

use super::{ffi_guard, inside_native_boundary, with_redacted_panic_hook};
use crate::status::{CODEC_INTERNAL_ERROR, CODEC_OK};

#[test]
fn panic_is_mapped_to_internal_error() {
    assert_eq!(
        ffi_guard(|| panic!("test-only panic")),
        CODEC_INTERNAL_ERROR
    );
}

#[test]
fn redaction_scope_is_thread_local_and_restored() {
    assert!(!inside_native_boundary());
    assert_eq!(
        with_redacted_panic_hook(|| {
            assert!(inside_native_boundary());
            CODEC_OK
        }),
        CODEC_OK
    );
    assert!(!inside_native_boundary());
}
