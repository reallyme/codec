# ReallyMeCodec Android

The `me.really:codec-android` AAR includes the Kotlin API and native JNI libraries.

## Install

```kotlin
dependencies {
    implementation("me.really:codec-android:0.1.22")
}
```

## Release Build

```sh
scripts/build_android_native_resources.sh build/android-jniLibs
packages/kotlin/gradlew -p packages/kotlin-android bundleReleaseAar \
  -Preallyme.codec.androidJniLibsDir="$PWD/build/android-jniLibs" \
  -Preallyme.codec.requireAndroidJniLibs=true
```

Copyright © 2026 ReallyMe Technologies Limited. All rights reserved
