# Protobuf

The codec protobuf operation contract lives in the publishable proto crate at
[`../crates/proto/proto/reallyme/codec/v1/codec.proto`](../crates/proto/proto/reallyme/codec/v1/codec.proto).
Use it when another protocol needs stable codec error envelopes, fixed-shape
codec results, codec-owned configuration values, or the executable
`CodecApiOperationRequest` boundary. Rust consumers enable the main crate's
`operation-contract` feature to access executable dispatch.

The proto crate defines messages only and intentionally declares no service.
Executable dispatch is owned by the main `reallyme-codec` crate so native Rust
APIs remain typed and ergonomic rather than being forced through serialization.
For every public operation, the generated protobuf messages are the
canonical request, response, and error operation contract. Do not add parallel Rust,
Swift, Kotlin, or TypeScript DTOs for those wire shapes unless the protobuf
schema is updated first and the adapter type is only a thin generated-type
facade.

Generated surfaces:

| Language | Surface |
|---|---|
| Rust | `reallyme-codec-proto` with the `generated` feature |
| Swift | Generated wire types internal to `ReallyMeCodec` |
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

Presence is part of the schema. Use `optional` for a scalar whose absence
differs from its default value, and a message or `oneof` for structured
alternatives. Do not add a separate `has_*` boolean. Generated message-presence
accessors are supported; they do not introduce another wire field. An enum's
`UNSPECIFIED` value means absence only when the operation explicitly permits it;
required policy enums reject it.

Optional limits and CBOR/DER observations preserve explicitly supplied zero or
false values through binary protobuf and ProtoJSON. Optional binding algorithms
also preserve an empty string. PEM's existing zero-means-default limits retain
their documented semantics. Required byte inputs deliberately treat missing
and empty identically; their operation defines whether empty is accepted.

The existing multicodec and multikey result messages retain their published
length/variable field pairs. These describe length rules rather than presence:
multicodec metadata supports fixed, variable, and not-applicable states, while
parsed keys support fixed or variable. SDK adapters reject contradictory pairs
with the typed provider error, and fixed parsed-key lengths must match the key
bytes. Field numbers and binary encodings remain unchanged.

Errors crossing package, RPC, storage, telemetry, FFI, or SDK boundaries must
map into `CodecErrorReason`. They must not include raw invalid input, PII,
secrets, or backend exception text.

Structured operation adapters converge on the same model:

1. Encode exactly one generated `CodecApiOperationRequest`.
2. Call the binary operation boundary (or its generated ProtoJSON request
   adapter).
3. Decode exactly one binary `CodecApiOperationResponse`.
4. Match its generated outcome oneof, then match the operation-specific result
   oneof or consume the typed `CodecApiError` directly.

The original `CodecOperation*` and `CodecError` messages retain their generated
Rust source shape. They share their field numbers and payload messages with
the expanded `CodecApiOperation*` and `CodecApiError` envelopes. Existing
callers keep their types and byte representations; additional operations use
the expanded generated envelope through the same binary entry points.

This generated operation lane has no out-of-band operation selector and no
JSON result envelope.
Operation-specific SDK helpers are conveniences that construct the generated
request before entering this same boundary:

| SDK | Generic binary request | Generic generated ProtoJSON request |
|---|---|---|
| Rust | `process_operation_response` | `process_operation_response_json` |
| TypeScript | `processOperation` | `processOperationJson` |
| Swift | `processOperation(_:)` | `processOperationJson(_:)` |
| Kotlin / Java | `processOperation(byte[])` | `processOperationJson(byte[])` |

Typed SDK methods construct the corresponding generated request and require
the exact generated result variant. The operation envelope covers base
encodings, predicates, JCS, general CBOR, schema-directed DER, TLV framing, and structured
metadata. Existing scalar signatures use generated borrowed request views without
allocating an extra request owner. Their legacy selectors and status
values are defined by protobuf; retired metadata selectors still return
`CODEC_INVALID_ARGUMENT`.

Scalar semantics live in `operation_contract::scalar_ops`. The generated
dispatcher and the legacy FFI/WASM scalar entry points call that same
implementation; the internal adapter crate only re-exports it. Borrowed
signatures retain their ownership and error types without serializing native
calls or duplicating operation policy.

The shared `vectors/codec-operation-vectors.json` suite covers every executable
schema operation. Rust checks its binary request bytes against Buffa and verifies
result values and both transports' typed rejection behavior. TypeScript, Swift,
and Kotlin execute the same cases and require byte-identical binary/ProtoJSON
responses. Release checks compare vector coverage with the schema so a new
operation cannot silently omit cross-lane conformance coverage.

