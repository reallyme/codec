# ReallyMeCodec Swift

`ReallyMeCodec` is the Swift SDK facade for
[ReallyMe Codec](https://github.com/reallyme/codec), for Apple platforms. It
does not reimplement codecs in Swift; all operations delegate to the Rust codec
C ABI.

The manifest sits at the repository root (`Package.swift`) so SwiftPM can add it
by Git URL; the source lives under `packages/swift` with the other language SDKs.

## Install

```swift
.package(
    url: "https://github.com/reallyme/codec",
    from: "0.3.1"
)
```

```swift
.product(name: "ReallyMeCodec", package: "codec")
```

Generated protobuf messages are internal to the Swift SDK. Use
`processOperation` or `processOperationJson` for the binary or JSON wire
contract; construct or parse protobuf messages in your own module when your
application needs typed wire access. This keeps the SDK's sensitive generated
messages out of public generic formatting paths.

## Quick Start

A tagged SwiftPM release includes the Rust FFI `.xcframework` binary target
only when its artifact version matches the Swift source version. Then
applications can construct the codec directly:

```swift
import ReallyMeCodec

let codec = try ReallyMeCodec()

let encoded = try codec.base64urlEncode([1, 2, 3])
let decoded = try codec.base64urlDecode(encoded)
```

`ReallyMeCodec` exposes PEM armor, lowercase hex, base64/base64url, multibase,
multicodec, multikey, deterministic CBOR, DAG-CBOR, CID helpers, and JCS. The package does not
silently fall back to local Swift implementations.

Deterministic generic CBOR and DAG-CBOR use typed value builders:

```swift
import Foundation

let value = ReallyMeDeterministicCbor.mapText([
    ("b", ReallyMeDeterministicCbor.unsigned(2)),
    ("a", ReallyMeDeterministicCbor.bytes(Data([0, 1, 2]))),
])
let encodedCbor = try codec.deterministicCborEncodeData(value)
let decodedCbor = try codec.deterministicCborDecode(encodedCbor)
let dagCbor = try codec.dagCborEncodeData(ReallyMeDagCbor.mapText([
    ("payload", decodedCbor),
]))
```

The Swift model is deliberately generated-transport-shaped: recursive CBOR
values are serialized into the shared protobuf operation request, decoded with
a raised SwiftProtobuf message-depth limit derived from the documented
semantic nesting cap, validated for unknown fields and resource budgets, and
then handed to Rust for canonical bytes. Encoding canonicalizes map ordering;
decoding rejects duplicate semantic keys, non-canonical input, unsupported
CBOR types, and over-limit values.
The deterministic-CBOR builder supports integer and text map keys; the
`ReallyMeDagCbor` builder intentionally exposes text-key maps only because
DAG-CBOR has the stricter key profile. Both routes use the same generated
protobuf operation contract and bounded SwiftProtobuf depth/resource checks.
The shared value type does not make the profiles interchangeable: DAG-CBOR
rejects integer map keys and integers above `Int64.max`.
CID verification rejects tags and floating-point encodings outside this
SDK's closed DAG-CBOR model as `invalidInput`.

PEM input, output, and decoded DER use `[UInt8]` rather than `String` so
callers can clear private-key material promptly with their own memory policy.
Swift-only `Data` overloads are available for common byte boundaries, but
`[UInt8]` remains the canonical cross-language API shape.

Local development builds can still pass an explicit Rust ABI library:

```sh
cargo build --locked -p reallyme-codec-ffi
```

```swift
let codecAbi = try ReallyMeCodecRustCAbiLibrary(path: "/path/to/libreallyme_codec_ffi.dylib")
let codec = try ReallyMeCodec(rustCAbiLibrary: codecAbi)
```

## Test

```sh
cargo build --locked -p reallyme-codec-ffi
REALLYME_CODEC_SWIFTPM_RUNTIME_FFI=1 \
REALLYME_CODEC_FFI_LIBRARY_PATH="$PWD/target/debug/libreallyme_codec_ffi.dylib" \
swift test
```

## Release Binding

The Swift preflight builds the release XCFramework and uploads its archive and
SwiftPM checksum. Download those files into `build/swift` before publishing,
then bind the checksum and matching version in the root manifest:

```sh
node scripts/prepare_swift_binary_manifest.mjs 0.3.1 "$(cat build/swift/ReallyMeCodecFFI.xcframework.checksum)"
```

Review and commit `Package.swift` on `main`, then rerun the preflights for that
new commit before starting any release workflow. The Swift release verifies
that the committed manifest matches the exact preflight artifact; it does not
write a binding commit during publication.

## License

Dual-licensed under the MIT License or Apache License, Version 2.0, at your
option. See [MIT](../../LICENSE-MIT) and [Apache 2.0](../../LICENSE-APACHE).

An untagged checkout without a matching binary artifact reports
`providerUnavailable` from `ReallyMeCodec()`. Local tests can pass an explicit,
verified Rust C ABI library path through `ReallyMeCodecRustCAbiLibrary`.
