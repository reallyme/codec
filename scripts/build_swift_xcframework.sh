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
FFI_RUSTFLAGS="${RUSTFLAGS:+${RUSTFLAGS} }-C panic=unwind"

require_tool() {
  if ! command -v "$1" >/dev/null 2>&1; then
    printf 'required tool not found: %s\n' "$1" >&2
    exit 1
  fi
}

build_target() {
  local target="$1"
  rustup target add "${target}"
  RUSTFLAGS="${FFI_RUSTFLAGS}" \
    cargo build --locked -p reallyme-codec-ffi --release --target "${target}"
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

install_modulemaps() {
  local slice
  for slice in "${FRAMEWORK_DIR}"/*; do
    if [ -d "${slice}/Headers" ]; then
      mkdir -p "${slice}/Modules"
      cat >"${slice}/Modules/module.modulemap" <<'MODULEMAP'
module ReallyMeCodecFFI {
  header "reallyme_codec_ffi.h"
  export *
}
MODULEMAP
    fi
  done
}

verify_xcframework_layout() {
  local header_modulemap
  header_modulemap="$(find "${FRAMEWORK_DIR}" -path '*/Headers/module.modulemap' -print -quit)"
  if [ -n "${header_modulemap}" ]; then
    printf 'invalid SwiftPM artifact layout: module map must not be exported from Headers: %s\n' \
      "${header_modulemap}" >&2
    exit 1
  fi
}

require_tool cargo
require_tool rustup
require_tool xcodebuild
require_tool lipo
require_tool find
require_tool sort
require_tool swift
require_tool touch
require_tool zip

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

uint32_t rm_codec_abi_version(void);
size_t rm_codec_max_proto_result_envelope_bytes(void);

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
    const uint8_t *request_ptr,
    size_t request_len,
    uint8_t *out_ptr,
    size_t out_capacity,
    size_t *len_out);

rm_codec_status_t rm_codec_process_proto_json(
    const uint8_t *request_ptr,
    size_t request_len,
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

install_modulemaps
verify_xcframework_layout

rm -f "${ZIP_PATH}" "${CHECKSUM_PATH}"
(
  cd "${BUILD_DIR}"
  # SwiftPM checksums cover the archive bytes, so normalize metadata and entry
  # ordering to make independent release builds produce the same artifact.
  TZ=UTC find "ReallyMeCodecFFI.xcframework" -exec touch -t 198001010000 {} +
  find "ReallyMeCodecFFI.xcframework" -print \
    | LC_ALL=C sort \
    | zip -X -q "ReallyMeCodecFFI.xcframework.zip" -@
)
swift package compute-checksum "${ZIP_PATH}" >"${CHECKSUM_PATH}"
printf 'SwiftPM artifact: %s\n' "${ZIP_PATH}"
printf 'SwiftPM checksum: %s\n' "$(cat "${CHECKSUM_PATH}")"
