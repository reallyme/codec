#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

if [[ $# != 1 || ! "$1" =~ ^[a-zA-Z0-9_-]+$ ]]; then
    printf 'usage: %s avd-name\n' "$0" >&2
    exit 2
fi
readonly AVD_NAME="$1"
readonly ANDROID_HOME_VALUE="${ANDROID_HOME:?Android SDK path is required}"
export ANDROID_AVD_HOME="${ANDROID_AVD_HOME:?isolated Android AVD home is required}"
if [[ "$ANDROID_HOME_VALUE" != /* || "$ANDROID_AVD_HOME" != /* ]]; then
    printf 'Android SDK and AVD directories must be absolute\n' >&2
    exit 2
fi
readonly COMMAND_LINE_TOOLS_VERSION="23.0"
readonly SDKMANAGER="$ANDROID_HOME_VALUE/cmdline-tools/$COMMAND_LINE_TOOLS_VERSION/bin/sdkmanager"
readonly AVDMANAGER="$ANDROID_HOME_VALUE/cmdline-tools/$COMMAND_LINE_TOOLS_VERSION/bin/avdmanager"
readonly ANDROID_R8_PLATFORM="platforms;android-37.0"
readonly ANDROID_R8_IMAGE_TARGET="android-37.1"
readonly ANDROID_R8_SYSTEM_IMAGE="system-images;$ANDROID_R8_IMAGE_TARGET;google_apis_ps16k;x86_64"

# Older hosted tools silently write android-0 for minor-version images, which
# disables emulator graphics features and can crash SurfaceFlinger. Install
# beside the hosted tools and select the reviewed version by its explicit path.
if [[ ! -x "$SDKMANAGER" || ! -x "$AVDMANAGER" ]]; then
    { yes 2>/dev/null || true; } |
        "$ANDROID_HOME_VALUE/cmdline-tools/latest/bin/sdkmanager" --install "cmdline-tools;$COMMAND_LINE_TOOLS_VERSION"
fi
if [[ ! -x "$SDKMANAGER" || ! -x "$AVDMANAGER" ]]; then
    printf 'Pinned Android command-line tools are unavailable\n' >&2
    exit 1
fi
if ! tool_version="$("$SDKMANAGER" --version)" || [[ "$tool_version" != "$COMMAND_LINE_TOOLS_VERSION" ]]; then
    printf 'Pinned Android command-line tools have an unexpected version\n' >&2
    exit 1
fi

mkdir -p "$ANDROID_AVD_HOME"
{ yes 2>/dev/null || true; } | "$SDKMANAGER" "emulator" "$ANDROID_R8_PLATFORM" "$ANDROID_R8_SYSTEM_IMAGE"
printf 'no\n' | "$AVDMANAGER" create avd --force -n "$AVD_NAME" -k "$ANDROID_R8_SYSTEM_IMAGE" --device pixel

# Reject a silently misparsed target before spending the boot budget on an
# emulator whose metadata cannot represent the system image being tested.
if ! LC_ALL=C grep -Fxq -- "target=$ANDROID_R8_IMAGE_TARGET" "$ANDROID_AVD_HOME/$AVD_NAME.ini"; then
    printf 'Android AVD target does not match the pinned system image\n' >&2
    exit 1
fi
