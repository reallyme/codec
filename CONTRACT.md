<!--
SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved

SPDX-License-Identifier: Apache-2.0
-->

# ReallyMe Codec Contract

This repository owns the `reallyme-codec` Rust crate family and the generated
SDK facades for Swift, Kotlin, and TypeScript.

## Owned Surfaces

- `crates/codec/**` contains the Rust source of truth for codec behavior.
- `crates/codec/ffi` exposes the native C ABI and JNI boundary used by Swift
  and Kotlin.
- `crates/codec/wasm-package` exposes the WASM boundary used by
  `@reallyme/codec`.
- `proto/reallyme/codec/v1/codec.proto` is the cross-language codec error
  envelope and configuration contract.
- `packages/codec`, `packages/swift`, and `packages/kotlin-codec` are thin
  TypeScript, Swift, and Java/Kotlin facades over the Rust implementation.

## Repository Shape

```text
reallyme/codec
  crates/
    codec/
    proto/codec/
  packages/
    codec/
    swift/
    kotlin-codec/
  proto/
    reallyme/codec/v1/codec.proto
```

Crypto primitives, provider policy, key generation, signing, encryption, KDFs,
and crypto conformance vectors belong in the separate crypto repository.

## SDK Boundary Rule

TypeScript uses the codec WASM package. Swift uses the codec C ABI. Java and
Kotlin use the codec JNI provider. Those SDKs may validate arguments and present
typed facades, but they must not hand-roll codec logic that would diverge from
Rust.

Expected failures are mapped to typed codec errors. Errors must not include raw
input bytes, backend exception text, PII, or secret material.
