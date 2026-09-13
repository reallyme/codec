// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

/// Typed Swift codec errors. Variants intentionally carry no raw input bytes
/// so callers can log failures without leaking document or key material.
public enum ReallyMeCodecError: Error, Equatable, Sendable {
    case unsupportedPlatform
    case dynamicLibraryNotFound
    case dynamicLibraryLoadFailed
    case symbolNotFound
    case invalidInput
    case providerFailure
}
