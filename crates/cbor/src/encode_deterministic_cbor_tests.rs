// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{
    Preflight, MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
    MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES, MAX_DETERMINISTIC_CBOR_OUTPUT_LEN,
};
use crate::DeterministicCborError;

#[test]
fn each_preflight_byte_budget_accepts_its_exact_limit() {
    // Exercise each guard independently. A complete CBOR tree can hit
    // another budget first because headers add output bytes, which would
    // hide a one-step regression in these individual accounting guards.
    let mut stats = Preflight {
        encoded_len: MAX_DETERMINISTIC_CBOR_OUTPUT_LEN,
        ..Preflight::default()
    };
    assert!(stats.ensure_limits().is_ok());
    stats.encoded_len += 1;
    assert_eq!(
        stats.ensure_limits(),
        Err(DeterministicCborError::OutputTooLarge)
    );

    let mut stats = Preflight {
        aggregate_text_bytes: MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES,
        ..Preflight::default()
    };
    assert!(stats.ensure_limits().is_ok());
    stats.aggregate_text_bytes += 1;
    assert_eq!(
        stats.ensure_limits(),
        Err(DeterministicCborError::AggregateTextBytesExceeded)
    );

    let mut stats = Preflight {
        aggregate_byte_string_bytes: MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
        ..Preflight::default()
    };
    assert!(stats.ensure_limits().is_ok());
    stats.aggregate_byte_string_bytes += 1;
    assert_eq!(
        stats.ensure_limits(),
        Err(DeterministicCborError::AggregateByteStringBytesExceeded)
    );
}
