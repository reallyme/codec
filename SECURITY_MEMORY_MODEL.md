<!--
SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved

SPDX-License-Identifier: Apache-2.0
-->

# ReallyMe Codec Memory Model

This document defines the baseline memory-safety and boundary model for the
`reallyme-codec` workspace.

## Scope

- Codec crates under `crates/codec/`
- Codec C ABI and JNI adapters under `crates/codec/ffi`
- Codec WASM adapters under `crates/codec/wasm-package`
- SDK packages under `packages/swift`, `packages/kotlin-codec`, and
  `packages/codec`

## Data Classes

| Class | Examples | Handling |
|---|---|---|
| Untrusted input | PEM text, JSON, CBOR, multibase strings, multikey strings | Validate length and syntax before interpretation |
| Public bytes | Encoded public keys, CIDs, multicodec prefixes | Preserve canonical form and reject ambiguous encodings |
| Structured output | JSON ABI payloads, proto errors | Do not include raw invalid input or backend exception text |

Codec operations are not cryptographic secret owners. If callers pass sensitive
bytes through a codec, the caller remains responsible for secret ownership and
zeroization. Codec APIs still avoid logging or embedding raw input in errors.

## Native Boundary

The native ABI validates pointer/length pairs, output buffers, produced-length
pointers, and integer conversions before constructing Rust slices. ABI exports
are protected by a panic firewall so unwinds never cross C or JNI boundaries.

Swift and Kotlin load `reallyme-codec-ffi` explicitly through
`REALLYME_CODEC_FFI_LIBRARY_PATH` in tests. The SDK packages must not fall back
to local codec implementations if the native provider is unavailable.

## Validation Gate

Before a release, run the codec subset:

```sh
cargo fmt --check
cargo check --workspace --all-features
RUSTFLAGS=-Dwarnings cargo check --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check -p reallyme-codec-wasm --target wasm32-unknown-unknown
node scripts/check_release_readiness.mjs
npm --prefix packages/codec test
swift test
packages/kotlin-codec/gradlew -p packages/kotlin-codec test
cargo deny check
```
