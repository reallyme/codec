#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

if [[ "$#" -lt 1 || "$#" -gt 2 ]]; then
  printf 'usage: %s <Codec XCFramework zip> [CSecp256k1 XCFramework zip]\n' "$0" >&2
  exit 1
fi

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
FIXTURE_DIR="${ROOT_DIR}/packages/swift/Tests/Fixtures/TwoBinaryTargets"
CONSUMER_DIR="$(mktemp -d "${TMPDIR:-/tmp}/codec-xcframework-consumer.XXXXXXXX")"
trap 'rm -rf "${CONSUMER_DIR}"' EXIT

# Use the published binary from Crypto's dependency graph. Its top-level
# module map exercises the shared include directory used by Swift Build.
SECP256K1_URL="https://github.com/reallyme/CSecp256k1/releases/download/0.1.0/CSecp256k1.xcframework.zip"
SECP256K1_CHECKSUM="bfe8cbeb8ffc7b2c7ee88392db2c0ee7f9bebc6f082fb6ec706e95d1ec006781"
secp256k1_archive="${2:-${CONSUMER_DIR}/CSecp256k1.xcframework.zip}"
if [[ "$#" -eq 1 ]]; then
  curl --fail --show-error --silent --location --retry 3 --max-time 120 \
    --output "${secp256k1_archive}" "${SECP256K1_URL}"
fi
if [[ "$(swift package compute-checksum "${secp256k1_archive}")" != "${SECP256K1_CHECKSUM}" ]]; then
  printf 'CSecp256k1 archive checksum mismatch\n' >&2
  exit 1
fi

cp -R "${FIXTURE_DIR}/." "${CONSUMER_DIR}"
ditto -x -k "$1" "${CONSUMER_DIR}"
ditto -x -k "${secp256k1_archive}" "${CONSUMER_DIR}"

# SwiftPM 6.4 defaults to Swift Build. Leave the engine selection to SwiftPM
# so this checks the same invocation a consumer uses.
swift run --package-path "${CONSUMER_DIR}" --scratch-path "${CONSUMER_DIR}/.build/macos" BinaryConsumer

simulator_sdk="$(xcrun --sdk iphonesimulator --show-sdk-path)"
for arch in arm64 x86_64; do
  swift build --package-path "${CONSUMER_DIR}" \
    --scratch-path "${CONSUMER_DIR}/.build/ios-simulator-${arch}" \
    --triple "${arch}-apple-ios16.0-simulator" --sdk "${simulator_sdk}" \
    --product BinaryConsumer
done
