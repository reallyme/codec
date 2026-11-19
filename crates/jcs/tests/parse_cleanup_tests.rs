// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Inspect initialized allocations while live, immediately before release.
#![cfg(not(target_arch = "wasm32"))]
#![allow(
    unsafe_code,
    clippy::unwrap_used,
    reason = "allocator regression harness"
)]

use codec_jcs::{canonicalize_json_text, JcsError};
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
use zeroize::Zeroizing;

const MARKER: &str = "cleanup-private-marker-18d5f";
static OBSERVE: AtomicBool = AtomicBool::new(false);
static UNWIPED: AtomicUsize = AtomicUsize::new(0);
struct ObserveRelease;
#[global_allocator]
static ALLOCATOR: ObserveRelease = ObserveRelease;

// SAFETY: Each request retains System's layout and ownership. Padding is
// initialized too, so inspecting a live allocation before deallocation is valid.
unsafe impl GlobalAlloc for ObserveRelease {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: Forward the caller's valid allocation layout unchanged.
        let pointer = unsafe { System.alloc(layout) };
        if !pointer.is_null() {
            // SAFETY: System returned layout.size() writable, exclusively owned bytes.
            unsafe { pointer.write_bytes(0, layout.size()) };
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        if OBSERVE.load(Relaxed) {
            // SAFETY: Inspect only initialized, still-live bytes with exclusive
            // deallocation ownership; never read memory after System releases it.
            let bytes = unsafe { std::slice::from_raw_parts(pointer, layout.size()) };
            if bytes
                .windows(MARKER.len())
                .any(|part| part == MARKER.as_bytes())
            {
                UNWIPED.fetch_add(1, Relaxed);
            }
        }
        // SAFETY: The pointer and original layout still belong to System.
        unsafe { System.dealloc(pointer, layout) };
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        let Ok(next) = Layout::from_size_align(size, layout.align()) else {
            return std::ptr::null_mut();
        };
        // SAFETY: The checked new layout is forwarded and initialized by alloc.
        let replacement = unsafe { self.alloc(next) };
        if !replacement.is_null() {
            // SAFETY: Distinct owned allocations do not overlap, and the copy
            // is bounded by both layouts. Retire the original layout afterward.
            unsafe { std::ptr::copy_nonoverlapping(pointer, replacement, size.min(layout.size())) };
            // SAFETY: This pointer/layout remains owned until deallocation.
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
fn escaped_keys_and_values_are_wiped_on_success_and_all_failure_paths() {
    assert!(releases(|| drop(std::hint::black_box(MARKER.to_owned()))) > 0);
    for (input, expected) in [
        (r#""cleanup-private-\u006darker-18d5f""#, None),
        (r#"{"cleanup-private-\u006darker-18d5f":0}"#, None),
        (
            r#"{"cleanup-private-\u006darker-18d5f":0,"cleanup-private-\u006darker-18d5f":1}"#,
            Some(JcsError::DuplicateProperty),
        ),
        (
            r#"["cleanup-private-\u006darker-18d5f",]"#,
            Some(JcsError::InvalidJson),
        ),
        (
            r#"{"cleanup-private-\u006darker-18d5f":]}"#,
            Some(JcsError::InvalidJson),
        ),
        (
            r#""cleanup-private-\u006darker-18d5f\uD800!""#,
            Some(JcsError::InvalidJson),
        ),
        (
            r#""cleanup-private-\u006darker-18d5f\q""#,
            Some(JcsError::InvalidJson),
        ),
        (
            r#""cleanup-private-\u006darker-18d5f" false"#,
            Some(JcsError::InvalidJson),
        ),
    ] {
        let mut error = None;
        let unwiped = releases(|| {
            // The returned canonical text belongs to the caller; this harness
            // wipes it so only parser and canonicalizer cleanup is measured.
            match canonicalize_json_text(input) {
                Ok(output) => drop(Zeroizing::new(output)),
                Err(reason) => error = Some(reason),
            }
        });
        assert_eq!(unwiped, 0);
        assert_eq!(error, expected);
    }
}
