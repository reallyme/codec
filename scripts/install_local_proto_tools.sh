#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
#
# SPDX-License-Identifier: MIT OR Apache-2.0

# Only public pinned tool artifacts are fetched. Schema build and generation
# run locally after installation, without a remote generation service.
set -euo pipefail
cd "$(dirname "$0")/.."
if [[ "$(uname -s)" != Darwin ]]; then
  printf '%s\n' 'Install protoc 36.2 and protoc-gen-swift 1.39.0 locally on this platform.' >&2
  exit 1
fi

tools_root="$PWD/.build/codec-proto-tools"
mkdir -p "$tools_root/bin"
if [[ ! -x "$tools_root/bin/protoc" ]] || [[ "$("$tools_root/bin/protoc" --version)" != 'libprotoc 36.2' ]]; then
  archive="$tools_root/protoc.zip"
  curl --fail --location --proto '=https' --tlsv1.2 --retry 3 \
    --output "$archive" \
    https://github.com/protocolbuffers/protobuf/releases/download/v36.2/protoc-36.2-osx-universal_binary.zip
  printf '%s  %s\n' '83d0dc80672f53486f54299bca6177159f9cad1fd54d4d3143eeea53c4423e1b' "$archive" | shasum -a 256 --check
  unzip -q -o "$archive" -d "$tools_root"
fi
if [[ ! -x "$tools_root/bin/protoc-gen-swift" ]] || [[ "$("$tools_root/bin/protoc-gen-swift" --version)" != 'protoc-gen-swift 1.39.0' ]]; then
  swift_source="$tools_root/swift-protobuf"
  swift_revision=3d537cb81aef272a03cb3be815439e9bf0eb3bd3
  git init --quiet "$swift_source"
  git -C "$swift_source" fetch --depth 1 https://github.com/apple/swift-protobuf.git "$swift_revision"
  test "$(git -C "$swift_source" rev-parse FETCH_HEAD)" = "$swift_revision"
  git -C "$swift_source" checkout --detach FETCH_HEAD
  swift build --package-path "$swift_source" --configuration release --product protoc-gen-swift
  install -m 755 "$swift_source/.build/release/protoc-gen-swift" "$tools_root/bin/protoc-gen-swift"
fi
PATH="$tools_root/bin:$PATH" node scripts/check_local_proto_tools.mjs
