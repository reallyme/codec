// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{CODEC_ABI_VERSION, MAX_CODEC_FFI_INPUT_BYTES, MAX_CODEC_FFI_OUTPUT_BYTES};

/// Returns the exact C ABI operation contract version implemented by this library.
///
/// This leaf function only returns a compile-time constant and therefore has
/// no operation that can panic or unwind across the C boundary.
#[no_mangle]
pub extern "C" fn rm_codec_abi_version() -> u32 {
    CODEC_ABI_VERSION
}

// The package version is separate from the ABI version: a release can change
// codec behaviour without changing C signatures. SDKs check both before
// accepting a provider so a stale binary cannot silently serve new sources.
fn package_version_component(value: &'static str) -> u32 {
    value.parse::<u32>().unwrap_or(u32::MAX)
}

/// Returns the major version of the compiled FFI crate package.
#[no_mangle]
pub extern "C" fn rm_codec_package_version_major() -> u32 {
    package_version_component(env!("CARGO_PKG_VERSION_MAJOR"))
}

/// Returns the minor version of the compiled FFI crate package.
#[no_mangle]
pub extern "C" fn rm_codec_package_version_minor() -> u32 {
    package_version_component(env!("CARGO_PKG_VERSION_MINOR"))
}

/// Returns the patch version of the compiled FFI crate package.
#[no_mangle]
pub extern "C" fn rm_codec_package_version_patch() -> u32 {
    package_version_component(env!("CARGO_PKG_VERSION_PATCH"))
}

/// Returns the authoritative generic C ABI aggregate input limit.
///
/// Platform SDKs use this value after ABI-version validation so caller-side
/// fail-fast checks cannot drift from the Rust boundary.
#[no_mangle]
pub extern "C" fn rm_codec_max_ffi_input_bytes() -> usize {
    MAX_CODEC_FFI_INPUT_BYTES
}

/// Returns the authoritative generic C ABI output allocation limit.
///
/// Two-pass SDK callers reject provider-reported lengths above this value
/// before allocating managed arrays.
#[no_mangle]
pub extern "C" fn rm_codec_max_ffi_output_bytes() -> usize {
    MAX_CODEC_FFI_OUTPUT_BYTES
}
