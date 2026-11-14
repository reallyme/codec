// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Observe initialized bytes immediately before the system allocator retires them.

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;

#[derive(Clone, Copy)]
struct Observation {
    pointer: *mut u8,
    allocation_size: usize,
    allocation_align: usize,
    inspected_length: usize,
    allocations: usize,
    retired: bool,
    stale: bool,
}

impl Observation {
    const EMPTY: Self = Self {
        pointer: std::ptr::null_mut(),
        allocation_size: 0,
        allocation_align: 0,
        inspected_length: 0,
        allocations: 0,
        retired: false,
        stale: false,
    };
}

thread_local! {
    // Restrict observation to the test thread, so allocations by the test
    // harness cannot consume an armed probe. Initialization allocates nothing.
    static OBSERVATION: Cell<Observation> = const { Cell::new(Observation::EMPTY) };
}

pub struct Probe;

/// Watch an existing allocation whose inspected bytes are all initialized.
#[allow(
    dead_code,
    reason = "shared probe also supports arm-only test binaries"
)]
pub fn watch(pointer: *mut u8, initialized_length: usize) {
    OBSERVATION.with(|state| {
        state.set(Observation {
            pointer,
            inspected_length: initialized_length,
            ..Observation::EMPTY
        });
    });
}

/// Watch the first allocation with this exact layout on the current thread.
/// The test must ensure its inspected prefix is initialized before retirement.
pub fn arm(size: usize, align: usize, initialized_length: usize) {
    OBSERVATION.with(|state| {
        state.set(Observation {
            allocation_size: size,
            allocation_align: align,
            inspected_length: initialized_length,
            ..Observation::EMPTY
        });
    });
}

fn allocated(pointer: *mut u8, layout: Layout) {
    let _ = OBSERVATION.try_with(|state| {
        let mut observation = state.get();
        if !pointer.is_null()
            && layout.size() == observation.allocation_size
            && layout.align() == observation.allocation_align
        {
            observation.allocations += 1;
            if observation.pointer.is_null() && !observation.retired {
                observation.pointer = pointer;
            }
            state.set(observation);
        }
    });
}

fn retiring(pointer: *mut u8, layout: Layout) {
    let _ = OBSERVATION.try_with(|state| {
        let mut observation = state.get();
        if pointer != observation.pointer {
            return;
        }
        observation.pointer = std::ptr::null_mut();
        observation.retired = true;
        if observation.inspected_length > layout.size() {
            observation.stale = true;
        } else {
            for offset in 0..observation.inspected_length {
                // SAFETY: An armed test guarantees this prefix was initialized.
                // The matching allocation is still live, and inspection never
                // includes struct padding or uninitialized spare capacity.
                if unsafe { pointer.add(offset).read_volatile() } != 0 {
                    observation.stale = true;
                }
            }
        }
        state.set(observation);
    });
}

/// Disarm before assertions, which may allocate while reporting a failure.
pub fn assert_erased(expected_allocations: Option<usize>) {
    let observation = OBSERVATION.with(|state| state.replace(Observation::EMPTY));
    assert!(observation.retired, "watched allocation was not retired");
    assert!(!observation.stale, "initialized bytes survived retirement");
    if let Some(count) = expected_allocations {
        assert_eq!(observation.allocations, count);
    }
}

// SAFETY: Allocation ownership, size and alignment are delegated unchanged to
// System. Probes read initialized memory only before deallocation/reallocation.
unsafe impl GlobalAlloc for Probe {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        // SAFETY: The caller provides the GlobalAlloc allocation contract.
        let pointer = unsafe { System.alloc(layout) };
        allocated(pointer, layout);
        pointer
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        // SAFETY: The caller provides the GlobalAlloc allocation contract.
        let pointer = unsafe { System.alloc_zeroed(layout) };
        allocated(pointer, layout);
        pointer
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        retiring(pointer, layout);
        // SAFETY: The caller transfers the matching allocation to System.
        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        retiring(pointer, layout);
        // SAFETY: The caller supplies the matching allocation and new size.
        let replacement = unsafe { System.realloc(pointer, layout, size) };
        if let Ok(new_layout) = Layout::from_size_align(size, layout.align()) {
            allocated(replacement, new_layout);
        }
        replacement
    }
}
