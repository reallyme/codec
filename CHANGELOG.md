# Changelog

## 0.3.4 (unreleased)

- Accept the largest default PEM encoding with LF or CRLF line endings.
- Add borrowed general-CBOR framing, validation, cursors and float policies.
- Add bounded general-CBOR encoding that preserves received items and map order.
- Reduce work when validating large general-CBOR maps. Complex keys can still
  exhaust the configured work budget.
- Preserve general-CBOR policy during child inspection; report truncated strings
  and reject empty tag-24 payloads.
- Reject invalid native pointer arguments, forged TypeScript byte views and
  nonzero ProtoJSON pad bits. Map native-loader failures to typed errors.
- Verify bundled Wasm bytes before initialization and reject precompiled modules.
  Clear temporary PEM serialization copies.
- Update SwiftProtobuf to 1.40.0.

## 0.3.3

- Add opt-in, bounded, schema-directed strict DER readers and an exact
  SEQUENCE-of-two-unsigned-INTEGER decoder for Rust consumers. Curve widths,
  scalar ranges, and signature normalization remain in the cryptographic layer.
- Reject noncanonical DER lengths, INTEGER sign padding, BIT STRING padding,
  OID subidentifiers, SET OF ordering, and trailing data with typed errors.

## 0.3.2 (2026-11-08)

- Update copyright ownership to ReallyMe Technologies Limited.

## 0.3.1 (2026-10-03)

- Recover the TypeScript facade after unexpected Wasm exceptions and reject
  null raw Wasm string inputs.
- Clear JCS object values on drop and bound nested protobuf decoding.
- Classify malformed DAG-CBOR tags and floating-point encodings consistently
  across decoding and CID verification. Unknown multikey codec names now return
  the unsupported-codec class in native SDKs, matching TypeScript.

## 0.3.0 (2026-10-02)

- Bound allocations and nesting before decoding protobuf, ProtoJSON, CBOR,
  DAG-CBOR, base encodings, and PEM input.
- Reject malformed and non-canonical inputs with stable typed errors, including
  private-key multikey prefixes, invalid compressed-point tags, invalid CIDs,
  ambiguous JSON numbers, and non-canonical encodings.
- Restrict default PEM decoding to public-key labels. Private-key labels require
  an explicit allowlist.
- Tighten secret buffer ownership, zeroization, and redacted diagnostic output
  across the Rust core and language adapters.
- Align scalar error classes across Rust, Swift, Kotlin, and TypeScript; give
  canonical CBOR failures distinct protobuf reasons.

Rust source compatibility changes: `encode_protobuf` now returns a typed
`Result`; multicodec prefix stripping and DAG-CBOR CID verification now return
typed outcomes; key length metadata uses an enum; and `CborValue` no longer
implements `Clone` and now implements `Drop`, so by-value destructuring no
longer compiles. `write_lower_hex` now returns a typed `Result`.
`ParsedMultikey` fields are now private and exposed through
read-only accessors so callers cannot forge key metadata. Generated protobuf
error reasons also gained variants.

Compatibility changes in 0.3.0:

- The `codec_spec_proto` Rust crate and the Swift `ReallyMeCodecProto` product
  were removed. Swift callers use the `ReallyMeCodec` product; Swift exhaustive
  error switches must handle `providerUnavailable`, `nonCanonical`,
  `unsupportedCodec`, and `unsupportedIpldValue`.
- The C ABI version is 6 rather than 5. Status codes for unknown multicodecs
  are distinct, and operation 31 represents key-binding validation without an
  algorithm; operation 30 rejects an empty algorithm.
- TypeScript no longer exports `requireReallyMeCodecWasmProvider`. Provider
  installation checks module identity and initialization, and a WebAssembly
  runtime trap disables further calls through that instance. `dagCborVerifyCid`
  now returns a boolean; `dagCborVerifyCidDetails` returns CID diagnostics.
- Kotlin `loadLibrary` requires an explicit local-development opt-in.
- DAG-CBOR nesting is limited to 64 levels rather than 128. Base58btc input
  and decoded-byte limits are 5,600 and 4,098 bytes. Base64 and base64url
  input is limited to 2 MiB.
- PEM option and label-count limits are enforced. Empty label allowlists select
  public-key PEM only, and private-key labels require explicit selection.
- Binary protobuf requests reject duplicate singular fields and multiple
  operation oneof members. ProtoJSON rejects unquoted exponent-form integers.

The JCS implementation remains a strict RFC 8785 subset: integer-valued
binary64 numbers outside the interoperable I-JSON integer range are rejected.
This includes the RFC's `1e20`, `1e21`, and `1e23` examples.

## 0.2.2

- Hardened C ABI pointer and output buffer aliasing checks. This version has
  the `v0.2.2` Git tag.
