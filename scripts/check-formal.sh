#!/usr/bin/env bash
# SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
#
# SPDX-License-Identifier: MIT OR Apache-2.0

set -euo pipefail

cargo fetch --locked
export CARGO_NET_OFFLINE=true
report_dir=$(mktemp -d)
trap 'rm -rf -- "$report_dir"' EXIT

verify_harness() {
  local package=$1
  local harness=$2
  local report="$report_dir/${harness##*::}.json"
  cargo kani --package "$package" --lib --no-default-features \
    --exact --harness "$harness" --output-format terse \
    -Z unstable-options --harness-timeout 60s --export-json "$report"

  # Cover failures must fail CI even when all assertions pass.
  jq --exit-status --arg harness "$harness" '
    .verification_results.results as $results
    | ($results | length == 1) and all($results[];
        .harness_id == $harness and .status == "Success"
        and ([.checks[] | select(.category == "cover")] as $covers
          | ($covers | length > 0) and all($covers[]; .status == "Satisfied")))
  ' "$report"
}

verify_harness reallyme-codec-cbor \
  decode_deterministic_cbor::proofs::mixed_key_wire_admission_rejects_bytewise_order
verify_harness reallyme-codec-cbor cid::proofs::cid_v0_wire_requires_the_bare_fixed_shape
verify_harness reallyme-codec-jcs \
  canonicalize::proofs::signed_and_unsigned_integer_admission_matches_the_interoperable_range
verify_harness reallyme-codec-ffi pointer::proofs::byte_range_validation_matches_address_and_slice_limits
