#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly REPO_ROOT
readonly ANDROID_HOME_VALUE="${ANDROID_HOME:-"$HOME/Library/Android/sdk"}"
readonly ANDROID_NDK_HOME_VALUE="${ANDROID_NDK_HOME:-"$ANDROID_HOME_VALUE/ndk/29.0.14206865"}"
readonly ADB="${ADB:-"$ANDROID_HOME_VALUE/platform-tools/adb"}"
readonly EMULATOR="${EMULATOR:-"$ANDROID_HOME_VALUE/emulator/emulator"}"
readonly SDKMANAGER="${SDKMANAGER:-"$ANDROID_HOME_VALUE/cmdline-tools/latest/bin/sdkmanager"}"
readonly AVDMANAGER="${AVDMANAGER:-"$ANDROID_HOME_VALUE/cmdline-tools/latest/bin/avdmanager"}"
readonly ANDROID_AVD_HOME_VALUE="${ANDROID_AVD_HOME:-"$HOME/.android/avd"}"
readonly AVD_NAME="${REALLYME_CODEC_ANDROID_AVD:-}"
readonly ANDROID_R8_PLATFORM="platforms;android-37.0"
readonly ANDROID_R8_SYSTEM_IMAGE="system-images;android-37.1;google_apis_ps16k;x86_64"
readonly ANDROID_R8_DATA_PARTITION_MB="4096"
readonly ANDROID_R8_PAGE_SIZE_BYTES="16384"
readonly RUNTIME_SERVICES_READY_TIMEOUT_SECONDS="60"
readonly GUEST_LOG_TIMEOUT_MS="10000"
readonly GUEST_LOG_TAIL_LINES="300"
readonly ADB_INSTALL_MAX_ATTEMPTS="3"
readonly APP_ID="me.really.codec.consumer.r8"
readonly ACTIVITY="me.really.codec.consumer.r8.ConsumerR8RuntimeActivity"
readonly LOG_TAG="ReallyMeCodecR8Gate"
readonly JNILIBS_DIR="$REPO_ROOT/build/android-jniLibs"
readonly NATIVE_ASSETS_DIR="$REPO_ROOT/build/android-native-assets"
readonly APK_PATH="$REPO_ROOT/packages/kotlin-android/consumer-r8-runtime/build/outputs/apk/release/consumer-r8-runtime-release.apk"
LOG_DIR="$(mktemp -d "${TMPDIR:-/tmp}/reallyme-codec-r8.XXXXXXXX")"
readonly LOG_DIR
readonly EMULATOR_LOG="$LOG_DIR/emulator.log"
readonly AVDMANAGER_LOG="$LOG_DIR/avdmanager.log"

emulator_pid=""
export ANDROID_AVD_HOME="$ANDROID_AVD_HOME_VALUE"

# shellcheck disable=SC2329 # Reached through the EXIT trap.
dump_emulator_log() {
    if [[ -f "$EMULATOR_LOG" ]]; then
        tail -200 "$EMULATOR_LOG" >&2 || true
    fi
}

# shellcheck disable=SC2329 # Reached through the EXIT trap.
dump_guest_logs() {
    # Bound the ADB call because the guest may stop responding during a crash.
    printf 'Android guest logcat (system, events, crash):\n' >&2
    if ! node -e '
      const { spawnSync } = require("node:child_process");
      const [adb, timeout, lines] = process.argv.slice(1);
      const result = spawnSync(adb, ["logcat", "-d", "-t", lines, "-b", "system,events,crash", "-v", "threadtime"], {
        timeout: Number(timeout),
        killSignal: "SIGKILL",
        stdio: ["ignore", "inherit", "inherit"],
      });
      process.exitCode = result.status ?? 1;
    ' "$ADB" "$GUEST_LOG_TIMEOUT_MS" "$GUEST_LOG_TAIL_LINES" >&2; then
        printf 'Android guest logcat could not be fully collected\n' >&2
    fi
}

# shellcheck disable=SC2329 # Invoked by the EXIT trap.
cleanup() {
    local exit_status="$1"
    if [[ -n "$emulator_pid" ]]; then
        if (( exit_status != 0 )); then
            dump_guest_logs
            dump_emulator_log
        fi
        "$ADB" emu kill >/dev/null 2>&1 || true
        wait "$emulator_pid" >/dev/null 2>&1 || true
    fi
    if ! rm -f -- "$EMULATOR_LOG" "$AVDMANAGER_LOG" || ! rmdir -- "$LOG_DIR"; then
        printf 'Android runtime gate could not remove temporary logs\n' >&2
    fi
}
trap 'cleanup "$?"' EXIT

