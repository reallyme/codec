#!/usr/bin/env bash
# SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
#
# SPDX-License-Identifier: Apache-2.0

set -euo pipefail

readonly TOOLCHAIN="${REALLYME_CODEC_SANITIZER_TOOLCHAIN:-nightly-2026-07-01}"
readonly TARGET="${REALLYME_CODEC_SANITIZER_TARGET:-$(rustc +"$TOOLCHAIN" -vV | sed -n 's/^host: //p')}"
readonly PACKAGES=(
    -p reallyme-codec
    -p reallyme-codec-ffi
)
readonly TEST_ARGS=(
    test
    --locked
    "${PACKAGES[@]}"
    --tests
    --target "$TARGET"
)

RUSTFLAGS="-Zsanitizer=address" cargo +"$TOOLCHAIN" "${TEST_ARGS[@]}"

# The pinned Rust nightly used for this release does not expose LLVM's
# UndefinedBehaviorSanitizer as `-Zsanitizer=undefined`. These runtime UB
# checks are the auditable UBSan-style lane available on this toolchain.
RUSTFLAGS="-Zub-checks=yes -Zextra-const-ub-checks=yes" cargo +"$TOOLCHAIN" "${TEST_ARGS[@]}"
