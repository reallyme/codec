#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

for tool in gh jar; do
    if ! command -v "$tool" >/dev/null; then
        printf 'Maven staging failed: %s is required\n' "$tool" >&2
        exit 1
    fi
done

script_dir="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd -P)"
cd -- "$script_dir/.."
exec node "$script_dir/stage_maven_central_bundle.mjs" "$@"
