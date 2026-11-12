// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Measure allocations on the calling thread. The test harness can allocate on
//! other threads even when this binary contains only one test.
#![allow(missing_docs, clippy::unwrap_used)]

use codec_cbor::general::{
    encode, frame_exact, validate_exact, LengthPolicy, Limits, MapEntry, Profile, ValueRef,
};
use stats_alloc::{Region, Stats, INSTRUMENTED_SYSTEM};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::marker::PhantomData;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

thread_local! {
    static MEASURING: Cell<bool> = const { Cell::new(false) };
}

struct ThreadScopedAllocator;
static SYSTEM: System = System;

fn selected_allocator() -> &'static dyn GlobalAlloc {
    // Allocation can also occur during TLS teardown, after all measurement
    // scopes have ended. That work belongs to the uninstrumented backend.
    if MEASURING.try_with(Cell::get).unwrap_or(false) {
        &INSTRUMENTED_SYSTEM
    } else {
        &SYSTEM
    }
}

// SAFETY: Both backends allocate through System. Switching instrumentation does
// not change pointer ownership, alignment, layout, or the deallocation backend.
#[allow(unsafe_code)]
unsafe impl GlobalAlloc for ThreadScopedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: Forward the caller's valid allocation layout unchanged.
        unsafe { selected_allocator().alloc(layout) }
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: Forward the valid layout and preserve zero-initialization.
        unsafe { selected_allocator().alloc_zeroed(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        // SAFETY: The caller transfers its exclusively owned live System block
        // with the original layout; either backend frees that same block once.
        unsafe { selected_allocator().dealloc(pointer, layout) };
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        // SAFETY: The caller owns the live block and supplies its current layout
        // and valid new size. Both backends preserve System's ownership rules.
        unsafe { selected_allocator().realloc(pointer, layout, new_size) }
    }
}

#[global_allocator]
static ALLOCATOR: ThreadScopedAllocator = ThreadScopedAllocator;

struct MeasurementScope {
    // A scope must end on the thread whose allocator flag it controls.
    _thread: PhantomData<Rc<()>>,
}

impl Drop for MeasurementScope {
    fn drop(&mut self) {
        let _ = MEASURING.try_with(|flag| flag.set(false));
    }
}

fn measure<T>(operation: impl FnOnce() -> T) -> (T, Stats) {
    MEASURING.with(|flag| {
        assert!(!flag.get(), "allocation measurement scopes must not nest");
        flag.set(true);
    });
    let scope = MeasurementScope {
        _thread: PhantomData,
    };
    let region = Region::new(&INSTRUMENTED_SYSTEM);
    let result = operation();
    drop(scope);
    (result, region.change())
}

fn assert_background_allocations_are_excluded() {
    const BACKGROUND_BYTES: usize = 4096;
    let begin = Arc::new(AtomicBool::new(false));
    let done = Arc::new(AtomicBool::new(false));
    let worker_begin = Arc::clone(&begin);
    let worker_done = Arc::clone(&done);
    let worker = std::thread::spawn(move || {
        while !worker_begin.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
        let buffer = vec![0xab; BACKGROUND_BYTES];
        worker_done.store(true, Ordering::Release);
        std::hint::black_box(buffer)
    });
    let ((), allocations) = measure(|| {
        begin.store(true, Ordering::Release);
        while !done.load(Ordering::Acquire) {
            std::thread::yield_now();
        }
    });
    let buffer = worker.join().unwrap();
    assert_eq!(buffer.len(), BACKGROUND_BYTES);
    assert_eq!(allocations.allocations, 0);
    assert_eq!(allocations.reallocations, 0);
}

#[test]
fn borrowed_validation_allocates_nothing_and_encoding_never_reallocates_payload() {
    // Keep the measurement cases in one test: the shared counters record only
    // its scoped calls, excluding unrelated harness and background activity.
    assert_background_allocations_are_excluded();
    // Nested, unsorted, compound keys exercise semantic comparison, not merely
    // the scalar fast path. No semantic tree or concatenated keys are needed.
    let input = [0xa2, 0x81, 0x02, 0x41, 0xaa, 0x81, 0x01, 0x41, 0xbb];
    let (frame, allocations) = measure(|| {
        let frame = frame_exact(&input, Limits::default(), LengthPolicy::AllowIndefinite).unwrap();
        validate_exact(&input, Profile::received(), Limits::default()).unwrap();
        frame
    });
    assert_eq!(allocations.allocations, 0);
    assert_eq!(allocations.reallocations, 0);

    let payload = [0xab; 512];
    let entries = [
        MapEntry::new(ValueRef::Unsigned(2), ValueRef::Bytes(&payload)),
        MapEntry::new(ValueRef::Unsigned(1), ValueRef::Received(&frame)),
    ];
    let (encoded, allocations) = measure(|| {
        encode(
            &ValueRef::Map(&entries),
            Profile::received(),
            Limits::default(),
        )
        .unwrap()
    });
    assert_eq!(allocations.allocations, 1);
    assert_eq!(allocations.reallocations, 0);
    assert_eq!(allocations.bytes_allocated, encoded.as_bytes().len());

    let (rejected, allocations) = measure(|| {
        encode(
            &ValueRef::Map(&entries),
            Profile::received(),
            Limits::default().with_output_bytes(1).unwrap(),
        )
    });
    assert!(rejected.is_err());
    assert_eq!(allocations.allocations, 0);
}
