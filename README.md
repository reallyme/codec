# ReallyMe Codec

Codecs for Rust, TypeScript, Swift, Java and Kotlin, covering base encodings, multiformats, CBOR, CID, canonical JSON and PEM.

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
| Canonical data | deterministic generic CBOR, DAG-CBOR, CID helpers, SHA-256 content hash, multihash helpers, JCS |
| General CBOR (Rust) | borrowed framing and received spans, arbitrary tags, unsorted unique maps, full integer range, floats, protected encoding |
| PEM | PEM decode and encode with label, size, line-width, and line-ending policy; decoding permits public keys by default |
| Protobuf | `reallyme.codec.v1` generated operation requests, generated ProtoJSON, and codec error envelopes |

## Install

### Rust

```sh
cargo add reallyme-codec
```

The Rust crates require Rust `1.100.0` or newer. Default features enable the
base-encoding, CBOR/JCS, multiformat, and PEM families. Consumers can enable the
strict DER reader or the borrowed `tlv` framing surface separately and select
only the families they use:

```toml
reallyme-codec = { version = "0.4.4", default-features = false, features = [
  "base64url",
  "multikey",
] }
```

The generated operation boundary is opt-in for Rust consumers: enable
`operation-contract` to use `reallyme_codec::operation_contract`. The `serde`
feature enables the base64url field adapters.

