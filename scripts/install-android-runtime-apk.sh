#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

if [[ $# != 2 ]]; then
    printf 'usage: %s device-serial /absolute/consumer.apk\n' "$0" >&2
    exit 2
fi

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly helper="$repo_root/.release-readiness/scripts/android-runtime.mjs"
readonly max_install_attempts=3
readonly retryable_device_exit=75
readonly service_recovery_seconds=60
readonly service_retry_delay_seconds=5
# Preserve the combined budget for ADB discovery, cold boot, and services.
wait_seconds=420

for ((attempt = 1; attempt <= max_install_attempts; attempt += 1)); do
    node "$helper" wait "$1" "$wait_seconds"
    if node "$helper" install "$1" "$2"; then
        exit 0
    else
        result=$?
    fi
    if (( result != retryable_device_exit || attempt == max_install_attempts )); then
        exit "$result"
    fi

    # A temporary package service disconnect can clear on a running device.
    # Recheck readiness before retrying, so cold starts do not repeat the race.
    printf 'Android package service disconnected on install attempt %s; waiting for recovery.\n' "$attempt" >&2
    sleep "$service_retry_delay_seconds"
    wait_seconds=$service_recovery_seconds
done
