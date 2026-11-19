# ReallyMe Codec

Codecs for base encodings, multiformats, CBOR, CID, canonical JSON, PEM, DER and
TLV framing. Rust implements the operations; SDKs are available for TypeScript,
Swift, Java, Kotlin and Android.

## Install

| Language | Package | Usage |
|---|---|---|
| Rust | `reallyme-codec` | [Rust API](https://docs.rs/reallyme-codec) |
| TypeScript | [`@reallyme/codec`](https://www.npmjs.com/package/@reallyme/codec) | [Wasm setup](packages/ts/README.md) |
| Swift | `ReallyMeCodec` | [SwiftPM setup](packages/swift/README.md) |
| Java/Kotlin | `me.really:codec:0.4.4` | [JVM setup](packages/kotlin/README.md) |
| Android | `me.really:codec-android` AAR | [Android setup](packages/kotlin-android/README.md) |

Use matching package versions across languages. Swift consumers need a release
tag with a matching binary artifact; an untagged main checkout may not load a
provider. Rust leaf crates share the umbrella crate's release version.

```toml
[dependencies]
reallyme-codec = "0.4.4"
```

Rust requires version 1.100 or newer. Base encodings, CBOR/JCS, multiformats and
PEM are enabled by default. Enable `der`, `tlv`, `json`, `serde` or
`operation-contract` when needed. To select individual families:

```toml
reallyme-codec = { version = "0.4.4", default-features = false, features = ["base64url", "multikey"] }
```

```rust
use reallyme_codec::base64url::{base64url_to_bytes, bytes_to_base64url};

fn decode_example() -> Result<Vec<u8>, reallyme_codec::base64url::Base64UrlError> {
    base64url_to_bytes(&bytes_to_base64url(b"hello"))
}
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

Rust's `cbor::general` API supports borrowed framing, tags, received encodings,
float policies and unsorted unique maps. A framing receipt alone does not imply
semantic validity; use `validate_prefix` or `validate_exact` when required.
`Limits::with_entries(65_536)` permits larger containers than the 4,096-entry
default while retaining the byte, depth, node and work budgets. The 65,536-node
budget allows at most 65,535 scalar children in one flat array.

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

## Protobuf API

The [schema](crates/proto/proto/reallyme/codec/v1/codec.proto) defines requests,
results, options and typed errors. Generated Rust messages are available in
`reallyme-codec-proto`; TypeScript exports them through `@reallyme/codec/proto`.
Swift and Kotlin provide serialized operation methods.

Enable `operation-contract` for Rust dispatch. Binary and ProtoJSON requests
both return binary protobuf responses. See [protobuf usage](docs/protobuf.md).

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
Kani 0.68.0 and run `bash scripts/check-formal.sh` for the bounded proof suite.

To regenerate protobuf bindings, install the pinned local tools and run:

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
