# Changelog

## 0.4.5 (unreleased)

- Allow explicit general-CBOR input, output, and aggregate byte-string budgets
  up to 2 MiB. Defaults remain 1 MiB; other resource limits are unchanged.
- Apply the selected input budget to received fragments in both encoding adapters.

## 0.4.4

- Require Rust 1.100 and ReallyMe Buffa 0.1.3.
- Bound memory use when decoding nested or repeated DER SET OF values.
- Clear escaped JCS keys and values after successful and failed parsing.
- Detect in-place changes to external native libraries loaded by Swift.
- Add PEM bundle decoding with strict newline-delimited armor and an explicit
  compatibility profile.
- Reject invalid EC point tags and oversized RSA payloads when stripping a
  multicodec prefix, consistently with multikey decoding.
- Add bounded JSON document decoding and configurable Base64 input/output
  budgets. Existing Base64 defaults remain unchanged.
- Permit explicit general-CBOR entry limits up to 65,536 while retaining the
  4,096-entry default and the existing byte, depth, work and node budgets.
- Expose protobuf error reason and origin in SDK errors. Swift operation
  failures use `operationFailure`; match `error.category` for broad categories.
- Return shared general-CBOR collection options once per response; individual
  frame options, when present, take precedence.

## 0.4.2

- Fix Swift XCFramework headers and module maps so Swift Build can load Codec
  alongside other binary dependencies.

## 0.4.1

- Add general-CBOR options for encoded-scalar map-key equality and binary16-only
  NaNs, usable independently of each other.
- Update generated Rust types to ReallyMe Buffa 0.1.2 and SwiftProtobuf to 1.41.0.

## 0.4.0

- Use ReallyMe Buffa 0.1.1 generated messages. Owned sensitive fields clear on
  drop. Rust callers must convert owned byte/string fields with `.into()` and
  use borrowing or `mem::take` when moving fields from a message with `Drop`.
- Add definite TLV identifier, length, header, prefix and exact framing, with
  explicit source limits up to 16 MiB.
- Reject precompiled TypeScript `WebAssembly.Module` inputs; initialize with
  the package's Wasm file instead.

## 0.3.4

- Refuse Swift native-library loads redirected by `DYLD_LIBRARY_PATH` or
  `DYLD_IMAGE_SUFFIX`, including digest-verified loads.
- Report oversized declared DAG-CBOR lengths consistently on native and Wasm
  hosts, and distinguish disallowed simple values from non-minimal encodings.
- Add borrowed general-CBOR framing, validation, cursors, float policies and
  encoding that preserves received items and caller map order.
- Reduce validation work for large maps. Complex keys can still exhaust the
  configured work budget.
- Reject forged TypeScript byte views and nonzero ProtoJSON Base64 pad bits.
- Verify bundled Wasm bytes before initialization and accept the largest
  default PEM encoding with either LF or CRLF line endings.

## 0.3.3

- Add opt-in schema-directed DER readers and unsigned-INTEGER-pair decoding.
  Reject noncanonical lengths, integer padding, bit-string padding, OIDs,
  SET OF ordering and trailing data.

## 0.3.1

- Recover the TypeScript facade after unexpected Wasm exceptions.
- Clear JCS object values on drop and bound nested protobuf decoding.
- Align unsupported multikey and invalid DAG-CBOR errors across SDKs.

## 0.3.0

- Add input, nesting and allocation limits across parsers. Require explicit PEM
  allowlists for private-key labels.
- `encode_protobuf` and `write_lower_hex` now return typed results. Multicodec
  prefix stripping and DAG-CBOR CID verification return typed outcomes.
- `CborValue` implements `Drop` and no longer implements `Clone`; by-value
  destructuring must be replaced with borrowing or explicit ownership transfer.
  `ParsedMultikey` fields are private and have read-only accessors.
- Remove the `codec_spec_proto` crate and Swift `ReallyMeCodecProto` product.
  Use Swift's `ReallyMeCodec` product. Exhaustive Swift error switches must
  handle `providerUnavailable`, `nonCanonical`, `unsupportedCodec` and
  `unsupportedIpldValue`.
- Advance the C ABI from version 5 to 6. Operation 31 validates binding without
  an algorithm; operation 30 rejects an empty algorithm.
- Remove TypeScript's `requireReallyMeCodecWasmProvider` export.
  `dagCborVerifyCid` returns a boolean; use `dagCborVerifyCidDetails` for details.
- Require an explicit development opt-in for Kotlin `loadLibrary`.
- Limit DAG-CBOR depth to 64, Base58btc input/output to 5,600/4,098 bytes, and
  Base64/Base64url input to 2 MiB.
- Reject duplicate singular protobuf fields, multiple operation selectors and
  unquoted exponent-form ProtoJSON integers.

## 0.2.2

- Tighten C ABI pointer and output-buffer aliasing validation.
