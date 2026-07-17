#!/usr/bin/env bash
# SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
#
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RESOURCES_ROOT="${1:-${ROOT_DIR}/packages/kotlin/native}"
FFI_RUSTFLAGS="${RUSTFLAGS:+${RUSTFLAGS} }-C panic=unwind"

case "$(uname -s)" in
  Darwin)
    LIBRARY_PATH="${ROOT_DIR}/target/release/libreallyme_codec_ffi.dylib"
    ;;
  Linux)
    LIBRARY_PATH="${ROOT_DIR}/target/release/libreallyme_codec_ffi.so"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    LIBRARY_PATH="${ROOT_DIR}/target/release/reallyme_codec_ffi.dll"
    ;;
  *)
    printf 'unsupported operating system for Kotlin native resource staging\n' >&2
    exit 1
    ;;
esac

RUSTFLAGS="${FFI_RUSTFLAGS}" cargo build --locked -p reallyme-codec-ffi --release
node "${ROOT_DIR}/scripts/stage_kotlin_native_resource.mjs" "${LIBRARY_PATH}" "${RESOURCES_ROOT}"
