#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 ReallyMe LLC
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

if [ "$#" -ne 4 ]; then
  printf 'usage: check_swift_static_library_paths.sh LIBRARY ROOT CARGO_HOME RUSTUP_HOME\n' >&2
  exit 2
fi

library="$1"
root_prefix="${2%/}/"
cargo_prefix="${3%/}/"
rustup_prefix="${4%/}/"

# The Rust standard library can contain paths below the runner's home that
# are unrelated to this build. Check only the source paths passed to rustc's
# remap flags, since those are the paths this artifact must not disclose.
if ! strings "$library" | LC_ALL=C awk \
  -v root_prefix="$root_prefix" \
  -v cargo_prefix="$cargo_prefix" \
  -v rustup_prefix="$rustup_prefix" '
    index($0, root_prefix) > 0 ||
    index($0, cargo_prefix) > 0 ||
    index($0, rustup_prefix) > 0 { found = 1 }
    END { exit found }
  '; then
  printf 'Swift artifact contains an unremapped source path\n' >&2
  exit 1
fi
