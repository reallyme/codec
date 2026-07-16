// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

package me.really.codec

/**
 * Typed codec SDK errors. Variants intentionally carry no raw input bytes so
 * callers can log failures without leaking document or key material.
 */
public sealed class ReallyMeCodecException(message: String) : RuntimeException(message) {
    /** Input had the wrong shape, encoding, label, or canonical form. */
    public class InvalidInput : ReallyMeCodecException("invalid input")

    /** The backing Rust provider failed internally. */
    public class ProviderFailure : ReallyMeCodecException("provider failure")
}
