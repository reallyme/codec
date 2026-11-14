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

The protobuf schema defines every public operation, request, result, option,
policy, and error. Pinned ReallyMe Buffa 0.1.1 generators produce the Rust contract;
the pinned generators in `buf.gen.yaml` produce the other lanes. Implementations
remain behind those generated definitions. New operations enter through the
schema before any native or SDK API is added.

Borrowed Rust readers, caller-supplied schema callbacks, and existing scalar
signatures are compatibility facades over the generated contract and the same
semantic implementation. Their lifetimes, ownership, and provider I/O remain
native concerns. They do not define independent policy or wire models. Legacy
scalar selectors and status values are generated from protobuf, retaining their
existing numeric ABI. New callers can use the generated operation envelope for
scalar, general-CBOR, DER, TLV framing, and structured operations alike.

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
