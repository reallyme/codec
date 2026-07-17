# reallyme-codec

Byte-format and canonical encoding utilities for Rust, TypeScript, Swift, Java and Kotlin.

## Published Surfaces

| Surface | Distribution | Native path |
|---|---|---|
| Rust | `reallyme-codec` on crates.io | Pure Rust |
| TypeScript | `@reallyme/codec` on npm | Rust WASM |
| Swift | `ReallyMeCodec` through SwiftPM | Rust C ABI |
| Java/Kotlin | `me.really:codec` on Maven Central | Rust JNI |
| Android | `me.really:codec-android` AAR | Rust JNI in `jniLibs` |
| Protobuf | `reallyme.codec.v1` schema | Generated bindings |

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
reallyme-codec = { version = "0.1.22", default-features = false, features = [
  "base64url",
  "multikey",
] }
```

### Swift

```swift
.package(
    url: "https://github.com/reallyme/codec",
    from: "0.1.22"
)
```

```swift
.product(name: "ReallyMeCodec", package: "codec")
```

### Kotlin

```kotlin
dependencies {
    implementation("me.really:codec:0.1.22")
}
```

### Android

```kotlin
dependencies {
    implementation("me.really:codec-android:0.1.22")
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

Swift and JVM packages include native libraries. See their package READMEs for installation and local-library loading.

## Protobuf

The importable wire/config contract lives in the publishable proto crate at
[`crates/proto/codec/proto/reallyme/codec/v1/codec.proto`](crates/proto/codec/proto/reallyme/codec/v1/codec.proto).
Service, application, and storage protos can import it when they need stable
codec identifiers or non-secret error envelopes.

The executable transport boundary accepts one generated
`CodecOperationRequest` and returns one binary `CodecProtoResultEnvelope`.
Operation-specific `*Proto` SDK helpers are request builders over that same
entrypoint; they are not separate wire APIs. Native Rust callers retain the
typed codec APIs and do not need to serialize.

ProtoJSON uses protobuf enum names and standard padded Base64 for bytes. JSON requests return binary protobuf responses. Prefer binary protobuf or native APIs for sensitive data.

The generated proto surfaces are available through:

| Language | Proto surface | Generic execution |
|---|---|---|
| Rust | `reallyme-codec-proto` | `process_proto`, `process_proto_json` |
| Swift | `ReallyMeCodecProto` | `processProto`, `processProtoJson` |
| Kotlin / Java | `me.really.codec.v1` | `processProto`, `processProtoJson` |
| TypeScript | `@reallyme/codec/proto` | `processProto`, `processProtoJson` |

See [docs/protobuf.md](docs/protobuf.md) for the boundary rules.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).

## Copyright And Trademarks

Copyright © 2026 by ReallyMe Technologies Limited.

ReallyMe® is a registered trademark of ReallyMe LLC.

Copyright © 2026 ReallyMe Technologies Limited. All rights reserved