General-CBOR receipts carry checked spans into the original request source,
including the received header, body, ordering, and IEEE floating-point bits.
DER schemas select the existing strict reader methods. `CodecApiError` adds
the precise generated general-CBOR, DER, and TLV reason enums while retaining
`CodecErrorReason` categories and explicit caller/provider attribution. Borrowed readers and
caller-supplied callbacks retain their signatures and ownership guarantees;
the generated contract owns their options, operations, and typed reasons.
Owned wire receipt collections are capped at twice the configured entry limit,
which accommodates alternating map keys and values. Metadata collection sizes
are checked before allocation. Borrowed native cursors keep their streaming
behavior. Generated sensitive lists erase retired storage when they grow.
Sensitive scalar temporaries have zeroizing owners during JSON decoding, so a
later malformed field cannot bypass cleanup. Hardening retains the pinned
generator's ProtoJSON rules, including null defaults and quoted integers.

Provider lifecycle requests describe verification and loading configuration.
Filesystem access and dynamic loading stay in the host adapter, outside the
pure codec core.
`CodecProviderOperation*` also describes initialization from verified WASM bytes
and recovery from the private cached module. Recovery takes an empty request
and never accepts a caller-provided compiled module.

The JSON transport is only the generated ProtoJSON view of
`CodecApiOperationRequest`. Unknown fields, malformed JSON, invalid enum values,
oversized JSON, and decoded messages that exceed the protobuf cap fail inside a
typed `CodecApiOperationResponse` error outcome. There are no SDK-specific JSON
option DTOs or parallel structured JSON dispatch paths.
An unknown ProtoJSON enum name is a malformed JSON transport value; an unknown
numeric enum carried by binary protobuf is a malformed request after decoding.
These transport errors retain their distinct reasons. Once a request decodes,
the same semantic error reasons apply to both transports.

Use the binary protobuf entry point for private keys, decrypted plaintext, or
other secret-bearing requests. JSON escape processing and duplicate field
decoding can create immutable or temporary copies that cannot be reliably
zeroized by the protobuf boundary. In particular, a JSON string that contains
any escape sequence is unescaped into the JSON parser's internal scratch
buffer, which is released without being wiped.

Generation is checked in. Install the exact published fork packages and run
one reproducible generation driver:

```sh
cargo install reallyme-protoc-gen-buffa --version '=0.1.1' --locked
cargo install reallyme-protoc-gen-buffa-packaging --version '=0.1.1' --locked
node scripts/generate_codec_proto.mjs
```

The driver verifies both renamed executables and their exact version output,
runs Buf, emits schema-derived support artifacts, applies the separate native
SDK safeguards, and formats Rust. ReallyMe Buffa generates Rust cleanup,
redaction, strict JSON admission, and guarded collections natively. No script
rewrites generated Rust. Custom byte/string owners supply the bounded decoding
and explicit JSON policies selected in `buf.gen.yaml`.

The sensitivity manifest at `scripts/codec_proto_sensitivity.mjs` must classify
every protobuf `bytes` and `string` field exactly once as sensitive or
intentionally public. Do not add a scalar field without making that security
decision. CI requires a published version tag for `buf breaking` and also
checks pull requests against `origin/main`. It then checks all generated
outputs for freshness with the pinned generation pipeline.

For local release checks, make the pinned Release Readiness 0.7.0 checkout
available. The wrapper checks the runner, core, and manifest digests before
execution. Set `REALLYME_RELEASE_READINESS_ROOT` to use an existing checkout.

```sh
git clone https://github.com/reallyme/release-readiness.git .release-readiness
git -C .release-readiness checkout --detach 18a6d91d99837e80f92e41fe04effb0b043d754e
scripts/build_kotlin_native_resource.sh build/sdk-native-resources
node scripts/write_native_manifest.mjs build/sdk-native-resources \
  build/sdk-native-resources/me/really/codec/native/native-manifest.json
REALLYME_CODEC_SWIFTPM_RUNTIME_FFI=1 \
REALLYME_CODEC_FFI_LIBRARY_PATH="$PWD/target/release/libreallyme_codec_ffi.dylib" \
REALLYME_CODEC_KOTLIN_NATIVE_RESOURCES_DIR="$PWD/build/sdk-native-resources" \
  node scripts/check_release_readiness.mjs --generated-freshness
```

This command uses an Apple Silicon macOS host for the Swift runtime lane and
also requires the pinned WASM tooling, the TypeScript package's installed
dependencies, and Java for Kotlin. The release workflows verify each packaged
artifact separately after consuming the schema and SDK gate for the same
revision.

