// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native C ABI and JNI surface for the ReallyMe codec packages.
//!
//! # Panic safety
//!
//! Unwinding across an `extern "C"` or JNI boundary is undefined behavior. Every
//! exported function routes its body through `guard::ffi_guard` or the JNI
//! environment guard, converting unexpected unwinds into deterministic provider
//! failures without exposing panic payloads to callers.

// This crate is a native/mobile ABI. Browsers consume codec operations through
// the wasm-bindgen crate under `crates/wasm`.
#![cfg(not(target_arch = "wasm32"))]

// `catch_unwind` cannot protect an embedding process when this crate is built
// with panic=abort. Refuse such artifacts at compile time instead of shipping
// a panic firewall that is structurally present but operationally inert.
#[cfg(not(panic = "unwind"))]
compile_error!("reallyme-codec-ffi must be compiled with panic=unwind");

/// C ABI surface for ReallyMe codec operations used by platform packages.
pub mod codec;
mod guard;
/// JNI bridge used by the Kotlin codec package.
#[cfg(feature = "jni")]
pub mod kotlin_codec;
/// Internal raw-pointer/length validation and buffer-write helpers.
mod pointer;
/// Status codes for the codec ABI surface.
mod status;
