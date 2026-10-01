// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Verify that a repeated singular secret field leaves no stale bytes in its owner.

#![cfg(feature = "generated")]
#![allow(unsafe_code)]

use reallyme_codec_proto::generated::proto::reallyme::codec::v1::CodecPemEncodeRequest;
use reallyme_codec_proto::{decode_protobuf, CodecWireResult};

const FIRST_DER_LENGTH: usize = 64;
const FIRST_DER_BYTE: u8 = 0xa5;
const LAST_DER_BYTE: u8 = 0x42;
const DER_FIELD_TAG: u8 = 0x12;

#[test]
fn duplicate_der_field_wipes_previous_capacity_before_replacement() -> CodecWireResult<()> {
    let mut wire = vec![DER_FIELD_TAG, 64];
    wire.extend(core::iter::repeat_n(FIRST_DER_BYTE, FIRST_DER_LENGTH));
    wire.extend([DER_FIELD_TAG, 1, LAST_DER_BYTE]);

    let mut decoded = decode_protobuf::<CodecPemEncodeRequest>(&wire)?;
    assert_eq!(decoded.der, [LAST_DER_BYTE]);
    let spare = decoded.der.spare_capacity_mut();
    assert!(spare.len() >= FIRST_DER_LENGTH - 1);
    for slot in spare.iter() {
        // SAFETY: The first wire merge initialized this allocation. The
        // generated pre-merge wipe writes every capacity byte before the
        // second, shorter value is copied into it. Reading the spare bytes is
        // confined to this regression test so it can detect stale DER.
        let byte = unsafe { slot.assume_init_ref() };
        assert_eq!(*byte, 0);
    }
    Ok(())
}
