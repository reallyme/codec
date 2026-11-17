// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Observe live initialized allocations before their allocator releases them.

#![cfg(not(target_arch = "wasm32"))]
#![allow(unsafe_code, clippy::unwrap_used)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};

const MARKER: &str = "private-cleanup-regression-83bd5f";
const NUMERIC_MARKER: &str = "773495662153840927";
static OBSERVE: AtomicBool = AtomicBool::new(false);
static UNWIPED: AtomicUsize = AtomicUsize::new(0);
struct ObserveRelease;
#[global_allocator]
static ALLOCATOR: ObserveRelease = ObserveRelease;

// SAFETY: Requests use System with their unchanged layout and ownership.
// Every byte is initialized on allocation, including padding. Inspection occurs
// only while the allocation is live and exclusively owned for deallocation.
unsafe impl GlobalAlloc for ObserveRelease {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: Forward the caller's valid allocation layout unchanged.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            // SAFETY: System returned this writable allocation of layout.size().
            unsafe { pointer.write_bytes(0, layout.size()) };
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        if OBSERVE.load(Relaxed) {
            // SAFETY: This allocation is live, initialized, and exclusively
            // owned for deallocation; no bytes are inspected after release.
            let bytes = unsafe { std::slice::from_raw_parts(pointer, layout.size()) };
            if bytes
                .windows(MARKER.len())
                .any(|window| window == MARKER.as_bytes())
                || bytes
                    .windows(NUMERIC_MARKER.len())
                    .any(|window| window == NUMERIC_MARKER.as_bytes())
            {
                UNWIPED.fetch_add(1, Relaxed);
            }
        }
        // SAFETY: The original pointer/layout pair belongs to System.
        unsafe { System.dealloc(pointer, layout) };
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let Ok(next) = Layout::from_size_align(size, layout.align()) else {
            return std::ptr::null_mut();
        };
        // SAFETY: The new layout is validated above, and alloc initializes it.
        let replacement = unsafe { self.alloc(next) };
        if !replacement.is_null() {
            // SAFETY: Distinct live allocations do not overlap. Copy length
            // is bounded by both layouts; the old allocation remains owned.
            unsafe { std::ptr::copy_nonoverlapping(pointer, replacement, size.min(layout.size())) };
            // SAFETY: Retire the original allocation with its original layout.
            unsafe { self.dealloc(pointer, layout) };
        }
        replacement
    }
}

fn releases(operation: impl FnOnce()) -> usize {
    UNWIPED.store(0, Relaxed);
    OBSERVE.store(true, Relaxed);
    operation();
    OBSERVE.store(false, Relaxed);
    UNWIPED.load(Relaxed)
}

#[test]
fn bundle_failures_clear_prior_blocks_and_partial_decoding() {
    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use codec_pem::{
        decode_pem_bundle, CodecPemBundleDecodeOptions, CodecPemSyntaxProfile, PemError,
    };
    assert!(releases(|| drop(std::hint::black_box(MARKER.to_owned()))) > 0);
    let body = STANDARD.encode(MARKER.as_bytes());
    let good = format!("-----BEGIN {MARKER}-----\n{body}\n-----END {MARKER}-----\n");
    let options = CodecPemBundleDecodeOptions::default()
        .with_syntax_profile(CodecPemSyntaxProfile::CODEC_PEM_SYNTAX_PROFILE_COMPATIBLE);
    let mut result = None;
    for (input, expected) in [
        (good.clone(), None),
        (good.repeat(16), None),
        (
            format!("{good}-----BEGIN A-----!-----END A-----"),
            Some(PemError::InvalidBase64),
        ),
        (
            format!("{good}-----BEGIN A-----{body}!-----END A-----"),
            Some(PemError::InvalidBase64),
        ),
        (
            format!("{good}-----BEGIN A-----Broken header\n\n{body}-----END A-----"),
            Some(PemError::InvalidBody),
        ),
        (
            format!("{good}-----BEGIN A-----{body}-----END B-----"),
            Some(PemError::LabelMismatch),
        ),
    ] {
        assert_eq!(
            releases(|| {
                result = decode_pem_bundle(input.as_bytes(), &options).err();
            }),
            0
        );
        assert_eq!(result, expected);
    }
    let full = good.repeat(3);
    assert_eq!(
        releases(|| {
            result = decode_pem_bundle(full.as_bytes(), &options.clone().with_max_blocks(2)).err();
        }),
        0
    );
    assert_eq!(result, Some(PemError::ResourceLimitExceeded));
    let input = format!(r#"{{"label":{NUMERIC_MARKER},"der":""}}"#);
    assert_eq!(
        releases(|| {
            assert!(serde_json::from_str::<codec_pem::CodecPemBundleBlock>(&input).is_err());
        }),
        0
    );
    let input = format!(r#"{{"label":"CERTIFICATE","der":{NUMERIC_MARKER}}}"#);
    assert_eq!(
        releases(|| {
            assert!(serde_json::from_str::<codec_pem::CodecPemBundleBlock>(&input).is_err());
        }),
        0
    );
}
