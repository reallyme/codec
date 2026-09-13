# ReallyMe Codec Memory Model

This document defines the baseline memory-safety and boundary model for the
`reallyme-codec` workspace.

## Scope

- Primitive codec crates under `crates/` and the `crates/codec` facade
- Codec C ABI and JNI adapters under `crates/ffi`
- Codec WASM adapters under `crates/wasm`
- SDK packages under `packages/swift`, `packages/kotlin`,
  `packages/kotlin-android`, and `packages/ts`

## Data Classes

| Class | Examples | Handling |
|---|---|---|
| Untrusted input | PEM text, JSON, CBOR, multibase strings, multikey strings | Validate length and syntax before interpretation |
| Public bytes | Encoded public keys, CIDs, multicodec prefixes | Preserve canonical form and reject ambiguous encodings |
| Errors | Protobuf error envelopes, SDK errors | Do not include raw invalid input or backend exception text |

Codec operations may carry sensitive data even though they do not manage keys.
Internal temporary buffers use zeroizing owners where practical. Callers own
their inputs and returned bytes, strings, and value trees, and remain responsible
for their lifetime and cleanup. For example, Rust `CborValue` supports `Zeroize`
and can be held in `Zeroizing<CborValue>`; it does not wipe itself on drop.
Managed-runtime strings and protobuf copies cannot guarantee complete erasure.
Codec APIs avoid logging or embedding raw input in errors.

## Native Boundary

The native ABI validates pointer/length pairs, output buffers, produced-length
pointers, and integer conversions before constructing Rust slices. ABI exports
are protected by a panic firewall so unwinds never cross C or JNI boundaries.
The caller must still provide live allocations of the declared size and obey
the documented ownership and aliasing rules; pointer arithmetic checks cannot
prove that an arbitrary address is allocated. Scalar output pointers must be
disjoint from input and byte-output regions. Byte-output buffers may reuse an
input region where the operation finishes reading before writing its result.

Local Swift tests select the Rust library through
`REALLYME_CODEC_FFI_LIBRARY_PATH`. Kotlin's Gradle tests build and stage the host
JNI library as a resource by default. SDK packages must not fall back to local
codec implementations if the native provider is unavailable.

## Validation Gate

Before a release, run the workspace checks:

```sh
cargo fmt --check
cargo check --workspace --all-features
RUSTFLAGS=-Dwarnings cargo check --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo nextest run --workspace --all-features
cargo test --workspace --all-features
cargo check -p reallyme-codec-wasm --target wasm32-unknown-unknown
node scripts/run_pinned_release_readiness.mjs
node --test scripts/*.test.mjs scripts/release-readiness/*.test.mjs
npm --prefix packages/ts test
cargo deny check
cargo audit
```

Also run the [Swift](packages/swift/README.md#test) and
[Kotlin](packages/kotlin/README.md#test) development tests against the
current native build, plus the [Android runtime gate](packages/kotlin-android/README.md#test).
For native boundary changes, `scripts/test_ffi_abi_release_artifact.sh` checks the
release panic policy and exported symbols. `scripts/test_native_sanitizers.sh`
runs memory and runtime-UB checks with the pinned nightly toolchain.
