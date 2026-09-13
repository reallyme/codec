# ReallyMe Codec Contract

This repository owns the `reallyme-codec` Rust crate family and the generated
SDK facades for Swift, Kotlin, and TypeScript.

## Owned Surfaces

- Codec leaf crates such as `crates/base64`, `crates/cbor`, and `crates/pem`
  implement primitive behavior. `crates/codec` provides the Rust facade and
  generated operation dispatch.
- `crates/ffi` exposes the native C ABI and JNI boundary used by Swift
  and Kotlin.
- `crates/wasm` exposes the WASM boundary used by
  `@reallyme/codec`.
- `crates/proto` owns the publishable protobuf crate, generated bindings,
  and source schema for the cross-language codec wire contract.
- `packages/ts`, `packages/swift`, `packages/kotlin`, and
  `packages/kotlin-android` are thin TypeScript, Swift, Java/Kotlin, and
  Android facades over the Rust implementation.

The protobuf schema is canonical for cross-language request, response, and
error shapes. Rust primitive modules remain canonical for behavior, but SDK
structured result types and error codes must be generated from, or mechanically
backed by, the protobuf schema rather than maintained as independent parallel
models. Scalar adapter calls remain available for base encodings, predicates,
and JCS; structured operations use the generated request/response boundary.

## Repository Shape

```text
reallyme/codec
  crates/
    codec/       # Rust facade and operation dispatch
    cbor/        # CBOR primitive (alongside the other codec leaf crates)
    proto/       # Schema, generated Rust messages, and wire codecs
    ffi/
    wasm/
  packages/
    ts/
    swift/
    kotlin/
    kotlin-android/
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
