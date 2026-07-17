# ReallyMeCodec Java/Kotlin

The `me.really:codec` JVM package provides codecs for Java and Kotlin.

## Install

```kotlin
dependencies {
    implementation("me.really:codec:0.1.22")
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

`ReallyMeCodec` covers base64, base64url, lowercase hex, base58btc,
multibase, multicodec, multikey, DAG-CBOR, JCS, and PEM armor. Structured
results such as multicodec metadata and PEM decode output are returned as the
compact JSON emitted by the Rust codec bridge.

PEM input, output, and decoded JSON use `ByteArray` rather than `String` so
callers can overwrite private-key armor promptly after use.

Local development builds can still load an explicit Rust ABI library when
debugging provider loading:

```kotlin
ReallyMeCodecRustNativeProvider.loadLibrary("/path/to/libreallyme_codec_ffi.dylib")
```

## Test

```sh
cd packages/kotlin-codec
./gradlew test
```

To test the Maven package as shipped, stage the Rust native library as a bundled
resource from the repository root and point Gradle at that resource tree:

```sh
scripts/build_kotlin_native_resource.sh build/kotlin-native-resources
packages/kotlin-codec/gradlew -p packages/kotlin-codec test \
  -Preallyme.codec.nativeResourcesDir="$PWD/build/kotlin-native-resources"
```

Host tests build and stage the native library automatically. `publishToMavenLocal` creates a host-only development artifact.

Copyright © 2026 ReallyMe Technologies Limited. All rights reserved
