# ReallyMeCodec Java/Kotlin

The `me.really:codec` JVM package provides codecs for Java and Kotlin.

## Install

```kotlin
dependencies {
    implementation("me.really:codec:0.2.2")
}
```

## Provider

The Maven artifact ships Rust JNI libraries as platform resources. The facade
extracts and loads the matching library on first use:

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
resource limits before returning SDK owners.

PEM input, output, and decoded DER use `ByteArray` rather than `String` so
callers can overwrite private-key material promptly after use.

Local development builds can still load an explicit Rust ABI library when
debugging provider loading:

```kotlin
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

Copyright © 2026 ReallyMe Technologies Limited. All rights reserved
