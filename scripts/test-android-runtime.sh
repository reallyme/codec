#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

if [[ $# != 1 || ! "$1" =~ ^emulator-[0-9]+$ ]]; then
    printf 'usage: %s emulator-PORT\n' "$0" >&2
    exit 2
fi
readonly ANDROID_SERIAL="$1"
readonly ADB="${ADB:?Android Debug Bridge path is required}"
readonly ANDROID_R8_PAGE_SIZE_BYTES="16384"
readonly APP_ID="me.really.codec.consumer.r8"
readonly ACTIVITY="me.really.codec.consumer.r8.ConsumerR8RuntimeActivity"
readonly LOG_TAG="ReallyMeCodecR8Gate"

fail() {
    printf '%s\n' "$1" >&2
    exit 1
}

adb() {
    timeout --kill-after=2s 10s "$ADB" -s "$ANDROID_SERIAL" "$@"
}

page_size="$(adb shell getconf PAGE_SIZE | tr -d '\r')"
[[ "$page_size" == "$ANDROID_R8_PAGE_SIZE_BYTES" ]] || fail "Android runtime gate requires a 16 KB page-size device"

adb shell am force-stop "$APP_ID" >/dev/null 2>&1 || true
start_output=""
if ! start_output="$(adb shell am start -W -n "$APP_ID/$ACTIVITY" 2>&1)"; then
    printf '%s\n' "$start_output" >&2
    fail "Android consumer R8 activity launch failed"
fi
if [[ "$start_output" != *"Status: ok"* ]]; then
    printf '%s\n' "$start_output" >&2
    fail "Android consumer R8 activity launch did not report success"
fi

log_deadline=$((SECONDS + 60))
while (( SECONDS <= log_deadline )); do
    logs="$(adb logcat -d -v raw -s "$LOG_TAG:I" 2>/dev/null || true)"
    if grep -Fxq FAIL <<<"$logs"; then
        printf '%s\n' "$logs" >&2
        fail "Android consumer R8 runtime gate failed"
    fi
    if grep -Fxq PASS <<<"$logs"; then
        printf '%s\n' "Android consumer R8 runtime gate passed"
        exit 0
    fi
    sleep 1
done

adb logcat -d -v raw -s "$LOG_TAG:I" >&2 || true
fail "Android consumer R8 runtime gate timed out"
