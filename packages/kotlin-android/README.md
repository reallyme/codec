<!--
SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
-->

# ReallyMeCodec Android

`me.really:codec-android` is the Android AAR for ReallyMe Codec. It packages
the same Kotlin facade as `me.really:codec` with Rust JNI libraries under
`jniLibs/<abi>/`, so Android applications do not need to build or load a native
library manually.

## Install

```kotlin
dependencies {
    implementation("me.really:codec-android:0.2.3")
}
```

## Release Build

Publishing uses **Kotlin Android Package Preflight** followed by **Kotlin Android
Package Release**, which verifies the required checks for the selected source
commit. To build the AAR locally from the repository root:

```sh
scripts/build_android_native_resources.sh build/android-jniLibs
packages/kotlin/gradlew -p packages/kotlin-android bundleReleaseAar \
  -Preallyme.codec.androidJniLibsDir="$PWD/build/android-jniLibs" \
  -Preallyme.codec.requireAndroidJniLibs=true
```

Native `.so` files are never sourced from the Git worktree. Every AAR build
must consume freshly built JNI libraries through
`reallyme.codec.androidJniLibsDir`; release workflows build them from the exact
reviewed source SHA before packaging. The generated native manifest is package
inventory, while the SHA-bound workflow and artifact transfer are the
provenance anchor.

## Typed CBOR

The Android artifact exposes the same typed deterministic-CBOR and DAG-CBOR
builders as the JVM package through `ReallyMeDeterministicCbor` and
`ReallyMeDagCbor`. `ByteArray` is the canonical mutable-byte boundary; callers
should clear sensitive arrays after use. DAG-CBOR builders expose text-key maps
only, while deterministic CBOR additionally supports integer-key maps.

Encoding canonicalizes map ordering. Decoding rejects duplicate semantic keys,
non-canonical encodings, unsupported CBOR types, and values beyond the shared
resource limits. Recursive values are carried through generated protobuf
messages with bounded message depth and unknown-field rejection before Rust
performs the canonical operation.

## Test

With the Android SDK, the configured NDK, and a running emulator or connected
test device available, run the R8 consumer gate from the repository root:

```sh
scripts/test_android_consumer_r8_runtime.sh
```

The gate builds fresh JNI libraries and a minified release consumer, installs
the APK, and checks codec operations on the device. The **Android Runtime Gate**
workflow runs this check in CI.

## License

Dual-licensed under the MIT License or Apache License, Version 2.0, at your
option. See [LICENSE](../../LICENSE) for both license texts.
