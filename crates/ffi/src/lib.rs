// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
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
// Kani compiles with panic=abort and builds only private boundary helpers.
// Keep all ABI exports out of that build so every ABI artifact still requires
// a working panic firewall.
#[cfg(not(kani))]
#[cfg(not(panic = "unwind"))]
compile_error!("reallyme-codec-ffi must be compiled with panic=unwind");

/// C ABI surface for ReallyMe codec operations used by platform packages.
#[cfg(not(kani))]
pub mod codec;
#[cfg(not(kani))]
mod guard;
#[cfg(any(kani, feature = "jni"))]
mod jni_capacity;
/// JNI bridge used by the Kotlin codec package.
#[cfg(all(not(kani), feature = "jni"))]
pub mod kotlin_codec;
/// Internal raw-pointer/length validation and buffer-write helpers.
mod pointer;
/// Status codes for the codec ABI surface.
mod status;
