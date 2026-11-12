// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Observe initialized output immediately before its real allocator frees it.
//! One test prevents concurrent allocator-observation cases within this binary.
#![allow(missing_docs, unsafe_code, clippy::unwrap_used)]

use codec_cbor::general::{
    encode, frame_exact, ErrorReason, Float, LengthPolicy, Limits, MapEntry, Profile, ValueRef,
};
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicBool, AtomicPtr, AtomicUsize, Ordering};
use zeroize::{Zeroize, ZeroizeOnDrop};

struct ObservedAllocator;
static WATCH_SIZE: AtomicUsize = AtomicUsize::new(0);
static WATCH_POINTER: AtomicPtr<u8> = AtomicPtr::new(core::ptr::null_mut());
static WIPED: AtomicBool = AtomicBool::new(false);
thread_local! { static OBSERVE: Cell<bool> = const { Cell::new(false) }; }

// SAFETY: All allocations and layouts are forwarded unchanged to System. The
// observer reads only the allocation selected by exact size, immediately before
// deallocation, while ownership is exclusively held by this allocator callback.
unsafe impl GlobalAlloc for ObservedAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: GlobalAlloc supplies the same valid layout forwarded to System.
        let pointer = unsafe { System.alloc(layout) };
        if OBSERVE.try_with(Cell::get).unwrap_or(false)
            && layout.size() == WATCH_SIZE.load(Ordering::SeqCst)
            && !pointer.is_null()
        {
            WATCH_POINTER.store(pointer, Ordering::SeqCst);
        }
        pointer
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        if WATCH_POINTER
            .compare_exchange(
                pointer,
                core::ptr::null_mut(),
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .is_ok()
        {
            // SAFETY: This pointer/layout is a live allocation about to be freed.
            // Zeroizing<Vec<u8>> erases its full capacity before deallocation,
            // including spare capacity if an error interrupts output writing.
            let contents = unsafe { core::slice::from_raw_parts(pointer, layout.size()) };
            WIPED.store(contents.iter().all(|byte| *byte == 0), Ordering::SeqCst);
        }
        // SAFETY: Preserve the original allocation's pointer, layout, and owner.
        unsafe { System.dealloc(pointer, layout) };
    }
}

#[global_allocator]
static ALLOCATOR: ObservedAllocator = ObservedAllocator;

fn protected<T: ZeroizeOnDrop>() {}

#[test]
fn initialized_output_is_wiped_on_success_and_semantic_rejection() {
    protected::<codec_cbor::general::EncodedCbor>();
    protected::<codec_cbor::general::Integer>();
    protected::<Float>();
    let mut float = Float::binary64(u64::MAX);
    float.zeroize();
    assert_eq!(float.bits(), 0);

    let payload = [0xab; 509];
    // Three-byte byte-string header plus contents: no populated reallocation.
    WATCH_SIZE.store(512, Ordering::SeqCst);
    OBSERVE.with(|observe| observe.set(true));
    let output = encode(
        &ValueRef::Bytes(&payload),
        Profile::preferred(),
        Limits::default(),
    )
    .unwrap();
    assert!(!WATCH_POINTER.load(Ordering::SeqCst).is_null());
    drop(output);
    assert!(WIPED.load(Ordering::SeqCst));

    // Exhaust work after only the header was written. The uninitialized spare
    // capacity must be erased as well as the initialized payload prefix.
    WIPED.store(false, Ordering::SeqCst);
    let error = encode(
        &ValueRef::Bytes(&payload),
        Profile::preferred(),
        Limits::default().with_work(518).unwrap(),
    )
    .err()
    .unwrap();
    assert_eq!(error.reason(), ErrorReason::WorkLimit);
    assert!(WIPED.load(Ordering::SeqCst));

    let mut invalid_text = [0xff; 512];
    invalid_text[..3].copy_from_slice(&[0x79, 0x01, 0xfd]);
    let frame = frame_exact(&invalid_text, Limits::default(), LengthPolicy::DefiniteOnly).unwrap();
    WIPED.store(false, Ordering::SeqCst);
    let error = encode(
        &ValueRef::Received(&frame),
        Profile::received(),
        Limits::default(),
    )
    .err()
    .unwrap();
    assert_eq!(error.reason(), ErrorReason::InvalidUtf8);
    assert!(WIPED.load(Ordering::SeqCst));

    // The duplicate is rejected after the entire output has been initialized;
    // the same protected owner must erase it on this error exit as well.
    let duplicate = [
        MapEntry::new(ValueRef::Bytes(&payload), ValueRef::Null),
        MapEntry::new(ValueRef::Bytes(&payload), ValueRef::Null),
    ];
    WATCH_SIZE.store(1027, Ordering::SeqCst);
    WIPED.store(false, Ordering::SeqCst);
    let error = encode(
        &ValueRef::Map(&duplicate),
        Profile::preferred(),
        Limits::default(),
    )
    .err()
    .unwrap();
    assert_eq!(error.reason(), ErrorReason::DuplicateMapKey);
    assert!(WATCH_POINTER.load(Ordering::SeqCst).is_null());
    assert!(WIPED.load(Ordering::SeqCst));
    WATCH_SIZE.store(0, Ordering::SeqCst);
    OBSERVE.with(|observe| observe.set(false));
}
