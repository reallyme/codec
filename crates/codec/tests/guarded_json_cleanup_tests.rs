// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Observe live initialized allocations before their allocator releases them.

#![cfg(all(feature = "json", not(target_arch = "wasm32")))]
#![allow(unsafe_code, clippy::unwrap_used)]

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering::Relaxed};
use std::sync::Mutex;

const MARKER: &str = "guarded-json-cleanup-regression-83bd5f";
static OBSERVE: AtomicBool = AtomicBool::new(false);
static UNWIPED: AtomicUsize = AtomicUsize::new(0);
// The observer is global: one fixture must not reset another observation window.
static CASE_LOCK: Mutex<()> = Mutex::new(());
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
fn escaped_string_payloads_and_keys_are_wiped() {
    let _case = CASE_LOCK.lock().unwrap();
    use reallyme_codec::json;
    assert!(releases(|| drop(std::hint::black_box(MARKER.to_owned()))) > 0);
    for (input, accepted) in [
        (format!(r#""{MARKER}\u0000""#), true),
        (format!(r#"{{"{MARKER}\n":"value"}}"#), true),
        (format!(r#"{{"value":"{MARKER}\n","nested":["x"]}}"#), true),
        (format!(r#"{{"value":"{MARKER}\n","nested": }}"#), false),
        (format!(r#"{{"value":"{MARKER}\n","bad":"\uD800"}}"#), false),
        (format!(r#"{{"{MARKER}\n":1,"{MARKER}\u000a":2}}"#), false),
    ] {
        assert_eq!(
            releases(|| {
                let result = json::decode(input.as_bytes(), None);
                assert_eq!(result.is_ok(), accepted);
                drop(result);
            }),
            0
        );
    }
}

#[test]
fn typed_projection_wipes_success_and_late_failure() {
    let _case = CASE_LOCK.lock().unwrap();
    use codec_proto::generated::proto::reallyme::codec::v1::CodecTextResult;
    use reallyme_codec::json;
    for (input, accepted) in [
        (format!(r#"{{"value":"{MARKER}\n"}}"#), true),
        (format!(r#"{{"value":"{MARKER}\n","unknown":true}}"#), false),
        (format!(r#"{{"value":"{MARKER}\n","value":null}}"#), false),
        (format!(r#"{{"value":"{MARKER}\uD800"}}"#), false),
    ] {
        assert_eq!(
            releases(|| {
                let result = json::from_slice::<CodecTextResult>(input.as_bytes(), None);
                assert_eq!(result.is_ok(), accepted);
                drop(result);
            }),
            0
        );
    }
}

#[test]
#[cfg(feature = "base64")]
fn bounded_base64_owner_wipes_success_and_late_alphabet_failure() {
    let _case = CASE_LOCK.lock().unwrap();
    use reallyme_codec::base64::{
        base64_to_bytes_with_limits, bytes_to_base64, Base64DecodeLimits,
    };
    let input = format!("{MARKER}{}", "x".repeat(16));
    let encoded = bytes_to_base64(input.as_bytes());
    let mut malformed = encoded.clone();
    malformed.replace_range(malformed.len() - 4.., "!!!!");
    for (input, accepted) in [(&encoded, true), (&malformed, false)] {
        assert_eq!(
            releases(|| {
                let result = base64_to_bytes_with_limits(input, &Base64DecodeLimits::default());
                assert_eq!(result.is_ok(), accepted);
                drop(result);
            }),
            0
        );
    }
}

#[test]
fn protobuf_string_and_byte_serialization_copies_are_wiped() {
    let _case = CASE_LOCK.lock().unwrap();
    use codec_proto::generated::proto::reallyme::codec::v1::{CodecBytesResult, CodecTextResult};
    assert_eq!(
        releases(|| {
            let text = CodecTextResult {
                value: MARKER.into(),
            };
            let wire = codec_proto::encode_protobuf(&text).unwrap();
            let decoded: CodecTextResult = codec_proto::decode_protobuf(&wire).unwrap();
            assert_eq!(decoded, text);
            drop((decoded, wire, text));
            let bytes = CodecBytesResult {
                value: MARKER.as_bytes().to_vec().into(),
            };
            let wire = codec_proto::encode_protobuf(&bytes).unwrap();
            let decoded: CodecBytesResult = codec_proto::decode_protobuf(&wire).unwrap();
            assert_eq!(decoded, bytes);
            drop((decoded, wire, bytes));
        }),
        0
    );
}
