# ReallyMe Codec for Swift

Swift codecs for macOS 13 and iOS 16 or newer, using the packaged Rust library.
Building from source requires Swift 6.3.

## Install

```swift
.package(url: "https://github.com/reallyme/codec", from: "0.4.5")
```

Add `.product(name: "ReallyMeCodec", package: "codec")` to your target.
Use a release tag with its matching binary artifact. An untagged checkout can
report `providerUnavailable` when no binary has been bound to the manifest.

```swift
import ReallyMeCodec

let codec = try ReallyMeCodec()
let encoded = try codec.base64urlEncode([1, 2, 3])
let decoded = try codec.base64urlDecode(encoded)
```

Typed CBOR values use `ReallyMeDeterministicCbor` and `ReallyMeDagCbor` builders:

```swift
import Foundation

let value = ReallyMeDeterministicCbor.mapText([
    ("b", ReallyMeDeterministicCbor.unsigned(2)),
    ("a", ReallyMeDeterministicCbor.bytes(Data([0, 1, 2]))),
])
let encodedCbor = try codec.deterministicCborEncodeData(value)
```

See [format limits](../../README.md#formats-and-limits) for the supported CBOR,
CID and JCS profiles. Clear sensitive `[UInt8]` buffers after use; `Data` and
managed copies do not provide a complete erasure guarantee.

Shared-operation errors use `ReallyMeCodecError.operationFailure` with generated
reason and origin values. Use `error.category` when matching broad error
categories. Platform failures can have no protobuf reason or origin.

`processOperation` and `processOperationJson` accept serialized requests and
return binary protobuf responses. Generated Swift messages remain internal to
the SDK. See the [protobuf API](../../docs/protobuf.md).

## Test

From the repository root:

```sh
cargo build --locked -p reallyme-codec-ffi
REALLYME_CODEC_SWIFTPM_RUNTIME_FFI=1 \
REALLYME_CODEC_FFI_LIBRARY_PATH="$PWD/target/debug/libreallyme_codec_ffi.dylib" \
swift test
```

For external native libraries, use
`ReallyMeCodecRustCAbiLibrary(path:expectedSHA256:)` with a pinned digest.
The `path:` initializer does not verify a digest. Keep library files in storage
that untrusted users cannot modify.

## License

MIT or Apache-2.0, at your option. See [MIT](../../LICENSE-MIT) and
[Apache 2.0](../../LICENSE-APACHE).