fail() {
    printf '%s\n' "$1" >&2
    exit 1
}

wait_for_runtime_services() {
    local runtime_services_deadline=$((SECONDS + RUNTIME_SERVICES_READY_TIMEOUT_SECONDS))
    local boot_completed=""
    local user_state=""
    local package_manager_state=""
    local storage_manager_state=""
    local volume_state=""
    local android_package_path=""
    while (( SECONDS <= runtime_services_deadline )); do
        boot_completed="$("$ADB" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r' || true)"
        user_state="$("$ADB" shell am get-started-user-state 0 2>/dev/null | tr -d '\r' || true)"
        package_manager_state="$("$ADB" shell service check package 2>/dev/null | tr -d '\r' || true)"
        storage_manager_state="$("$ADB" shell service check mount 2>/dev/null | tr -d '\r' || true)"
        volume_state="$("$ADB" shell sm list-volumes all 2>/dev/null || true)"
        android_package_path="$("$ADB" shell pm path android 2>/dev/null || true)"
        # Binder registration precedes full framework initialization. Waiting for
        # the unlocked user also covers the storage service's internal PM link.
        # The service command also prints "not found" for an absent service.
        if [[ "$boot_completed" == "1" ]] &&
            [[ "$user_state" == "RUNNING_UNLOCKED" ]] &&
            [[ "$package_manager_state" == "Service package: found" ]] &&
            [[ "$storage_manager_state" == "Service mount: found" ]] &&
            [[ "$volume_state" == *" mounted "* ]] &&
            [[ "$android_package_path" == package:* ]]; then
            return
        fi
        sleep 2
    done
    printf 'Android readiness at timeout: boot=%q user=%q package_service=%q mount_service=%q volumes=%q android_package=%q\n' \
        "$boot_completed" "$user_state" "$package_manager_state" "$storage_manager_state" \
        "$volume_state" "$android_package_path" >&2
    fail "Android package and storage services were not ready before timeout"
}

install_apk() {
    local install_attempt=1
    local install_output=""
    while (( install_attempt <= ADB_INSTALL_MAX_ATTEMPTS )); do
        # Native and Gradle builds may outlast a framework restart. Readiness
        # observed before those builds cannot authorize a later installation.
        wait_for_runtime_services
        if install_output="$("$ADB" install --no-streaming -r "$APK_PATH" 2>&1)"; then
            if [[ "$install_output" == *"Success"* ]]; then
                return
            fi
        fi

        printf '%s\n' "$install_output" >&2
        # Retry only known framework-startup failures. APK validation errors must
        # remain immediate failures rather than being hidden by generic retries.
        if { [[ "$install_output" != *"Broken pipe"* ]] &&
            [[ "$install_output" != *"StorageManager.getVolumes()"* ]] &&
            [[ "$install_output" != *"PackageManagerInternal.freeStorage"* ]]; } ||
            (( install_attempt == ADB_INSTALL_MAX_ATTEMPTS )); then
            break
        fi

        printf 'Android runtime services rejected install attempt %s; retrying after recovery\n' "$install_attempt" >&2
        sleep 5
        ((install_attempt += 1))
    done

    "$ADB" shell df -h /data >&2 || true
    fail "Android consumer R8 APK installation failed"
}

ensure_avd_exists() {
    mkdir -p "$ANDROID_AVD_HOME_VALUE"
    [[ -x "$SDKMANAGER" ]] || fail "Android sdkmanager is required at $SDKMANAGER"
    [[ -x "$AVDMANAGER" ]] || fail "Android avdmanager is required at $AVDMANAGER"

    if [[ ! -x "$EMULATOR" ]]; then
        { yes 2>/dev/null || true; } | "$SDKMANAGER" "emulator" >/dev/null
    fi
    [[ -x "$EMULATOR" ]] || fail "Android emulator is required at $EMULATOR"

    local avd_exists="false"
    while IFS= read -r existing_avd; do
        if [[ "$existing_avd" == "$AVD_NAME" ]]; then
            avd_exists="true"
            break
        fi
    done < <("$EMULATOR" -list-avds)

    if [[ "$avd_exists" == "true" ]]; then
        return
    fi

    { yes 2>/dev/null || true; } | "$SDKMANAGER" "emulator" "$ANDROID_R8_PLATFORM" "$ANDROID_R8_SYSTEM_IMAGE" >/dev/null
    printf 'no\n' | "$AVDMANAGER" create avd --force -n "$AVD_NAME" -k "$ANDROID_R8_SYSTEM_IMAGE" --device "pixel" >"$AVDMANAGER_LOG"

    while IFS= read -r existing_avd; do
        if [[ "$existing_avd" == "$AVD_NAME" ]]; then
            return
        fi
    done < <("$EMULATOR" -list-avds)

    if [[ -f "$AVDMANAGER_LOG" ]]; then
        cat "$AVDMANAGER_LOG" >&2 || true
    fi
    "$EMULATOR" -list-avds >&2 || true
    fail "Android AVD was not available after creation"
}

