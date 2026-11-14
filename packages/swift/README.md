# ReallyMeCodec Swift

Swift codecs for Apple platforms.

## Install

```swift
.package(
    url: "https://github.com/reallyme/codec",
    from: "0.4.1"
)
```

```swift
.product(name: "ReallyMeCodec", package: "codec")
```

Generated Swift protobuf messages are internal. Use `processOperation` or `processOperationJson` for serialized requests, or generate matching wire types in your application.

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

The Swift API includes base encodings, multiformats, deterministic CBOR, DAG-CBOR, CID, PEM and JCS.

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

Encoding canonicalizes map ordering. Decoding rejects duplicate keys, non-canonical input, unsupported types and exceeded limits. DAG-CBOR accepts text map keys only. Positive DAG-CBOR integers must not exceed `Int64.max`. CID verification rejects types outside the supported DAG-CBOR profile.

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

## License

Dual-licensed under the MIT License or Apache License, Version 2.0, at your
option. See [MIT](../../LICENSE-MIT) and [Apache 2.0](../../LICENSE-APACHE).

An untagged checkout without a matching binary artifact reports
`providerUnavailable` from `ReallyMeCodec()`. Local tests can pass an explicit
Rust C ABI library path through `ReallyMeCodecRustCAbiLibrary(path:)`, which
does not verify the file. Use `ReallyMeCodecRustCAbiLibrary(path:expectedSHA256:)`
to refuse any library whose SHA-256 does not match a pinned digest. Both
initializers fail with `dynamicLibraryLoadFailed` when `DYLD_LIBRARY_PATH` or
`DYLD_IMAGE_SUFFIX` would make the dynamic loader map a different file, or when
the loaded image is not the file at the given path.
