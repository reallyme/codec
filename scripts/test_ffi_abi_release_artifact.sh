#!/usr/bin/env bash
# SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

readonly ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
readonly SYMBOLS=(
  rm_codec_abi_version
  rm_codec_max_ffi_input_bytes
  rm_codec_max_ffi_output_bytes
  rm_codec_max_operation_response_bytes
  rm_codec_process
  rm_codec_process_bool
  rm_codec_process_operation
  rm_codec_process_operation_json
)

case "$(uname -s)" in
  Darwin)
    readonly LIBRARY_PATH="${ROOT_DIR}/target/release/libreallyme_codec_ffi.dylib"
    ;;
  Linux)
    readonly LIBRARY_PATH="${ROOT_DIR}/target/release/libreallyme_codec_ffi.so"
    ;;
  MINGW*|MSYS*|CYGWIN*)
    readonly LIBRARY_PATH="${ROOT_DIR}/target/release/reallyme_codec_ffi.dll"
    ;;
  *)
    echo "unsupported host for release artifact ABI test" >&2
    exit 1
    ;;
esac

cd "${ROOT_DIR}"

TEMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/reallyme-ffi-abi.XXXXXX")"
readonly TEMP_DIR
trap 'rm -rf "${TEMP_DIR}"' EXIT

if env -u RUSTFLAGS cargo build --locked -p reallyme-codec-ffi --release --target-dir "${ROOT_DIR}/target" >"${TEMP_DIR}/abort-build.log" 2>&1; then
  echo "release FFI build unexpectedly succeeded without panic=unwind" >&2
  exit 1
fi
if ! grep -Fq "reallyme-codec-ffi must be compiled with panic=unwind" "${TEMP_DIR}/abort-build.log"; then
  echo "release FFI build failed before verifying the panic=unwind requirement" >&2
  exit 1
fi

readonly FFI_RUSTFLAGS="${RUSTFLAGS:+${RUSTFLAGS} }-C panic=unwind"
RUSTFLAGS="${FFI_RUSTFLAGS}" cargo build --locked -p reallyme-codec-ffi --release --target-dir "${ROOT_DIR}/target"

if [[ ! -f "${LIBRARY_PATH}" ]]; then
  echo "release FFI artifact was not produced at ${LIBRARY_PATH}" >&2
  exit 1
fi

if command -v nm >/dev/null 2>&1; then
  readonly NM_TOOL="nm"
elif command -v llvm-nm >/dev/null 2>&1; then
  readonly NM_TOOL="llvm-nm"
else
  echo "nm or llvm-nm is required for release artifact symbol verification" >&2
  exit 1
fi

readonly SYMBOL_DUMP="${TEMP_DIR}/symbols"

"${NM_TOOL}" -g "${LIBRARY_PATH}" >"${SYMBOL_DUMP}"

for symbol in "${SYMBOLS[@]}"; do
  if ! grep -Eq "(^|[[:space:]])_?${symbol}$" "${SYMBOL_DUMP}"; then
    echo "release FFI artifact is missing exported symbol ${symbol}" >&2
    exit 1
  fi
done

echo "release FFI ABI artifact verified: ${LIBRARY_PATH}"