[[ -x "$ADB" ]] || fail "adb is required at $ADB"
[[ -d "$ANDROID_NDK_HOME_VALUE" ]] || fail "ANDROID_NDK_HOME is required at $ANDROID_NDK_HOME_VALUE"

if [[ -n "$AVD_NAME" ]]; then
    ensure_avd_exists
    # Runtime gates must not inherit a stale or undersized userdata image. The
    # Android 17 image's default partition can be too small for package install.
    "$EMULATOR" -avd "$AVD_NAME" -wipe-data -partition-size "$ANDROID_R8_DATA_PARTITION_MB" -no-window -no-audio -no-boot-anim -no-snapshot -gpu swiftshader_indirect >"$EMULATOR_LOG" 2>&1 &
    emulator_pid="$!"
fi

device_deadline=$((SECONDS + 120))
until "$ADB" get-state >/dev/null 2>&1; do
    if [[ -n "$emulator_pid" ]] && ! kill -0 "$emulator_pid" >/dev/null 2>&1; then
        fail "Android emulator exited before adb detected a device"
    fi
    if (( SECONDS > device_deadline )); then
        fail "Android device was not detected before timeout"
    fi
    sleep 2
done

boot_deadline=$((SECONDS + 240))
while [[ "$("$ADB" shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" != "1" ]]; do
    if (( SECONDS > boot_deadline )); then
        fail "Android device did not finish booting"
    fi
    sleep 2
done
wait_for_runtime_services
page_size=""
if ! page_size="$("$ADB" shell getconf PAGE_SIZE | tr -d '\r')"; then
    fail "Android runtime page size could not be determined"
fi
[[ "$page_size" == "$ANDROID_R8_PAGE_SIZE_BYTES" ]] || fail "Android runtime gate requires a 16 KB page-size device"

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
    :consumer-r8-runtime:assembleRelease \
    -Preallyme.codec.androidJniLibsDir="$JNILIBS_DIR" \
    -Preallyme.codec.androidNativeAssetsDir="$NATIVE_ASSETS_DIR" \
    -Preallyme.codec.requireAndroidJniLibs=true

install_apk

# Clear previous consumer results while preserving guest boot and crash logs.
"$ADB" logcat -b main -c
"$ADB" shell am force-stop "$APP_ID" >/dev/null 2>&1 || true
start_output=""
if ! start_output="$("$ADB" shell am start -W -n "$APP_ID/$ACTIVITY" 2>&1)"; then
    printf '%s\n' "$start_output" >&2
    fail "Android consumer R8 activity launch failed"
fi
if [[ "$start_output" != *"Status: ok"* ]]; then
    printf '%s\n' "$start_output" >&2
    fail "Android consumer R8 activity launch did not report success"
fi

log_deadline=$((SECONDS + 60))
while (( SECONDS <= log_deadline )); do
    logs="$("$ADB" logcat -d -v brief -s "$LOG_TAG:I" 2>/dev/null || true)"
    if [[ "$logs" == *"$LOG_TAG"*"FAIL"* ]]; then
        printf '%s\n' "$logs" >&2
        fail "Android consumer R8 runtime gate failed"
    fi
    if [[ "$logs" == *"$LOG_TAG"*"PASS"* ]]; then
        printf '%s\n' "Android consumer R8 runtime gate passed"
        exit 0
    fi
    sleep 1
done

"$ADB" logcat -d -v brief -s "$LOG_TAG:I" >&2 || true
fail "Android consumer R8 runtime gate timed out"
