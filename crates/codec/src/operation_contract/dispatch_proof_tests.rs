// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::super::{admit_operation_field, execute_scalars::check_lengths};
use super::{proto, require_operation, ApiRequest, Operation};
use codec_proto::generated::wire_policy::OPERATION_FIELD_NUMBERS;
use codec_proto::{
    CodecWireError, CodecWireErrorBranch, CodecWireErrorOrigin, MAX_CODEC_PROVIDER_INPUT_BYTES,
};
use core::mem::ManuallyDrop;
use proto::CodecErrorReason as Reason;

#[kani::proof]
#[kani::unwind(64)]
fn operation_admission_rejects_a_second_or_malformed_registered_field() {
    let field: u64 = kani::any();
    let wire_type: u64 = kani::any();
    let previously_seen: bool = kani::any();
    let recognized = OPERATION_FIELD_NUMBERS.contains(&field);
    let mut seen = previously_seen;
    let outcome = admit_operation_field(field, wire_type, &mut seen);

    if !recognized {
        assert!(matches!(outcome, Ok(false)));
        assert_eq!(seen, previously_seen);
    } else if wire_type == 2 && !previously_seen {
        assert!(matches!(outcome, Ok(true)));
        assert!(seen);
    } else {
        assert!(
            matches!(outcome, Err(error) if is_boundary_error(error, Reason::CODEC_ERROR_REASON_BOUNDARY_MALFORMED_PROTOBUF))
        );
        assert_eq!(seen, previously_seen);
    }

    kani::cover!(
        recognized && wire_type == 2 && !previously_seen,
        "single registered operation field"
    );
    kani::cover!(
        recognized && wire_type == 2 && previously_seen,
        "second operation field is rejected"
    );
    kani::cover!(
        recognized && wire_type != 2 && !previously_seen,
        "registered tag with wrong wire type"
    );
    kani::cover!(
        !recognized && previously_seen,
        "unknown field leaves the selector unchanged"
    );
}

#[kani::proof]
#[kani::unwind(8)]
fn the_generated_operation_guard_preserves_presence_and_selection() {
    let present: bool = kani::any();
    let hex: bool = kani::any();
    let input = present.then(|| {
        let request = Box::new(proto::CodecBytesRequest::default());
        if hex {
            Operation::LowerHexEncode(request)
        } else {
            Operation::Base64Encode(request)
        }
    });
    // Fixture owners isolate the selector transfer. Native owner tests
    // exercise the generated zeroization paths.
    let mut request = ManuallyDrop::new(ApiRequest { operation: input });
    let outcome = ManuallyDrop::new(require_operation(&mut request));
    assert!(request.operation.is_none());
    assert!(match &*outcome {
        Ok(Operation::LowerHexEncode(_)) => present && hex,
        Ok(Operation::Base64Encode(_)) => present && !hex,
        Err(error) =>
            !present
                && is_boundary_error(
                    *error,
                    Reason::CODEC_ERROR_REASON_BOUNDARY_MISSING_OPERATION
                ),
        _ => false,
    });
    kani::cover!(
        !present,
        "empty generated envelope returns a typed caller error"
    );
    kani::cover!(present && hex, "lowercase hexadecimal variant is retained");
    kani::cover!(present && !hex, "padded base64 variant is retained");
}

#[kani::proof]
#[kani::unwind(4)]
fn resource_admission_aggregates_both_scalar_fields_without_wrapping() {
    let lengths: [usize; 2] = kani::any();
    let total = u128::from(u64::try_from(lengths[0]).unwrap())
        + u128::from(u64::try_from(lengths[1]).unwrap());
    let cap = u128::from(u64::try_from(MAX_CODEC_PROVIDER_INPUT_BYTES).unwrap());
    let outcome = check_lengths(&lengths);
    if total <= cap {
        assert!(outcome.is_ok());
    } else {
        assert!(matches!(outcome, Err(error)
            if error.branch() == CodecWireErrorBranch::BaseEncoding
                && error.origin() == CodecWireErrorOrigin::Caller
                && error.reason() == Reason::CODEC_ERROR_REASON_BASE_INPUT_TOO_LARGE));
    }
    kani::cover!(total == cap && lengths[1] > 0, "exact aggregate cap");
    kani::cover!(
        total == cap + 1 && lengths[0] > 0 && lengths[1] > 0,
        "either field fits separately"
    );
}

fn is_boundary_error(error: CodecWireError, reason: Reason) -> bool {
    error.reason() == reason
        && error.branch() == CodecWireErrorBranch::Boundary
        && error.origin() == CodecWireErrorOrigin::Caller
}
