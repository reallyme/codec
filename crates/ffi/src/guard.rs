// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Panic firewall for the C ABI surface.
//!
//! Every `extern "C"` export routes its body through [`ffi_guard`]. Unwinding
//! across an `extern "C"` boundary is undefined behavior, so the guard converts
//! any panic escaping a codec operation (or any other unexpected unwind)
//! into a deterministic [`CODEC_INTERNAL_ERROR`] status code. This makes the
//! no-unwind guarantee explicit in the ABI implementation. Shipped native
//! artifacts are compiled with `panic=unwind` so this guard remains active even
//! though the workspace's general release profile uses `panic=abort`.
//! Downstream builds must preserve unwind semantics for this firewall to catch
//! panics instead of terminating the process.

use std::cell::Cell;
use std::sync::Once;

use crate::status::{CodecStatus, CODEC_INTERNAL_ERROR};

thread_local! {
    static INSIDE_NATIVE_BOUNDARY: Cell<bool> = const { Cell::new(false) };
}

static INSTALL_REDACTING_HOOK: Once = Once::new();

struct NativeBoundaryScope {
    previous: bool,
}

impl NativeBoundaryScope {
    fn enter() -> Self {
        let previous = INSIDE_NATIVE_BOUNDARY.replace(true);
        Self { previous }
    }
}

impl Drop for NativeBoundaryScope {
    fn drop(&mut self) {
        INSIDE_NATIVE_BOUNDARY.set(self.previous);
    }
}

fn inside_native_boundary() -> bool {
    matches!(INSIDE_NATIVE_BOUNDARY.try_with(Cell::get), Ok(true))
}

fn install_redacting_panic_hook() {
    INSTALL_REDACTING_HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |panic_info| {
            if !inside_native_boundary() {
                previous(panic_info);
            }
        }));
    });
}

/// Runs a native-boundary operation with panic-hook output redacted.
///
/// Rust invokes the process-wide panic hook before `catch_unwind` or JNI's
/// panic conversion sees the failure. The installed hook delegates to the
/// host's existing hook everywhere except this thread's active native call,
/// preventing dependency panic text or source paths from reaching stderr.
/// Nested calls restore the prior thread-local state deterministically.
pub fn with_redacted_panic_hook<F, T>(operation: F) -> T
where
    F: FnOnce() -> T,
{
    install_redacting_panic_hook();
    let _scope = NativeBoundaryScope::enter();
    operation()
}

/// Run an FFI operation body behind a panic firewall.
///
/// On the normal path the operation's own [`CodecStatus`] is returned
/// unchanged, so status codes, output-buffer semantics, and the ABI are
/// untouched. If the operation panics, the unwind is caught at this boundary
/// and reported as [`CODEC_INTERNAL_ERROR`]; the panic payload is dropped and
/// never re-raised or exposed to the caller.
///
/// # Unwind safety
///
/// The closure is wrapped in [`AssertUnwindSafe`] because the FFI bodies borrow
/// caller-owned raw pointers, which are not `UnwindSafe`. This is sound for this
/// crate:
///
/// - The operations act on caller-owned raw buffers and short-lived local
///   state; there is no shared `&mut`/interior-mutable state that could be
///   observed in a torn, half-updated form after a caught panic.
/// - A panic between output writes leaves caller memory in its pre-existing
///   state (each buffer write is a single `copy_from_slice`), so the caller
///   never observes a partially mutated logical value across the boundary.
/// - Secret-bearing temporaries that the FFI layer owns are held in
///   `Zeroizing`/`ZeroizeOnDrop` wrappers wherever the boundary must allocate
///   copied material. Those destructors run during the unwind, before this
///   function converts the panic into a status. Caller-owned input and output
///   buffers remain the caller's responsibility.
///
/// [`AssertUnwindSafe`]: std::panic::AssertUnwindSafe
#[inline]
pub fn ffi_guard<F>(operation: F) -> CodecStatus
where
    F: FnOnce() -> CodecStatus,
{
    with_redacted_panic_hook(|| {
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation)) {
            Ok(status) => status,
            Err(_payload) => CODEC_INTERNAL_ERROR,
        }
    })
}

#[cfg(test)]
mod tests {
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
}
