<!--
SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved

SPDX-License-Identifier: Apache-2.0
-->

# Protobuf

The codec protobuf operation contract lives in the publishable proto crate at
[`../crates/proto/proto/reallyme/codec/v1/codec.proto`](../crates/proto/proto/reallyme/codec/v1/codec.proto).
Use it when another protocol needs stable codec error envelopes, fixed-shape
codec results, codec-owned configuration values, or the single executable
`CodecOperationRequest` boundary.

The proto crate defines messages only and intentionally declares no service.
Executable dispatch is owned by the main `reallyme-codec` crate so native Rust
APIs remain typed and ergonomic rather than being forced through serialization.
For cross-language boundaries, the generated protobuf messages are the
canonical request, response, and error operation contract. Do not add parallel Rust,
Swift, Kotlin, or TypeScript DTOs for those wire shapes unless the protobuf
schema is updated first and the adapter type is only a thin generated-type
facade.

Generated surfaces:

| Language | Surface |
|---|---|
| Rust | `reallyme-codec-proto` with the `generated` feature |
| Swift | `ReallyMeCodecProto` |
| Kotlin | generated `me.really.codec.v1` types |
| TypeScript | `@reallyme/codec/proto` |

Schema shape:

- Fixed-shape codec outputs have dedicated result messages. A caller should not
  need to parse generic maps, opaque JSON, or stringly typed tuples to read
  multicodec metadata, multikey parse output, CID verification, or PEM decode
  output.
- Error envelopes are split by codec family with a `oneof`. The individual
  family messages intentionally share `CodecErrorReason`; the reason enum is
  the stable public code while the `oneof` preserves the failing subsystem.
- Individual multicodec algorithms are data, not message types. They are carried
  by `name`, `algorithm_name`, `tag`, `key_material_kind`, and length fields in
  `CodecMulticodecSpec` and `CodecMultikeyParseResult`.
- Raw codec payloads such as DER, public-key bytes, canonical CBOR, and CIDs
  stay in their native byte/string fields. That keeps Rust as the source of
  truth and avoids hand-rolled Swift, Kotlin, or TypeScript codec logic.

Errors crossing package, RPC, storage, telemetry, FFI, or SDK boundaries must
map into `CodecErrorReason`. They must not include raw invalid input, PII,
secrets, or backend exception text.

All transport adapters converge on the same model:

1. Encode exactly one generated `CodecOperationRequest`.
2. Call the binary operation boundary (or its generated ProtoJSON request
   adapter).
3. Decode exactly one binary `CodecOperationResponse`.
4. Match its generated outcome oneof, then match the operation-specific result
   oneof or consume the typed `CodecError` directly.

There is no out-of-band operation selector and no JSON result envelope.
Operation-specific SDK helpers are conveniences that construct the generated
request before entering this same boundary:

| SDK | Generic binary request | Generic generated ProtoJSON request |
|---|---|---|
| Rust | `process_operation_response` | `process_operation_response_json` |
| TypeScript | `processOperation` | `processOperationJson` |
| Swift | `processOperation(_:)` | `processOperationJson(_:)` |
| Kotlin / Java | `processOperation(byte[])` | `processOperationJson(byte[])` |

Every typed SDK method constructs the corresponding generated request and
requires the exact generated result variant. No operation-specific `*Proto`
facade or opaque payload decoder is retained.

The JSON transport is only the generated ProtoJSON view of
`CodecOperationRequest`. Unknown fields, malformed JSON, invalid enum values,
oversized JSON, and decoded messages that exceed the protobuf cap fail inside a
typed `CodecOperationResponse` error outcome. There are no SDK-specific JSON
option DTOs or parallel structured JSON dispatch paths.

Generation is intentionally checked in. After installing `protoc-gen-buffa`
version `0.9.0`, regenerate and harden the artifacts with the complete enforced
pipeline:

```sh
buf lint
buf generate
node scripts/redact_codec_proto_debug.mjs
node scripts/redact_codec_proto_debug.mjs --check-idempotent
cargo fmt --package reallyme-codec-proto
```

The sensitivity manifest at `scripts/codec_proto_sensitivity.mjs` must classify
every protobuf `bytes` and `string` field exactly once as sensitive or
intentionally public. Do not add a scalar field without making that security
decision. CI also runs `buf breaking --against origin/main`, the pinned
hardening pipeline, and a generated-tree diff to catch schema drift or stale or
unhardened artifacts.

`CodecPemDecodeRequest.pem`, `CodecPemDecodeResult.der`, parsed public keys,
DAG-CBOR payloads, and result-envelope payloads can contain private or
privacy-sensitive material. Generated Rust owned messages zeroize these fields
on drop, sensitive Rust owned-view handles are not generated, and generated
`Debug` output is redacted. This does **not** make generated messages generally
log-safe: the required Rust `serde::Serialize` and borrowed-view serialization
paths emit sensitive fields verbatim for ProtoJSON. Generic serde logging,
structured tracing fields, `serde_json::to_*`, reflection, and similar dump
paths can therefore expose PEM, DER, payloads, multikeys, and public-key bytes.
Use serialization only as an explicit transport operation with a controlled
destination, and never pass a sensitive generated message to generic logging or
telemetry.

Generated Swift sensitive messages also shadow the
concrete `textFormatString` overloads. SwiftProtobuf's text-format methods are
protocol-extension methods, however, so code that first erases a sensitive
message to `any SwiftProtobuf.Message` can bypass those concrete overloads and
traverse the fields. Do not type-erase, text-format, reflect, interpolate, or
log sensitive protobuf messages; decode only the required fields and keep
those values inside the documented owner lifetime. The constant hash used by
these Swift messages deliberately avoids secret-dependent hashing at the cost
of collisions; attacker-sized `Set` or dictionary key workloads can degrade
toward quadratic behavior.

Generated Java sensitive messages use the same constant-hash policy. This
prevents secret-dependent hash state but deliberately makes those message types
poor keys for hash collections and can make attacker-sized `HashMap`/`HashSet`
workloads degrade toward quadratic behavior.

TypeScript protobuf-es messages are plain JavaScript objects and cannot install
reliable per-message redaction hooks. `console.log`, `JSON.stringify`,
protobuf-es JSON conversion, object inspection, spreading, and structured
logging can expose their byte and string fields. Treat every TypeScript message
classified as sensitive as a raw secret-bearing object: never log or inspect it,
keep its lifetime short, and wipe mutable `Uint8Array` owners as soon as the
transport operation completes. Redaction in Rust, Swift, or Java does not carry
across into the TypeScript runtime model.

Swift and Kotlin do not expose PEM-specific protobuf builders because those
runtimes require immutable `Data` or `ByteString` owners. Managed-runtime
callers must minimize copies and wipe mutable transient byte arrays as soon as
practical.
