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

Native `.so` files are never sourced from the Git worktree. Every AAR build
must consume freshly built JNI libraries through
`reallyme.codec.androidJniLibsDir`; release workflows build them from the exact
audited source SHA before packaging. The generated native manifest is package
inventory, while the SHA-bound workflow and artifact transfer are the
provenance anchor.
