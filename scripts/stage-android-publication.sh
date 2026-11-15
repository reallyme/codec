#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly REPO_ROOT
if [[ $# != 1 || "$1" != /* ]]; then
    printf 'usage: %s /absolute/maven/repository\n' "$0" >&2
    exit 2
fi
readonly MAVEN_REPOSITORY="$1"
readonly ANDROID_NDK_HOME_VALUE="${ANDROID_NDK_HOME:?Android NDK path is required}"
readonly JNILIBS_DIR="$REPO_ROOT/build/android-jniLibs"
readonly NATIVE_ASSETS_DIR="$REPO_ROOT/build/android-native-assets"

fail() {
    printf '%s\n' "$1" >&2
    exit 1
}

ANDROID_NDK_HOME="$ANDROID_NDK_HOME_VALUE" "$REPO_ROOT/scripts/build_android_native_resources.sh" "$JNILIBS_DIR"

case "$(uname -s)" in
    Darwin) ndk_host_tag="darwin-x86_64" ;;
    Linux) ndk_host_tag="linux-x86_64" ;;
    *) fail "unsupported host for Android ELF verification" ;;
esac
readonly LLVM_READELF="$ANDROID_NDK_HOME_VALUE/toolchains/llvm/prebuilt/$ndk_host_tag/bin/llvm-readelf"
[[ -x "$LLVM_READELF" ]] || fail "Android NDK llvm-readelf is required"
for abi in arm64-v8a armeabi-v7a x86_64 x86; do
    library="$JNILIBS_DIR/$abi/libreallyme_codec_ffi.so"
    [[ -f "$library" ]] || fail "Android native library is missing for $abi"
    program_headers="$("$LLVM_READELF" -W -l "$library")"
    load_segments=0
    while IFS= read -r alignment; do
        ((load_segments += 1))
        ((alignment >= 0x4000)) || fail "Android $abi LOAD segment is below 16 KB page alignment"
    done < <(awk '$1 == "LOAD" { print $NF }' <<<"$program_headers")
    ((load_segments > 0)) || fail "Android $abi library has no LOAD segments"
done

node "$REPO_ROOT/scripts/write_native_manifest.mjs" "$JNILIBS_DIR" "$NATIVE_ASSETS_DIR/reallyme-codec/native-manifest.json"

ANDROID_NDK_HOME="$ANDROID_NDK_HOME_VALUE" "$REPO_ROOT/packages/kotlin/gradlew" \
    -p "$REPO_ROOT/packages/kotlin-android" \
    verifyReleaseAarContainsJniLibs publishAllPublicationsToLocalReleaseRepository \
    "-Preallyme.maven.localReleaseRepositoryDir=$MAVEN_REPOSITORY" \
    "-Preallyme.codec.androidJniLibsDir=$JNILIBS_DIR" \
    "-Preallyme.codec.androidNativeAssetsDir=$NATIVE_ASSETS_DIR" \
    -Preallyme.codec.requireAndroidJniLibs=true
