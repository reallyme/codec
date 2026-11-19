# ReallyMe Codec for Android

The `me.really:codec-android` AAR includes the Kotlin API and its native JNI
libraries. Applications do not need to build or load a library manually.

```kotlin
dependencies {
    implementation("me.really:codec-android:0.4.4")
}
```

Use the [Kotlin API](../kotlin/README.md) for examples, error handling and CBOR
builders. Clear sensitive `ByteArray` values after use.

## Build and test

With the Android SDK and configured NDK installed, build the AAR from the
repository root:

```sh
scripts/build_android_native_resources.sh build/android-jniLibs
packages/kotlin/gradlew -p packages/kotlin-android bundleReleaseAar \
  -Preallyme.codec.androidJniLibsDir="$PWD/build/android-jniLibs" \
  -Preallyme.codec.requireAndroidJniLibs=true
```

With a running emulator or connected device:

```sh
scripts/test_android_consumer_r8_runtime.sh
```

## License

MIT or Apache-2.0, at your option. See [MIT](../../LICENSE-MIT) and
[Apache 2.0](../../LICENSE-APACHE).
