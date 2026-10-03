# ReallyMeCodec Android

The `me.really:codec-android` AAR includes the Kotlin API and native JNI libraries.

## Install

```kotlin
dependencies {
    implementation("me.really:codec-android:0.3.1")
}
```

## Release Build

To build the AAR locally from the repository root:

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

## Test

With the Android SDK, the configured NDK, and a running emulator or connected
test device available, run the R8 consumer gate from the repository root:

```sh
scripts/test_android_consumer_r8_runtime.sh
```

The runtime test requires the Android SDK, configured NDK and a running emulator or connected device.

## License

Dual-licensed under the MIT License or Apache License, Version 2.0, at your
option. See [MIT](../../LICENSE-MIT) and [Apache 2.0](../../LICENSE-APACHE).
