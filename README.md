<div align="center">

# ReallyMe Codec

**Cross-platform Codec**

[![Code Checks](https://github.com/reallyme/codec/actions/workflows/ci.yml/badge.svg)](https://github.com/reallyme/codec/actions/workflows/ci.yml)
[![reallyme-codec](https://img.shields.io/crates/v/reallyme-codec?label=reallyme-codec&color=0f766e)](https://crates.io/crates/reallyme-codec)
[![npm codec](https://img.shields.io/npm/v/@reallyme/codec?label=npm%20codec&color=0f766e)](https://www.npmjs.com/package/@reallyme/codec)
[![Maven codec](https://img.shields.io/maven-central/v/me.really/codec?label=maven%20codec&color=0f766e)](https://central.sonatype.com/artifact/me.really/codec)
[![Security Policy](https://img.shields.io/badge/security-policy-0f766e)](SECURITY.md)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](LICENSE-MIT)

</div>

ReallyMe Codec keeps encodings consistent across Rust, TypeScript, Swift, Java,
and Kotlin. It covers base encodings, multiformats, canonical CBOR/JCS, PEM armor,
DER, TLV framing, and the protobuf operation contract. Rust implements the codec
behavior; TypeScript calls Rust through WASM, Swift through the C ABI, and
Java/Kotlin through JNI.

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
| Rust | `reallyme-codec` on crates.io | Pure Rust; [API](https://docs.rs/reallyme-codec) |
| TypeScript | `@reallyme/codec` on npm | Rust WASM; [setup](packages/ts/README.md) |
| Swift | `ReallyMeCodec` through SwiftPM | Rust C ABI; [setup](packages/swift/README.md) |
| Java/Kotlin | `me.really:codec` on Maven Central | Rust JNI; [setup](packages/kotlin/README.md) |
| Android | `me.really:codec-android` AAR | Rust JNI in `jniLibs`; [setup](packages/kotlin-android/README.md) |
| Protobuf | `reallyme.codec.v1` schema | Generated bindings |

## Source Map

| Area | Path |
|---|---|
| Rust facade and operation dispatch | `crates/codec` |
| Primitive codecs | `crates/base64`, `crates/cbor`, and the other codec leaf crates |
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
| General CBOR (Rust) | borrowed framing and received spans, arbitrary tags, unsorted unique maps, full integer range, floats, protected encoding |
| PEM | armor encoding, single-block and bundle decoding, explicit labels and limits; decoding permits public keys by default |
| DER | strict schema-directed ASN.1 DER decoding |
| TLV | borrowed framing for definite BER and DER identifiers, lengths and values |
| JSON | bounded parsing and projection into owned Serde types |
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

Swift 6.3 or newer is required.

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

ReallyMe Codec is pre-1.0. We follow the Rust community convention for
[0.x compatibility](https://doc.rust-lang.org/cargo/reference/semver.html):
breaking changes increment the minor version, such as `0.2.x` to `0.3.x`, and
compatible fixes increment the patch version, such as `0.4.0` to `0.4.1`.

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

Swift, using the released SwiftPM package:

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

The SDKs provide typed builders for deterministic CBOR:

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

## Formats and limits

- Base64 uses the standard padded alphabet; base64url is unpadded. Both reject
  nonzero pad bits. Hex accepts lowercase only.
- Deterministic CBOR uses RFC 8949 section 4.2.3 length-first map-key ordering.
  It supports integer or text map keys and exact integers, but rejects floats,
  tags, indefinite lengths and unsupported simple values. Limits are 1 MiB for
  encoded input/output, 64 container levels, 65,536 nodes and 16,384 entries per
  container. Map keys count as nodes.
- DAG-CBOR supports text map keys and signed 64-bit integers. This package's
  profile excludes floats and tags.
- JCS is a strict RFC 8785 subset. It rejects duplicate properties, unpaired
  surrogates, Unicode noncharacters and integer-valued numbers outside ±(2^53−1), including `1e19`.
- CID parsing checks syntax, not content integrity. CIDv1 can contain unknown
  hash codes or truncated digests. Use `verify_dag_cbor_cid` / `dagCborVerifyCid`
  to verify canonical DAG-CBOR against its complete SHA-256 CID in lowercase
  base32.
- PEM decoding permits public-key labels by default. Select private-key labels
  explicitly. Armor decoding does not validate the decoded key or certificate.
- DER checks values against a supplied schema. TLV framing checks only outer
  headers and extents; it does not recursively validate ASN.1 values.

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

By default, general map keys use [RFC 8949 section 5.6.1](https://www.rfc-editor.org/rfc/rfc8949.html#section-5.6.1)
basic-model equality: integer and float types are distinct; float zero signs are
equivalent; NaNs compare right-zero-extended significands; arrays are ordered and
maps compare unordered pairs. Registered-tag or application-specific equivalence
requires additional consumer checks. `encode` consumes borrowed `ValueRef` inputs,
preserves map order, and embeds `Received` items verbatim; an incompatible profile
rejects those bytes rather than normalizing them. Its output zeroizes on drop.

`Limits` lets callers configure input/output bytes, depth, nodes, container entries,
aggregate byte/text contents, and work. Entries default to 4,096 per container;
`Limits::with_entries(65_536)` explicitly permits larger arrays, maps, or chunked
strings. Hard ceilings are 1 MiB per input/output, 64 open containers/tags,
65,536 total nodes, 65,536 entries per container, 1 MiB per aggregate string
category, and 64 Mi work credits (16 Mi by default). Every container and child
counts toward the node budget, so a flat array can contain at most 65,535 scalar
elements. Raising the entry limit leaves all other budgets unchanged. Complex
keys can exhaust the work budget before other limits.

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

DAG-CBOR remains a separate profile with CID helpers; the supported subset
uses text map keys and signed 64-bit integers, with no floats or tags. Its structured
encode/decode methods use the same generated operation boundary and Rust
semantic implementation.

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

Swift, Java, and Kotlin do not reimplement codecs. Public SDK releases ship
prebuilt Rust FFI/JNI libraries and load them through SwiftPM or Maven package
artifacts. `ReallyMeCodecRustCAbiLibrary(path:)` and
`ReallyMeCodecRustNativeProvider.loadLibrary(path)` remain available for local
development against a freshly built `reallyme-codec-ffi` library.

## Protobuf API

The [schema](crates/proto/proto/reallyme/codec/v1/codec.proto) defines requests,
results, options and typed errors. Generated Rust messages are available in
`reallyme-codec-proto` with its `generated` feature; TypeScript exports them
through `@reallyme/codec/proto`.
Swift and Kotlin provide serialized operation methods.

Enable `operation-contract` for Rust dispatch. Binary and ProtoJSON requests
both return binary protobuf responses. See [protobuf usage](docs/protobuf.md).

The generated proto surfaces are available through:

| Language | Proto surface | Generic execution |
|---|---|---|
| Rust | `reallyme-codec-proto` | `process_operation_response`, `process_operation_response_json` |
| Swift | Internal generated wire types | `processOperation`, `processOperationJson` |
| Kotlin / Java | `me.really.codec.v1` | `processOperation`, `processOperationJson` |
| TypeScript | `@reallyme/codec/proto` | `processOperation`, `processOperationJson` |

## Documentation

- [CONTRACT.md](CONTRACT.md) - package, proto, and SDK boundary operation contract
- [SECURITY.md](SECURITY.md) - vulnerability reporting and security model
- [SECURITY_MEMORY_MODEL.md](SECURITY_MEMORY_MODEL.md) - codec boundary and memory handling model
- [docs/protobuf.md](docs/protobuf.md) - binary protobuf and ProtoJSON usage

## Guarded decoding

With the `json` feature, `json::decode` returns a bounded document and
`json::from_slice` projects it into an owned Serde target. Object order and
number spelling are preserved; duplicate decoded names and exceeded limits
return typed errors.

`base64::base64_to_bytes_with_limits` takes independent schema-defined encoded
and decoded byte budgets and returns `Zeroizing<Vec<u8>>`. The existing decoder
retains its 2,097,152-byte input default; the existing scalar operation retains
its separate 1 MiB transport cap. Configurable budgets do not override transport
limits.

See [data handling](SECURITY_MEMORY_MODEL.md) before processing sensitive data.
Input limits do not impose a total heap quota, and managed copies cannot be
reliably erased.

## Build and test

From the repository root:

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo check -p reallyme-codec-wasm --target wasm32-unknown-unknown
cargo audit
cargo deny check
```

SDK build/test commands are in their package READMEs. See
[fuzzing](fuzz/README.md) for corpus replay and fuzz targets. On Linux, install
Kani 0.68.0 and `jq`, then run `bash scripts/check-formal.sh` for the bounded proof
suite.

To regenerate protobuf bindings, make Buf CLI 1.75.0, Node.js 20.19 or newer,
Rust and Swift available. Install the pinned local tools and run:

```sh
npm ci --prefix packages/ts
cargo install reallyme-protoc-gen-buffa --version '=0.1.3' --locked
cargo install reallyme-protoc-gen-buffa-packaging --version '=0.1.3' --locked
scripts/install_local_proto_tools.sh
PATH="$PWD/.build/codec-proto-tools/bin:$PATH" node scripts/generate_codec_proto.mjs
```

The local protobuf tool installer requires macOS. Keep the resulting generated
files with the schema changes.

## Security

Report vulnerabilities through [the security policy](SECURITY.md).

## License

Licensed under either the [MIT License](LICENSE-MIT) or the
[Apache License, Version 2.0](LICENSE-APACHE), at your option.

Third-party components retain their own licenses and notices.

## Copyright And Trademarks

Copyright © 2026 by ReallyMe Technologies Limited.

ReallyMe® is a registered trademark of ReallyMe LLC.