The `tlv` feature reads identifiers, definite lengths, or complete headers
with BER/ISO 7816 or canonical DER rules. It frames prefix or exact values
without copying payloads. Callers can opt into source limits up to 16 MiB.
See the
[TLV contract](docs/protobuf.md#definite-tlv-framing) for profiles, checked source
spans, limits, and typed failures. Schema-directed DER remains a separate strict
reader.

### Swift

```swift
.package(
    url: "https://github.com/reallyme/codec",
    from: "0.4.4"
)
```

```swift
.product(name: "ReallyMeCodec", package: "codec")
```

### Kotlin

```kotlin
dependencies {
    implementation("me.really:codec:0.4.4")
}
```

### Android

```kotlin
dependencies {
    implementation("me.really:codec-android:0.4.4")
}
```

### TypeScript

```sh
npm install @reallyme/codec
```

For production deployments, pin exact package versions or release tags so codec
behavior remains identical across language lanes. SwiftPM consumers must use a
release tag with its matching FFI artifact; a Git revision of `main` can have
an unbound manifest and make `ReallyMeCodec()` unavailable.

## Versioning

This package is pre-1.0. Review compatibility notes before upgrading.

For Rust consumers, pin to the minor line you have reviewed, for example
`reallyme-codec = "0.4"`. A full version written by `cargo add`, such as
`reallyme-codec = "0.4.4"`, uses Cargo caret semantics and remains on the same
`0.4.x` compatibility line. For npm, Maven, SwiftPM, and release artifacts used
in production, prefer exact versions or locked dependency files so every
language lane runs the same reviewed codec release.

## Quick Start

Rust:

```rust
use reallyme_codec::base64url::{base64url_to_bytes, bytes_to_base64url};

fn round_trip() -> Result<(), reallyme_codec::base64url::Base64UrlError> {
    let encoded = bytes_to_base64url(b"hello");
    let decoded = base64url_to_bytes(&encoded)?;
    assert_eq!(decoded, b"hello");
    Ok(())
}
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

TypeScript, after [initializing the WASM provider](packages/ts/README.md#usage):

```ts
import { ReallyMeCodec } from "@reallyme/codec";

const encoded = ReallyMeCodec.base64urlEncode(new Uint8Array([1, 2, 3]));
```

```swift
import Foundation

let value = ReallyMeDeterministicCbor.mapText([
    ("b", ReallyMeDeterministicCbor.unsigned(2)),
    ("a", ReallyMeDeterministicCbor.bytes(Data([0, 1, 2]))),
])
let cbor = try codec.deterministicCborEncodeData(value)
```

```kotlin
import me.really.codec.ReallyMeDeterministicCbor

val value = ReallyMeDeterministicCbor.mapText(
    linkedMapOf(
        "b" to ReallyMeDeterministicCbor.unsignedLong(2),
        "a" to ReallyMeDeterministicCbor.bytes(byteArrayOf(0, 1, 2)),
    ),
)
val cbor = ReallyMeCodec.deterministicCborEncode(value)
```

```ts
import { ReallyMeDeterministicCbor } from "@reallyme/codec";

const value = ReallyMeDeterministicCbor.mapText([
  ["b", ReallyMeDeterministicCbor.unsigned(2n)],
  ["a", ReallyMeDeterministicCbor.bytes(new Uint8Array([0, 1, 2]))],
]);
const cbor = ReallyMeCodec.deterministicCborEncode(value);
```

Rust consumers also have `reallyme_codec::cbor::general` (or
`codec_cbor::general` from `reallyme-codec-cbor`). `frame_prefix`/`frame_exact`
return borrowed framing receipts; `validate_prefix`/`validate_exact` additionally
check UTF-8 and semantic key uniqueness under an explicit `Profile`. Neither
operation interprets arbitrary tag registrations. `Profile::received()` permits
received widths and indefinite containers; `Profile::preferred()` requires
definite lengths, shortest arguments, and shortest exact floats. Both accept
unsorted unique maps. Optional map-order and finite-only/no-float policies are
independent selections.

`Frame::tagged_bytes()` exposes the complete received wrapper and contiguous
opaque byte-string contents separately. Its `validate_embedded` operation
requires tag 24 and exactly one embedded item; empty or trailing payloads fail.
Chunked strings expose borrowed chunks instead of inventing a contiguous span.
`Cursor` traverses received items with cumulative budgets and exact consumed and
remaining spans. Framing receipts do not imply basic validity, and borrowed
storage requires a caller-owned zeroizing buffer for sensitive input.

General map keys use RFC 8949 section 5.6.1 basic-model equality. Integer and float types are distinct; float zero signs are equivalent. Registered-tag and application-specific equivalence require additional caller checks. Encoding preserves map order and received items; incompatible profiles return an error.

`Limits` lets callers configure input/output bytes, depth, nodes, container entries,
aggregate byte/text contents, and work. Entries default to 4,096 per container;
`Limits::with_entries(65_536)` explicitly permits larger arrays, maps, or chunked
strings. Hard ceilings are 1 MiB per input/output, 64 open containers/tags,
65,536 total nodes, 65,536 entries per container, 1 MiB per aggregate string
category, and 64 Mi work credits (16 Mi by default). Every container and child
counts toward the node budget, so a flat array can contain at most 65,535 scalar
elements. Raising the entry limit leaves all other budgets unchanged. Complex keys can exhaust the work budget before other limits.

Deterministic-CBOR encode canonicalizes map ordering. Decode rejects duplicate
semantic map keys, non-canonical map order, non-minimal integer or length
forms, unsupported CBOR types, and inputs outside the documented resource
limits. The generic profile permits at most 64 nested containers, 65,536 nodes
(including map keys), and 16,384 entries per container. Encoded input and output
are each capped at 1 MiB; aggregate UTF-8 text and aggregate byte-string content
are each capped at 1 MiB.

Map keys use RFC 8949 §4.2.3 length-first ordering. This profile differs from
the bytewise ordering in RFC 8949 §4.2.1 when keys have different encoded
lengths. JCS is a strict RFC 8785 subset: integer-valued binary64 numbers
outside the interoperable ±(2⁵³−1) range are rejected, including `1e19`.

DAG-CBOR uses text map keys and signed 64-bit integers; floats and tags are outside the supported profile.

`is_valid_cid_string` / `isValidCidString` and `try_parse_cid` / `tryParseCid`
check bounded CID syntax. CIDv0 requires the full 32-byte SHA-256 multihash;
CIDv1 preserves opaque hash codes and 0–64 digest bytes when the declared length
matches. Unknown hash codes, truncated SHA-256 digests, and identity hashes can
pass these checks. Parsing establishes framing and resource bounds; hash
support and content integrity require verification. This separation preserves
the variable digest lengths carried by [multihash](https://multiformats.io/multihash/).
`verify_dag_cbor_cid` / `dagCborVerifyCid` validates a canonical DAG-CBOR block
and matches only CIDv1 DAG-CBOR with its complete 32-byte SHA-256 digest and
canonical lowercase base32 text.

Swift and JVM packages include native libraries. See their package READMEs for installation and local-library loading.

## Protobuf

The importable wire/config operation contract lives in the publishable proto crate at
[`crates/proto/proto/reallyme/codec/v1/codec.proto`](crates/proto/proto/reallyme/codec/v1/codec.proto).
Service, application, and storage protos can import it when they need stable
codec identifiers or non-secret error envelopes.

The executable transport boundary accepts one generated
`CodecOperationRequest` and returns one binary `CodecOperationResponse` whose
oneofs identify either the exact operation result or a typed `CodecError`.
Structured SDK methods build requests for that same entrypoint and require
the matching result variant. Base encodings and JCS call Rust through dedicated
scalar adapter functions. Native Rust callers retain typed codec APIs and do
not need to serialize.

ProtoJSON uses protobuf enum names and standard padded Base64 for bytes. JSON requests return binary protobuf responses. Prefer binary protobuf or native APIs for sensitive data.

The generated proto surfaces are available through:

| Language | Proto surface | Generic execution |
|---|---|---|
| Rust | `reallyme-codec-proto` | `process_operation_response`, `process_operation_response_json` |
| Swift | Internal generated wire types | `processOperation`, `processOperationJson` |
| Kotlin / Java | `me.really.codec.v1` | `processOperation`, `processOperationJson` |
| TypeScript | `@reallyme/codec/proto` | `processOperation`, `processOperationJson` |

See [docs/protobuf.md](docs/protobuf.md) for the boundary rules.

## License

Licensed under either the [MIT License](LICENSE-MIT) or the
[Apache License, Version 2.0](LICENSE-APACHE), at your option.

Third-party components retain their own licenses and notices.

## Copyright And Trademarks

Copyright © 2026 by ReallyMe Technologies Limited.

ReallyMe® is a registered trademark of ReallyMe LLC.
