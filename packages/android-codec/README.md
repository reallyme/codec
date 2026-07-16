<!--
SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved

SPDX-License-Identifier: Apache-2.0
-->

# ReallyMeCodec Android

`me.really:codec-android` is the Android AAR for ReallyMe Codec. It packages
the same Kotlin facade as `me.really:codec` with Rust JNI libraries under
`jniLibs/<abi>/`, so Android applications do not need to build or load a native
library manually.

## Install

```kotlin
dependencies {
    implementation("me.really:codec-android:0.1.21")
}
```

## Release Build

```sh
scripts/build_android_native_resources.sh build/android-jniLibs
packages/kotlin-codec/gradlew -p packages/android-codec bundleReleaseAar \
  -Preallyme.codec.androidJniLibsDir="$PWD/build/android-jniLibs" \
  -Preallyme.codec.requireAndroidJniLibs=true
```
