#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly REPO_ROOT
if [[ $# != 3 || "$1" != /* || ! "$2" =~ ^[a-zA-Z0-9_-]+$ || "$3" != /* ]]; then
    printf 'usage: %s /absolute/output.apk avd-name /absolute/maven/repository\n' "$0" >&2
    exit 2
fi
readonly APK_PATH="$1"
readonly AVD_NAME="$2"
readonly MAVEN_REPOSITORY="$3"
readonly ANDROID_HOME_VALUE="${ANDROID_HOME:?Android SDK path is required}"
readonly SDKMANAGER="$ANDROID_HOME_VALUE/cmdline-tools/latest/bin/sdkmanager"
readonly AVDMANAGER="$ANDROID_HOME_VALUE/cmdline-tools/latest/bin/avdmanager"
readonly ANDROID_R8_PLATFORM="platforms;android-37.0"
readonly ANDROID_R8_SYSTEM_IMAGE="system-images;android-37.1;google_apis_ps16k;x86_64"
export ANDROID_AVD_HOME="${ANDROID_AVD_HOME:?isolated Android AVD home is required}"
mkdir -p "$ANDROID_AVD_HOME"

{ yes 2>/dev/null || true; } | "$SDKMANAGER" "emulator" "$ANDROID_R8_PLATFORM" "$ANDROID_R8_SYSTEM_IMAGE"
# Use the same hardware profile as the Android R8 runtime workflow.
printf 'no\n' | "$AVDMANAGER" create avd --force -n "$AVD_NAME" -k "$ANDROID_R8_SYSTEM_IMAGE" --device pixel

# The published AAR is a new dependency on every CI candidate. Bind its
# sealed bytes into an isolated copy of the consumer's strict verification
# metadata so the checked-in third-party pins and source tree stay intact.
source_sha="$(git -C "$REPO_ROOT" rev-parse --verify HEAD)"
test "$source_sha" = "${GITHUB_SHA:?candidate SHA is required}"
node "$REPO_ROOT/.release-readiness/scripts/android-runtime.mjs" verify-publication "$MAVEN_REPOSITORY" "$source_sha"
consumer_build="$(mktemp -d)"
trap 'rm -rf -- "$consumer_build"' EXIT
git -C "$REPO_ROOT" archive --format=tar "$source_sha" -- packages/kotlin-android |
    tar -xf - -C "$consumer_build"
consumer_project="$consumer_build/packages/kotlin-android"
node "$REPO_ROOT/scripts/pin_android_consumer_publication.mjs" \
    "$MAVEN_REPOSITORY" "$consumer_project/gradle/verification-metadata.xml"
"$REPO_ROOT/packages/kotlin/gradlew" -p "$consumer_project" \
    --dependency-verification strict :consumer-r8-runtime:assembleRelease \
    "-Preallyme.codec.androidPublicationRepository=$MAVEN_REPOSITORY"
cp -- "$consumer_project/consumer-r8-runtime/build/outputs/apk/release/consumer-r8-runtime-release.apk" "$APK_PATH"
