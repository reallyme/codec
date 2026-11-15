#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

version=0.68.0
checksum=32e2b484d73ede0bbf64a2cf0879c4259422497d8aec4ee67d448de9ae7843d3
bundle="kani-$version-x86_64-unknown-linux-gnu.tar.gz"
download_dir=$(mktemp -d)
trap 'rm -rf -- "$download_dir"' EXIT

cargo install --locked kani-verifier --version "=$version"
bundle_path="$download_dir/$bundle"
curl --fail --silent --show-error --location --retry 3 --max-time 180 \
  --output "$bundle_path" "https://github.com/model-checking/kani/releases/download/kani-$version/$bundle"
printf '%s  %s\n' "$checksum" "$bundle_path" | sha256sum --check --strict
cargo kani setup --use-local-bundle "$bundle_path"
cargo kani --version
