# ReallyMeCodec Android

The `me.really:codec-android` AAR includes the Kotlin API and native JNI libraries.

## Install

```kotlin
dependencies {
    implementation("me.really:codec-android:0.2.2")
}
```

## Release Build

```sh
scripts/build_android_native_resources.sh build/android-jniLibs
packages/kotlin/gradlew -p packages/kotlin-android bundleReleaseAar \
  -Preallyme.codec.androidJniLibsDir="$PWD/build/android-jniLibs" \
  -Preallyme.codec.requireAndroidJniLibs=true
```

## Typed CBOR

The Android artifact exposes the same typed deterministic-CBOR and DAG-CBOR
builders as the JVM package through `ReallyMeDeterministicCbor` and
`ReallyMeDagCbor`. `ByteArray` is the canonical mutable-byte boundary; callers
should clear sensitive arrays after use. DAG-CBOR builders expose text-key maps
only, while deterministic CBOR additionally supports integer-key maps.

Encoding canonicalizes map ordering. Decoding rejects duplicate keys, non-canonical input, unsupported types and exceeded resource limits.

Copyright © 2026 ReallyMe Technologies Limited. All rights reserved
