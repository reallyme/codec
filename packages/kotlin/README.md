# ReallyMeCodec Java/Kotlin

The `me.really:codec` JVM package provides codecs for Java and Kotlin.

## Install

```kotlin
dependencies {
    implementation("me.really:codec:0.4.2")
}
```

## Provider

The Maven artifact ships Rust JNI libraries as platform resources. The facade
extracts and loads the matching library on first use:

The Linux resources are built for glibc, with a verified minimum symbol
version of `GLIBC_2.34` in both `linux-x86_64` and `linux-aarch64` artifacts.
They do not support musl-based systems such as Alpine Linux. On an unsupported
system, provider loading fails with `ProviderFailure`; there is no fallback
codec implementation. Consumers targeting older glibc or musl need a native
build for that platform before using this package.

```kotlin
import me.really.codec.ReallyMeCodec

val encoded = ReallyMeCodec.base64urlEncode(byteArrayOf(1, 2, 3))
val decoded = ReallyMeCodec.base64urlDecode(encoded)
```

Java callers use the same Maven artifact through static facade methods:

```java
import me.really.codec.ReallyMeCodec;

String encoded = ReallyMeCodec.base64urlEncode(new byte[] {1, 2, 3});
byte[] decoded = ReallyMeCodec.base64urlDecode(encoded);
```

Structured results, including multicodec metadata and decoded PEM, are returned as typed SDK values.

Use typed builders for deterministic CBOR and DAG-CBOR:

```kotlin
import me.really.codec.ReallyMeDeterministicCbor
import me.really.codec.ReallyMeDagCbor

val value = ReallyMeDeterministicCbor.mapText(
    listOf(
        "b" to ReallyMeDeterministicCbor.unsignedLong(2),
        "a" to ReallyMeDeterministicCbor.bytes(byteArrayOf(0, 1, 2)),
    ),
)
val encodedCbor = ReallyMeCodec.deterministicCborEncode(value)
val dagCbor = ReallyMeCodec.dagCborEncode(
    ReallyMeDagCbor.mapText(listOf("payload" to value)),
)
```

Encoding canonicalizes map ordering. Decoding rejects duplicate semantic keys,
non-canonical input, unsupported CBOR types, and values beyond the documented
resource limits before returning SDK owners. Although the builders share a
value type, DAG-CBOR rejects integer map keys and positive integers above
`Long.MAX_VALUE`.
CID verification rejects tags and floating-point encodings outside this
SDK's closed DAG-CBOR model as `InvalidInput`.

PEM input, output, and decoded DER use `ByteArray` rather than `String` so
callers can overwrite private-key material promptly after use.

Local development builds can still load an explicit Rust ABI library when
debugging provider loading:

```kotlin
import me.really.codec.ReallyMeCodecRustNativeProvider

// Explicitly enable external native paths for local development only.
System.setProperty("reallyme.codec.allowUnverifiedNative", "true")
ReallyMeCodecRustNativeProvider.loadLibrary("/path/to/libreallyme_codec_ffi.dylib")
```

## Test

```sh
cd packages/kotlin
./gradlew test
```

To test the Maven package as shipped, stage the Rust native library as a bundled
resource from the repository root and point Gradle at that resource tree:

```sh
scripts/build_kotlin_native_resource.sh build/kotlin-native-resources
packages/kotlin/gradlew -p packages/kotlin test \
  -Preallyme.codec.nativeResourcesDir="$PWD/build/kotlin-native-resources"
```

Host tests build and stage the native library automatically. `publishToMavenLocal` creates a host-only development artifact.

## License

Dual-licensed under the MIT License or Apache License, Version 2.0, at your
option. See [MIT](../../LICENSE-MIT) and [Apache 2.0](../../LICENSE-APACHE).
