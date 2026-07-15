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

ReallyMe Codec provides platform-agnostic byte-format, multiformat, canonical
encoding, and protobuf error-contract utilities for Rust, TypeScript, Swift,
Java, and Kotlin. The Rust codec crates are the source of truth; TypeScript uses
the codec WASM package, Swift uses the codec C ABI library, and Java/Kotlin use
the codec JNI library.

## Why

Encoding behavior must be identical across servers, Apple platforms, Android,
browsers, and WASM. ReallyMe Codec keeps PEM armor, base encodings, multibase,
multicodec, multikey, DAG-CBOR, CID helpers, and JSON canonicalization in one
audited Rust implementation instead of maintaining separate hand-written SDK
implementations.

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
| FFI and WASM adapters | `crates/codec/ffi`, `crates/codec/wasm-package` |
| TypeScript package | `packages/codec` |
| Swift package | `packages/swift`, `Package.swift` |
| Java/Kotlin package | `packages/kotlin-codec` |
| Android AAR build | `packages/android-codec` |
| Protobuf contract | `proto/reallyme/codec/v1/codec.proto` |

## Supported Codecs

| Category | Surface |
|---|---|
| Base encodings | base64, unpadded base64url, lowercase hex, base58btc |
| Multiformats | multibase, multicodec, multikey, key-binding validation |
| Canonical data | DAG-CBOR, CID helpers, SHA-256 content hash, multihash helpers, JCS |
| PEM | strict PEM decode and encode with label, size, line-width, and line-ending policy |
| Protobuf | `reallyme.codec.v1` codec error envelopes |

## Install

### Rust

```sh
cargo add reallyme-codec
```

The Rust crates require Rust `1.96.0` or newer. The default feature set enables
every codec family. Consumers that need a smaller dependency surface can select
only the families they use:

```toml
reallyme-codec = { version = "0.1.20", default-features = false, features = [
  "base64url",
  "multikey",
] }
```

### Swift

```swift
.package(
    url: "https://github.com/reallyme/codec",
    from: "0.1.20"
)
```

```swift
.product(name: "ReallyMeCodec", package: "codec")
```

### Kotlin

```kotlin
dependencies {
    implementation("me.really:codec:0.1.20")
}
```

### Android

```kotlin
dependencies {
    implementation("me.really:codec-android:0.1.20")
}
```

### TypeScript

```sh
npm install @reallyme/codec
```

For production deployments, pin exact package versions, release tags, or Git
revisions so codec behavior remains identical across all language lanes.

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

Swift, Java, and Kotlin do not reimplement codecs. Public SDK releases ship
prebuilt Rust FFI/JNI libraries and load them through SwiftPM or Maven package
artifacts. `ReallyMeCodecRustCAbiLibrary(path:)` and
`ReallyMeCodecRustNativeProvider.loadLibrary(path)` remain available for local
development against a freshly built `reallyme-codec-ffi` library.

## Protobuf

The importable wire/config contract lives at
[`proto/reallyme/codec/v1/codec.proto`](proto/reallyme/codec/v1/codec.proto).
Service, application, and storage protos can import it when they need stable
codec identifiers or non-secret error envelopes.

Fixed-shape runtime results also have protobuf forms at the FFI/WASM boundary.
Use the `*Proto` methods, such as `multikeyParseProto`, `decodePemProto`, and
`dagCborVerifyCidProto`, when another protocol layer needs binary protobuf
bytes instead of the SDK JSON convenience shapes.

The generated proto surfaces are available through:

| Language | Proto surface |
|---|---|
| Rust | `reallyme-codec-proto` |
| Swift | `ReallyMeCodecProto` |
| Kotlin | `me.really.codec.v1` |
| TypeScript | `@reallyme/codec/proto` |

See [docs/protobuf.md](docs/protobuf.md) for the boundary rules.

## Documentation

- [CONTRACT.md](CONTRACT.md) - package, proto, and SDK boundary contract
- [SECURITY.md](SECURITY.md) - vulnerability reporting and security model
- [SECURITY_MEMORY_MODEL.md](SECURITY_MEMORY_MODEL.md) - codec boundary and memory handling model
- [docs/protobuf.md](docs/protobuf.md) - protobuf generation and ownership rules
- [docs/rust-publishing.md](docs/rust-publishing.md) - Rust crate release order

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).

## Copyright And Trademarks

Copyright © 2026 by ReallyMe LLC.

ReallyMe<sup>®</sup> is a registered trademark of ReallyMe LLC.
