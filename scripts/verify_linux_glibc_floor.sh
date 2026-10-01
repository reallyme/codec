#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 ReallyMe LLC
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

if [ "$#" -ne 1 ]; then
  echo "usage: verify_linux_glibc_floor.sh <native-resources-directory>" >&2
  exit 2
fi
if ! command -v readelf >/dev/null 2>&1; then
  echo "readelf is required to verify Linux native symbol versions" >&2
  exit 1
fi

readonly RESOURCE_ROOT="$1/me/really/codec/native"
readonly MAX_GLIBC_VERSION="2.34"
for architecture in linux-x86_64 linux-aarch64; do
  library="${RESOURCE_ROOT}/${architecture}/libreallyme_codec_ffi.so"
  if [ ! -f "$library" ]; then
    echo "missing Linux native resource for ${architecture}" >&2
    exit 1
  fi
  required="$(readelf --version-info "$library" | \
    grep -oE 'GLIBC_[0-9]+[.][0-9]+' | cut -d_ -f2 | sort -Vu | tail -n 1)"
  if [ -z "$required" ]; then
    echo "Linux native resource does not declare glibc symbol versions: ${architecture}" >&2
    exit 1
  fi
  highest="$(printf '%s\n%s\n' "$required" "$MAX_GLIBC_VERSION" | sort -V | tail -n 1)"
  if [ "$highest" != "$MAX_GLIBC_VERSION" ]; then
    echo "Linux native resource exceeds the documented glibc floor: ${architecture}" >&2
    exit 1
  fi
done

echo "Linux native glibc symbol versions do not exceed ${MAX_GLIBC_VERSION}"