`CodecPemDecodeRequest.pem`, `CodecPemDecodeResult.der`, parsed public keys,
DAG-CBOR payloads, and result-envelope payloads can contain private or
privacy-sensitive material. Generated Rust owned messages zeroize these fields
on drop. Buffa omits standard owned-view handles and `HasMessageView` for
sensitive messages and their containers. Generated `Debug` output is redacted.
Callers own the backing storage of borrowed views. A caller that explicitly
constructs a generic Buffa view owner must choose wiping storage for secrets.
Generated messages
are not generally log-safe: Rust `serde::Serialize` and borrowed-view serialization
paths emit sensitive fields verbatim for ProtoJSON. Generic serde logging,
structured tracing fields, `serde_json::to_*`, reflection, and similar dump
paths can therefore expose PEM, DER, payloads, multikeys, and public-key bytes.
Use serialization only as an explicit transport operation with a controlled
destination, and never pass a sensitive generated message to generic logging or
telemetry.

ReallyMe Buffa 0.1.1 omits rejected enum names and numbers from enum JSON
diagnostics. With `zeroize` enabled, standalone enum readers clear names
when Serde transfers ownership of the string. The deserializer remains
responsible for its own buffers. Codec message admission returns a fixed
typed failure for invalid JSON.

SwiftProtobuf's generic `Message` text formatter traverses fields, even if a
concrete message shadows `textFormatString`. For this reason the Swift package
compiles generated wire messages as internal types in `ReallyMeCodec` and does
not vend a generated-message product. The public Swift API accepts and returns
serialized operation bytes and typed SDK results; application code cannot
obtain a generated message from the package for generic formatting. Internal
code must still treat generated messages as secret-bearing transport objects;
erasing one to `any SwiftProtobuf.Message` inside the SDK still exposes raw
fields to the generic text formatter.
The constant hash used by
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

Swift and Kotlin PEM facades accept mutable byte arrays and construct the
generated requests internally. The generated message types remain available
to generic operation callers; their `Data` or `ByteString` storage can introduce
additional managed copies. The Kotlin typed facades parse response bytes with
aliasing and finish conversion while the SDK still owns the response array; it
wipes that array before returning. Generic callers receiving raw response bytes
own their lifetime and cleanup. Minimize copies and wipe mutable transient byte
arrays as soon as practical.

## Definite TLV framing

The five TLV operations use the expanded operation envelope in family band
11000–11999. `CodecTlvSourceRequest` carries
sensitive source bytes and `CodecTlvOptions`; its profile must be explicitly set
to `ISO7816_DEFINITE_BER` or `ASN1_DER_FRAMING`. Omitted or unknown profiles fail
closed. Limits use protobuf presence: omission selects defaults, while zero
and values above the hard ceilings are invalid. Set `input_bytes` explicitly
to opt into a larger source limit; the transport envelope has room for the
16 MiB source and its binary or base64 JSON framing.

| Limit | Default | Hard ceiling |
| --- | --- | --- |
| Actual source bytes, including trailing input | 1 MiB | 16 MiB |
| Complete encoded identifier octets | 4 | 32 |
| Long-form length payload octets | 4 | 8 |

The length-octet limit excludes the initial discriminator. The returned
`length_octets` source span includes it. The ISO 7816 profile preserves the
complete received wire identifier, including continuation bits, and accepts
redundant definite length octets. End-of-contents and zero initial high-tag
groups are rejected; padding recovery belongs to the consuming protocol. The
DER framing profile requires a minimal
ASN.1 identifier and definite length for the outer header. Neither profile
recursively validates constructed contents or checks ASN.1 value semantics.
Use schema-directed `der_decode` for those guarantees. Indefinite lengths and
the reserved length discriminator `FF` are rejected.

Use `tlv_read_identifier` for concatenated wire identifiers or staged parsing:
it leaves the following octets unconsumed, even if they would not form a length.
Use `tlv_read_length` after the identifier has been read: it returns a
length-field span, declared uint64 length, and bytes consumed. Neither
operation requires a value. They use the same profile rules as
`tlv_read_header`, `tlv_frame_prefix`, and `tlv_frame_exact`.

A header receipt contains checked identifier, length-field, and header spans,
the uint64 declared value length, and the header bytes consumed. It can succeed
when no value bytes have arrived, including a declared length larger than the
platform's address space. All supplied bytes after that header are the bounded
remainder; no contents span is invented. Prefix framing requires the complete
value before returning received and contents spans and total consumption.
Exact framing additionally rejects every trailing byte. Declared lengths are
compared with supplied bytes before conversion to a native offset, so truncated
values have the same typed failure on 32-bit and 64-bit hosts.

The `tlv` Rust feature exposes borrowed views over this generated contract.
Native and wire calls share the same framing implementation. Native calls
borrow all payload bytes; only bounded receipt metadata is owned. The source
owner remains responsible for wiping sensitive input. `CodecApiError.tlv`
contains a generated detail reason and explicit caller/provider attribution.
`vectors/tlv-framing-vectors.json` supplies independent exact wire goldens for
native Rust, protobuf and ProtoJSON dispatch, WASM, Swift C ABI, and Kotlin JNI.
