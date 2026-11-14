// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Verify that a repeated singular secret field leaves no stale bytes in its owner.

#![cfg(feature = "generated")]
#![allow(unsafe_code)]

use reallyme_codec_proto::generated::proto::reallyme::codec::v1::CodecPemEncodeRequest;
use reallyme_codec_proto::{decode_protobuf, CodecWireResult};

#[path = "support/allocator.rs"]
mod allocator;

#[global_allocator]
static ALLOCATOR: allocator::Probe = allocator::Probe;

const FIRST_DER_LENGTH: usize = 64;
const FIRST_DER_BYTE: u8 = 0xa5;
const LAST_DER_BYTE: u8 = 0x42;
const DER_FIELD_TAG: u8 = 0x12;

#[test]
fn duplicate_der_field_wipes_previous_capacity_before_replacement() -> CodecWireResult<()> {
    let mut wire = vec![DER_FIELD_TAG, 64];
    wire.extend(core::iter::repeat_n(FIRST_DER_BYTE, FIRST_DER_LENGTH));
    wire.extend([DER_FIELD_TAG, 1, LAST_DER_BYTE]);

    allocator::arm(FIRST_DER_LENGTH, 1, FIRST_DER_LENGTH);
    let decoded = decode_protobuf::<CodecPemEncodeRequest>(&wire)?;
    assert_eq!(decoded.der, [LAST_DER_BYTE]);
    // Native replacement retires the old owner instead of reusing its Vec.
    // Inspect its entire initialized allocation before free, not only the
    // surviving owner's spare capacity.
    allocator::assert_erased(Some(1));
    Ok(())
}
