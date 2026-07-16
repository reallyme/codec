<!--
SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved

SPDX-License-Identifier: Apache-2.0
-->

# ReallyMeCodec Java/Kotlin

`me.really:codec` is the Java/Kotlin JVM codec package for ReallyMe. It exposes
the same codec surface as the Rust `reallyme-codec` crate and delegates
operations to the Rust native provider instead of reimplementing encoders, PEM
parsing, or DAG-CBOR on the JVM.

## Install

```kotlin
dependencies {
    implementation("me.really:codec:0.1.21")
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

For local host-only development, `./gradlew test` builds the Rust FFI library
and stages it under Gradle's generated resources before running the tests.
`./gradlew publishToMavenLocal` does the same and publishes a host-only local
artifact for development.

Release and preflight builds pass
`-Preallyme.codec.requireFullNativeResources=true`; in that mode Maven
publication fails unless all supported JVM platform libraries are present.
