<!--
SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved

SPDX-License-Identifier: Apache-2.0
-->

# Protobuf

The codec protobuf contract lives at
[`../proto/reallyme/codec/v1/codec.proto`](../proto/reallyme/codec/v1/codec.proto).
Use it when another protocol needs stable codec error envelopes, fixed-shape
codec results, or codec-owned configuration values.

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

Runtime adapters expose protobuf bytes for fixed-shape results that would
otherwise require hand-parsed JSON at protocol boundaries. Prefer these binary
methods when another crate or package is consuming codec output:

| Result | Method family |
|---|---|
| Multicodec metadata and tables | `multicodec*Proto` |
| Multikey parse result | `multikeyParseProto` |
| DAG-CBOR CID verification | `dagCborVerifyCidProto` |
| PEM decode metadata and DER | `decodePemProto` / `pemDecodeProto` |

JSON convenience methods remain available for application code, but protocol
and FFI consumers should treat the protobuf forms as the stable binary
contract.

Generation is intentionally checked in. Regenerate all protobuf artifacts with
`buf generate` after installing `protoc-gen-buffa` version `0.8.1`, then run
`cargo fmt --package reallyme-codec-proto`; CI runs `buf lint`,
`buf breaking --against origin/main`, generation, Rust formatting, and
`git diff --exit-code` to catch schema drift or stale generated files.

`CodecPemDecodeResult.der` can carry private-key DER. Protobuf libraries in
Swift, Kotlin, Java, TypeScript, and Rust store decoded `bytes` fields in their
ordinary byte containers, so callers that request PEM protobuf output must move
the DER into an appropriate sensitive-buffer owner and wipe transient protobuf
objects as soon as practical.
