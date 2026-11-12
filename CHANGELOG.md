# Changelog

All packages in the ReallyMe Codec release line share one version. Security
changes are called out when they affect accepted input or an integration
boundary.

## 0.3.4 (unreleased)

- Derive C sanitizer harness version expectations from Cargo package metadata,
  rejecting missing, invalid, and mismatched release versions without copying
  release numbers into the harness.
- Refresh all four Cargo lockfiles, including Proptest 1.12.0 and the itoa,
  const-oid, and Syn patch updates.
- Update SwiftProtobuf to 1.40.0, Gradle to 9.9.1, and the Bouncy Castle tooling
  constraint to 1.87 with publisher-verified dependency hashes.
- Update Rust to 1.100.0, Buf to 1.75.0, cargo-audit to 0.23.0, nextest to
  0.9.151, wasm-pack to 0.15.1, and the OSV scanner action to 2.7.1.

## 0.3.3

- Add opt-in, bounded, schema-directed strict DER readers and an exact
  SEQUENCE-of-two-unsigned-INTEGER decoder for Rust consumers. Curve widths,
  scalar ranges, and signature normalization remain in the cryptographic layer.
- Reject noncanonical DER lengths, INTEGER sign padding, BIT STRING padding,
  OID subidentifiers, SET OF ordering, and trailing data with typed errors.
- Expand parser property tests, fuzzing, Miri coverage, and regression checks
  against Cargo feature unification.
- Harden CI pin verification and tooling checks, including bounded subprocess
  deadlines that tolerate loaded runners.

## 0.3.2 (2026-11-08)

- Change project ownership and copyright references to ReallyMe Technologies
  Limited across source, generated artifacts, package metadata, and release
  policy checks.
- Refresh Rust, TypeScript, Swift, Kotlin, Android, WASM, build-tool, and CI
  dependencies together with their committed lockfiles and verification
  metadata, including the pinned release-readiness policy core at v0.7.0.
- Keep every public package and SDK lane on the shared 0.3.2 release version.

## 0.3.1 (2026-10-03)

- Recover the TypeScript facade after unexpected WASM exceptions by wiping the
  trapped instance's linear memory and creating a fresh instance. Raw WASM
  string exports reject null inputs without exhausting the bindgen stack.
- Wipe JCS object-member values on drop and tighten protobuf decoder memory
  accounting for nested arrays.
- Classify malformed DAG-CBOR tags and floating-point encodings consistently
  across decoding and CID verification. Unknown multikey codec names now return
  the unsupported-codec class in native SDKs, matching TypeScript.
- Validate the Cargo lockfile during WASM builds and package preflight, and
  expand boundary and cross-language regression coverage.

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
- Update dependency locks and release-readiness policy checks.
- Align scalar error classes across Rust, Swift, Kotlin, and TypeScript; give
  canonical CBOR failures distinct protobuf reasons.
- Keep generated Swift wire messages internal to the SDK, preventing generic
  SwiftProtobuf text formatting from exposing their sensitive fields.
- Generate Rust and SDK codec limits from one reviewed table.
- Move scalar adapter policy out of the public Rust facade and wrap CID and
  multihash types behind codec-owned APIs.

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

## 0.2.3

- Published the current shared Rust and SDK codec surface.

## 0.2.2

- Hardened C ABI pointer and output buffer aliasing checks. This version has
  the `v0.2.2` Git tag.
