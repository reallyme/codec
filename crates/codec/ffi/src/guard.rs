// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Panic firewall for the C ABI surface.
//!
//! Every `extern "C"` export routes its body through [`ffi_guard`]. Unwinding
//! across an `extern "C"` boundary is undefined behavior, so the guard converts
//! any panic escaping a codec operation (or any other unexpected unwind)
//! into a deterministic [`CODEC_INTERNAL_ERROR`] status code. This makes the
//! no-unwind guarantee a property of the code itself rather than a property of
//! `[profile.release] panic = "abort"`, protecting debug builds and downstream
//! consumers that static-link with the default `panic = "unwind"`.

use crate::status::{CodecStatus, CODEC_INTERNAL_ERROR};

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
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation)) {
        Ok(status) => status,
        Err(_payload) => CODEC_INTERNAL_ERROR,
    }
}
