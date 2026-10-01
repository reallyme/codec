// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec

/**
 * Typed codec SDK errors. Variants intentionally carry no raw input bytes so
 * callers can log failures without leaking document or key material.
 */
public sealed class ReallyMeCodecException(message: String) : RuntimeException(message) {
    /** Input had the wrong shape, encoding, label, or canonical form. */
    public open class InvalidInput : ReallyMeCodecException("invalid input")

    /** The input parses but violates a canonical encoding rule. */
    public class NonCanonical : InvalidInput()

    /** A multicodec name or prefix is not supported by this release. */
    public class UnsupportedCodec : InvalidInput()

    /** An IPLD value type is outside the SDK's closed DAG-CBOR value model. */
    public class UnsupportedIpldValue : InvalidInput()

    /** The backing Rust provider failed internally. */
    public class ProviderFailure : ReallyMeCodecException("provider failure")
}
