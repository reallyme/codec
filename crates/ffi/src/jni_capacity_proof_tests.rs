// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{
    checked_input_total, probed_output_capacity, validate_output_copy, CopyFailure, OutputKind,
    CODEC_BUFFER_TOO_SMALL, CODEC_OK, MAX_CODEC_PROTO_MESSAGE_BYTES,
    MAX_CODEC_PROVIDER_OUTPUT_BYTES,
};

#[kani::proof]
fn jni_preflight_bounds_the_aggregate_before_copying() {
    let first: usize = kani::any();
    let second: usize = kani::any();
    let third: usize = kani::any();
    let maximum: usize = kani::any();
    let actual = checked_input_total(0, first, maximum)
        .and_then(|used| checked_input_total(used, second, maximum))
        .and_then(|used| checked_input_total(used, third, maximum));
    let total = wide(first) + wide(second) + wide(third);
    let expected = (total <= wide(maximum)).then(|| usize::try_from(total).unwrap());
    assert_eq!(actual, expected);
    kani::cover!(total == wide(maximum), "inputs reach the cap exactly");
    kani::cover!(
        first <= maximum && second <= maximum && third <= maximum && total > wide(maximum),
        "each input fits alone but the aggregate exceeds the cap"
    );
    kani::cover!(
        total > wide(usize::MAX),
        "sum cannot wrap into an allowed allocation"
    );
}

#[kani::proof]
fn jni_probe_admits_only_protocol_specific_capacities() {
    let protobuf: bool = kani::any();
    let status: i32 = kani::any();
    let produced: usize = kani::any();
    let kind = if protobuf {
        OutputKind::Protobuf
    } else {
        OutputKind::Generic
    };
    let cap = if protobuf {
        MAX_CODEC_PROTO_MESSAGE_BYTES
    } else {
        MAX_CODEC_PROVIDER_OUTPUT_BYTES
    };
    let actual = probed_output_capacity(kind, status, produced);
    let valid = (!protobuf && status == CODEC_OK && produced == 0)
        || (status == CODEC_BUFFER_TOO_SMALL && (1..=cap).contains(&produced));
    assert_eq!(actual, valid.then_some(produced));
    kani::cover!(
        !protobuf && status == CODEC_OK && produced == 0,
        "empty generic result"
    );
    kani::cover!(
        protobuf && status == CODEC_OK && produced == 0,
        "empty protobuf result rejected"
    );
    kani::cover!(
        protobuf && status == CODEC_BUFFER_TOO_SMALL && produced == cap,
        "exact protobuf cap"
    );
    kani::cover!(
        !protobuf && status == CODEC_BUFFER_TOO_SMALL && produced > cap,
        "generic cap exceeded"
    );
}

#[kani::proof]
fn jni_copy_requires_success_and_its_own_exact_length() {
    let capacity: usize = kani::any();
    let status: i32 = kani::any();
    let copied: usize = kani::any();
    kani::assume(capacity > 0);
    let actual = validate_output_copy(status, copied, capacity);
    let expected = match status {
        CODEC_OK if copied == capacity => Ok(()),
        CODEC_OK => Err(CopyFailure::LengthMismatch),
        other => Err(CopyFailure::Status(other)),
    };
    assert_eq!(actual, expected);
    kani::cover!(
        status == CODEC_OK && copied == capacity,
        "successful exact copy"
    );
    kani::cover!(
        status == CODEC_OK && copied == 0,
        "copy did not report any bytes"
    );
    kani::cover!(
        status == CODEC_OK && copied > capacity,
        "copy reported a larger extent"
    );
    kani::cover!(
        status != CODEC_OK && copied == capacity,
        "failure cannot publish output"
    );
}

fn wide(value: usize) -> u128 {
    u128::from(u64::try_from(value).unwrap())
}
