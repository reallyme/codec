#!/usr/bin/env bash
# SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
#
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BUILD_DIR="${ROOT_DIR}/build/swift"
HEADERS_DIR="${BUILD_DIR}/headers"
FRAMEWORK_DIR="${BUILD_DIR}/ReallyMeCodecFFI.xcframework"
ZIP_PATH="${BUILD_DIR}/ReallyMeCodecFFI.xcframework.zip"
CHECKSUM_PATH="${BUILD_DIR}/ReallyMeCodecFFI.xcframework.checksum"

require_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'required tool not found: %s\n' "$1" >&2
    exit 1
  fi
}

build_target() {
  local target="$1"
  rustup target add "${target}"
  cargo build -p reallyme-codec-ffi --release --target "${target}"
}

copy_or_lipo() {
  local output="$1"
  shift
  if [ "$#" -eq 1 ]; then
    cp "$1" "${output}"
  else
    lipo -create "$@" -output "${output}"
  fi
}

require_tool cargo
require_tool rustup
require_tool xcodebuild
require_tool lipo
require_tool ditto
require_tool swift

rm -rf "${BUILD_DIR}"
mkdir -p "${HEADERS_DIR}" "${BUILD_DIR}/libs"

cat >"${HEADERS_DIR}/reallyme_codec_ffi.h" <<'HEADER'
// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

#ifndef REALLYME_CODEC_FFI_H
#define REALLYME_CODEC_FFI_H

#include <stddef.h>
#include <stdint.h>

typedef int32_t rm_codec_status_t;

rm_codec_status_t rm_codec_process(
    uint32_t operation,
    const uint8_t *first_ptr,
    size_t first_len,
    const uint8_t *second_ptr,
    size_t second_len,
    const uint8_t *third_ptr,
    size_t third_len,
    uint8_t *out_ptr,
    size_t out_capacity,
    size_t *len_out);

rm_codec_status_t rm_codec_process_proto(
    uint32_t operation,
    const uint8_t *first_ptr,
    size_t first_len,
    const uint8_t *second_ptr,
    size_t second_len,
    const uint8_t *third_ptr,
    size_t third_len,
    uint8_t *out_ptr,
    size_t out_capacity,
    size_t *len_out);

rm_codec_status_t rm_codec_process_bool(
    uint32_t operation,
    const uint8_t *first_ptr,
    size_t first_len,
    const uint8_t *second_ptr,
    size_t second_len,
    int32_t *result_out);

#endif
HEADER

cat >"${HEADERS_DIR}/module.modulemap" <<'MODULEMAP'
module ReallyMeCodecFFI {
  header "reallyme_codec_ffi.h"
  export *
}
MODULEMAP

build_target aarch64-apple-darwin
build_target x86_64-apple-darwin
build_target aarch64-apple-ios
build_target aarch64-apple-ios-sim
build_target x86_64-apple-ios

copy_or_lipo \
  "${BUILD_DIR}/libs/libreallyme_codec_ffi_macos.a" \
  "${ROOT_DIR}/target/aarch64-apple-darwin/release/libreallyme_codec_ffi.a" \
  "${ROOT_DIR}/target/x86_64-apple-darwin/release/libreallyme_codec_ffi.a"

copy_or_lipo \
  "${BUILD_DIR}/libs/libreallyme_codec_ffi_ios.a" \
  "${ROOT_DIR}/target/aarch64-apple-ios/release/libreallyme_codec_ffi.a"

copy_or_lipo \
  "${BUILD_DIR}/libs/libreallyme_codec_ffi_ios_simulator.a" \
  "${ROOT_DIR}/target/aarch64-apple-ios-sim/release/libreallyme_codec_ffi.a" \
  "${ROOT_DIR}/target/x86_64-apple-ios/release/libreallyme_codec_ffi.a"

xcodebuild -create-xcframework \
  -library "${BUILD_DIR}/libs/libreallyme_codec_ffi_macos.a" -headers "${HEADERS_DIR}" \
  -library "${BUILD_DIR}/libs/libreallyme_codec_ffi_ios.a" -headers "${HEADERS_DIR}" \
  -library "${BUILD_DIR}/libs/libreallyme_codec_ffi_ios_simulator.a" -headers "${HEADERS_DIR}" \
  -output "${FRAMEWORK_DIR}"

rm -f "${ZIP_PATH}" "${CHECKSUM_PATH}"
(
  cd "${BUILD_DIR}"
  ditto -c -k --sequesterRsrc --keepParent "ReallyMeCodecFFI.xcframework" "ReallyMeCodecFFI.xcframework.zip"
)
swift package compute-checksum "${ZIP_PATH}" >"${CHECKSUM_PATH}"
printf 'SwiftPM artifact: %s\n' "${ZIP_PATH}"
printf 'SwiftPM checksum: %s\n' "$(cat "${CHECKSUM_PATH}")"
