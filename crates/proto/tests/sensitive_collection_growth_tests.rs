// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Inline sensitive scalars must not survive a repeated-message reallocation.
#![cfg(feature = "generated")]
#![allow(
    unsafe_code,
    reason = "allocator probe reads only initialized fields before deallocation"
)]
#![allow(clippy::unwrap_used, reason = "bounded synthetic test messages")]

use buffa::Message;
use reallyme_codec_proto::generated::proto::reallyme::codec::v1::CodecDerValue;
use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicBool, AtomicPtr, Ordering};

struct Probe;
static WATCHED: AtomicPtr<u8> = AtomicPtr::new(std::ptr::null_mut());
static RETIRED: AtomicBool = AtomicBool::new(false);
static STALE: AtomicBool = AtomicBool::new(false);
const INITIAL_VALUES: usize = 4;

fn inspect_retired(pointer: *mut u8, layout: Layout) {
    if WATCHED
        .compare_exchange(
            pointer,
            std::ptr::null_mut(),
            Ordering::SeqCst,
            Ordering::SeqCst,
        )
        .is_err()
    {
        return;
    }
    RETIRED.store(true, Ordering::SeqCst);
    for index in 0usize..INITIAL_VALUES {
        let offset = index
            .checked_mul(std::mem::size_of::<CodecDerValue>())
            .and_then(|offset| offset.checked_add(std::mem::offset_of!(CodecDerValue, bool_value)));
        let Some(offset) = offset.filter(|offset| *offset < layout.size()) else {
            STALE.store(true, Ordering::SeqCst);
            return;
        };
        // SAFETY: WATCHED identifies the fully initialized, four-element Vec
        // created below. Only its one-byte Option<bool> fields are read, never
        // struct padding or spare slots. The allocator has not freed/reallocated
        // this range yet. The hardened move writes zeros before retiring it.
        if unsafe { pointer.add(offset).read_volatile() } != 0 {
            STALE.store(true, Ordering::SeqCst);
        }
    }
}

// SAFETY: All pointer ownership, alignment and layout operations are delegated
// unchanged to System. Inspection occurs before retiring the watched allocation.
unsafe impl GlobalAlloc for Probe {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: The caller supplies the GlobalAlloc allocation contract.
        unsafe { System.alloc(layout) }
    }
    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: The caller supplies the GlobalAlloc allocation contract.
        unsafe { System.alloc_zeroed(layout) }
    }
    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        inspect_retired(pointer, layout);
        // SAFETY: The caller transfers this matching live allocation to System.
        unsafe { System.dealloc(pointer, layout) }
    }
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        inspect_retired(pointer, layout);
        // SAFETY: The caller supplies the matching allocation and new size.
        unsafe { System.realloc(pointer, layout, size) }
    }
}
#[global_allocator]
static ALLOCATOR: Probe = Probe;

#[test]
fn binary_repeated_message_growth_erases_the_retired_inline_scalar_slots() {
    assert_eq!(std::mem::size_of::<Option<bool>>(), 1);
    let mut parent = CodecDerValue::default();
    parent.children = Vec::with_capacity(INITIAL_VALUES).into();
    for _ in 0..INITIAL_VALUES {
        let mut value = CodecDerValue::default();
        value.bool_value = Some(true);
        parent.children.push(value);
    }
    assert_eq!(parent.children.capacity(), INITIAL_VALUES);
    STALE.store(false, Ordering::SeqCst);
    RETIRED.store(false, Ordering::SeqCst);
    WATCHED.store(
        parent.children.as_ptr().cast_mut().cast::<u8>(),
        Ordering::SeqCst,
    );
    // children (field 7) contains bool_value (field 5), explicitly true.
    let result = parent.merge_from_slice(&[0x3a, 2, 0x28, 1]);
    WATCHED.store(std::ptr::null_mut(), Ordering::SeqCst);
    result.unwrap();
    assert!(RETIRED.load(Ordering::SeqCst));
    assert!(!STALE.load(Ordering::SeqCst));
    assert_eq!(parent.children.len(), 5);
    assert!(parent
        .children
        .iter()
        .all(|value| value.bool_value == Some(true)));
}

#[test]
fn json_repeated_message_growth_preserves_each_optional_scalar() {
    let value: CodecDerValue = reallyme_codec_proto::decode_json(
        br#"{"children":[{"boolValue":true},{"boolValue":false},{"boolValue":null},{},{"boolValue":true}]}"#,
    ).unwrap();
    assert_eq!(
        value
            .children
            .iter()
            .map(|child| child.bool_value)
            .collect::<Vec<_>>(),
        [Some(true), Some(false), None, None, Some(true)]
    );
}
