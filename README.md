<!--
SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved

SPDX-License-Identifier: Apache-2.0
-->

# reallyme-codec

[![Code Checks](https://github.com/reallyme/codec/actions/workflows/code-checks.yml/badge.svg)](https://github.com/reallyme/codec/actions/workflows/code-checks.yml)
[![reallyme-codec](https://img.shields.io/crates/v/reallyme-codec?label=reallyme-codec&color=0f766e)](https://crates.io/crates/reallyme-codec)
[![npm codec](https://img.shields.io/npm/v/@reallyme/codec?label=npm%20codec&color=0f766e)](https://www.npmjs.com/package/@reallyme/codec)
[![Maven codec](https://img.shields.io/maven-central/v/me.really/codec?label=maven%20codec&color=0f766e)](https://central.sonatype.com/artifact/me.really/codec)
[![Security Policy](https://img.shields.io/badge/security-policy-0f766e)](SECURITY.md)
[![License](https://img.shields.io/badge/license-Apache--2.0-blue)](LICENSE)

ReallyMe Codec keeps encodings consistent across Rust, TypeScript, Swift, Java,
and Kotlin. It covers base encodings, multiformats, canonical CBOR/JCS, PEM armor, 
and the protobuf error operation contract. Rust implements the codec behavior; TypeScript
calls Rust through WASM, Swift through the C ABI, and Java/Kotlin through JNI.

## Why

Identity systems depend on exact bytes. A public key, CID, PEM document,
DAG-CBOR value, or canonical JSON value must encode the same way on servers,
Apple platforms, Android, browsers, and WASM. ReallyMe Codec keeps that behavior
in one Rust implementation and exposes it through small language packages.

## Package Model

ReallyMe Codec is one cross-language codec operation contract for identity data. The
publishable Rust leaf crates exist for dependency hygiene, implementation
modularity, and crates.io dependency resolution. They are support crates
released in lockstep with `reallyme-codec`, not separately marketed products
with independent compatibility promises.

SDK consumers should usually start with the umbrella package for their
language: `reallyme-codec`, `@reallyme/codec`, `ReallyMeCodec`, or
`me.really:codec`. Direct use of leaf crates is appropriate when a Rust
consumer deliberately needs a smaller primitive surface and accepts the same
lockstep ReallyMe Codec release line.

## Published Surfaces

| Surface | Distribution | Native path |
|---|---|---|
| Rust | `reallyme-codec` on crates.io | Pure Rust |
| TypeScript | `@reallyme/codec` on npm | Rust WASM |
| Swift | `ReallyMeCodec` through SwiftPM | Rust C ABI |
| Java/Kotlin | `me.really:codec` on Maven Central | Rust JNI |
| Android | `me.really:codec-android` AAR | Rust JNI in `jniLibs` |
| Protobuf | `reallyme.codec.v1` schema | Generated bindings |

## Source Map

| Area | Path |
|---|---|
| Core codecs | `crates/codec/*` |
| FFI and WASM adapters | `crates/ffi`, `crates/wasm` |
| TypeScript package | `packages/ts` |
| Swift package | `packages/swift`, `Package.swift` |
| Java/Kotlin package | `packages/kotlin` |
| Android AAR build | `packages/kotlin-android` |
| Protobuf operation contract | `crates/proto/proto/reallyme/codec/v1/codec.proto` |

## Supported Codecs

| Category | Surface |
|---|---|
| Base encodings | base64, unpadded base64url, lowercase hex, base58btc |
| Multiformats | multibase, multicodec, multikey, key-binding validation |
| Canonical data | deterministic generic CBOR, DAG-CBOR, CID helpers, SHA-256 content hash, multihash helpers, JCS |
| PEM | strict PEM decode and encode with label, size, line-width, and line-ending policy |
| Protobuf | `reallyme.codec.v1` generated operation requests, generated ProtoJSON, and codec error envelopes |

## Install

### Rust

```sh
cargo add reallyme-codec
```

The Rust crates require Rust `1.96.0` or newer. The default feature set enables
every codec family. Consumers that need a smaller dependency surface can select
only the families they use:

```toml
reallyme-codec = { version = "0.2.0", default-features = false, features = [
  "base64url",
  "multikey",
] }
```

### Swift

```swift
.package(
    url: "https://github.com/reallyme/codec",
    from: "0.2.0"
)
```

```swift
.product(name: "ReallyMeCodec", package: "codec")
```

### Kotlin

```kotlin
dependencies {
    implementation("me.really:codec:0.2.0")
}
```

### Android

```kotlin
dependencies {
    implementation("me.really:codec-android:0.2.0")
}
```

### TypeScript

```sh
npm install @reallyme/codec
```

For production deployments, pin exact package versions, release tags, or Git
revisions so codec behavior remains identical across all language lanes.

## Versioning

ReallyMe Codec is pre-1.0. We follow the Rust community convention for
[0.x compatibility](https://doc.rust-lang.org/cargo/reference/semver.html):
breaking changes increment the minor version, such as `0.1.x` to `0.2.0`, and
additive compatible changes increment the patch version, such as `0.2.0` to
`0.2.1`.

For Rust consumers, pin to the minor line you have reviewed, for example
`reallyme-codec = "0.2"`. A full version written by `cargo add`, such as
`reallyme-codec = "0.2.0"`, uses Cargo caret semantics and remains on the same
`0.2.x` compatibility line. For npm, Maven, SwiftPM, and release artifacts used
in production, prefer exact versions or locked dependency files so every
language lane runs the same reviewed codec release.

## Quick Start

Rust:

```rust
use reallyme_codec::base64url::{base64url_to_bytes, bytes_to_base64url};

let encoded = bytes_to_base64url(b"hello");
let decoded = base64url_to_bytes(&encoded)?;
# Ok::<(), reallyme_codec::base64url::Base64UrlError>(())
```

Swift:

```swift
import ReallyMeCodec

let codec = try ReallyMeCodec()
let encoded = try codec.base64urlEncode([1, 2, 3])
```

Kotlin:

```kotlin
import me.really.codec.ReallyMeCodec

val encoded = ReallyMeCodec.base64urlEncode(byteArrayOf(1, 2, 3))
```

Java:

```java
import me.really.codec.ReallyMeCodec;

String encoded = ReallyMeCodec.base64urlEncode(new byte[] {1, 2, 3});
```

TypeScript:

```ts
import { ReallyMeCodec } from "@reallyme/codec";

const encoded = ReallyMeCodec.base64urlEncode(new Uint8Array([1, 2, 3]));
```

Deterministic generic CBOR is a first-class structured surface. Public SDKs
build typed values, cross the generated protobuf operation boundary, and use
the Rust primitive codec for canonical bytes:

```swift
let value = ReallyMeDeterministicCbor.mapText([
    ("b", ReallyMeDeterministicCbor.unsigned(2)),
    ("a", ReallyMeDeterministicCbor.bytes(Data([0, 1, 2]))),
])
let cbor = try codec.deterministicCborEncodeData(value)
```

```kotlin
val value = ReallyMeDeterministicCbor.mapText(
    linkedMapOf(
        "b" to ReallyMeDeterministicCbor.unsignedLong(2),
        "a" to ReallyMeDeterministicCbor.bytes(byteArrayOf(0, 1, 2)),
    ),
)
val cbor = ReallyMeCodec.deterministicCborEncode(value)
```

```ts
const value = ReallyMeDeterministicCbor.mapText([
  ["b", ReallyMeDeterministicCbor.unsigned(2n)],
  ["a", ReallyMeDeterministicCbor.bytes(new Uint8Array([0, 1, 2]))],
]);
const cbor = ReallyMeCodec.deterministicCborEncode(value);
```

Deterministic-CBOR encode canonicalizes map ordering. Decode rejects duplicate
semantic map keys, non-canonical map order, non-minimal integer or length
forms, unsupported CBOR types, and inputs outside the documented resource
limits. DAG-CBOR remains a separate profile with CID helpers; its structured
encode/decode methods use the same generated operation boundary and Rust
semantic implementation.

Swift, Java, and Kotlin do not reimplement codecs. Public SDK releases ship
prebuilt Rust FFI/JNI libraries and load them through SwiftPM or Maven package
artifacts. `ReallyMeCodecRustCAbiLibrary(path:)` and
`ReallyMeCodecRustNativeProvider.loadLibrary(path)` remain available for local
development against a freshly built `reallyme-codec-ffi` library.

## Protobuf

The importable wire/config operation contract lives in the publishable proto crate at
[`crates/proto/proto/reallyme/codec/v1/codec.proto`](crates/proto/proto/reallyme/codec/v1/codec.proto).
Service, application, and storage protos can import it when they need stable
codec identifiers or non-secret error envelopes.

The executable transport boundary accepts one generated
`CodecOperationRequest` and returns one binary `CodecOperationResponse` whose
oneofs identify either the exact operation result or a typed `CodecError`.
Operation-specific `*Proto` SDK helpers are request builders over that same
entrypoint; they are not separate wire APIs. Native Rust callers retain the
typed codec APIs and do not need to serialize.

The protobuf schema is the canonical operation contract for cross-language request,
response, and error shapes. Rust defines codec behavior, but SDKs and adapters
must not define parallel DTOs or error models that can evolve independently
from the generated protobuf types.

Codec's protobuf transport JSON surface is generated ProtoJSON from Buffa,
using protobuf enum names and standard protobuf base64 encoding for bytes. It
is the JSON representation of the same protobuf operation contract, not a separate
hand-written JSON API. JSON is request-only; results remain the binary protobuf
response. JSON requests are decoded through generated message types and
re-encoded at the boundary so expansion cannot bypass the binary protobuf size
cap. Use this lane for Connect JSON, CLIs, browser adapters, and conformance
tooling that require JSON; prefer binary protobuf or native SDK methods when
moving secret-bearing bytes.

The generated proto surfaces are available through:

| Language | Proto surface | Generic execution |
|---|---|---|
| Rust | `reallyme-codec-proto` | `process_operation_response`, `process_operation_response_json` |
| Swift | `ReallyMeCodecProto` | `processOperation`, `processOperationJson` |
| Kotlin / Java | `me.really.codec.v1` | `processOperation`, `processOperationJson` |
| TypeScript | `@reallyme/codec/proto` | `processOperation`, `processOperationJson` |

See [docs/protobuf.md](docs/protobuf.md) for the boundary rules.

## Documentation

- [CONTRACT.md](CONTRACT.md) - package, proto, and SDK boundary operation contract
- [SECURITY.md](SECURITY.md) - vulnerability reporting and security model
- [SECURITY_MEMORY_MODEL.md](SECURITY_MEMORY_MODEL.md) - codec boundary and memory handling model
- [docs/protobuf.md](docs/protobuf.md) - protobuf generation and ownership rules
- [docs/rust-publishing.md](docs/rust-publishing.md) - Rust crate release order

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).

## Copyright And Trademarks

Copyright © 2026 by ReallyMe LLC.

ReallyMe<sup>®</sup> is a registered trademark of ReallyMe LLC.
