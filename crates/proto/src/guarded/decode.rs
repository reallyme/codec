// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{SecretBytes, SecretString};
use buffa::bytes::Buf;
use buffa::types::{FieldPayload, WirePayload};
use buffa::{DecodeError, ProtoBytes, ProtoString};
use zeroize::Zeroizing;

fn copy_bytes(value: &[u8]) -> Result<SecretBytes, DecodeError> {
    let mut bytes = Zeroizing::new(Vec::new());
    bytes
        .try_reserve_exact(value.len())
        .map_err(|_| DecodeError::MessageTooLarge)?;
    bytes.extend_from_slice(value);
    Ok(core::mem::take(&mut *bytes).into())
}

fn read_bytes(payload: &mut FieldPayload<impl Buf>) -> Result<SecretBytes, DecodeError> {
    // The runtime has validated and bounded this payload. Reserve exactly once
    // before writing any secret, including when the input spans Buf chunks.
    let mut bytes = Zeroizing::new(Vec::new());
    bytes
        .try_reserve_exact(payload.remaining())
        .map_err(|_| DecodeError::MessageTooLarge)?;
    while payload.has_remaining() {
        let chunk = payload.chunk();
        let consumed = chunk.len();
        bytes.extend_from_slice(chunk);
        payload.advance(consumed);
    }
    Ok(core::mem::take(&mut *bytes).into())
}

fn text(bytes: SecretBytes) -> Result<SecretString, DecodeError> {
    // Validate before transferring ownership to String. Neither success nor
    // invalid UTF-8 releases an unguarded fragmented-input scratch buffer.
    core::str::from_utf8(bytes.as_slice()).map_err(|_| DecodeError::InvalidUtf8)?;
    let mut bytes = bytes.into_zeroizing();
    match String::from_utf8(core::mem::take(&mut *bytes)) {
        Ok(value) => Ok(value.into()),
        Err(error) => {
            let _owner = Zeroizing::new(error.into_bytes());
            Err(DecodeError::InvalidUtf8)
        }
    }
}

impl ProtoBytes for SecretBytes {
    fn from_wire(payload: WirePayload<'_>) -> Result<Self, DecodeError> {
        copy_bytes(payload.as_slice())
    }
    fn decode_length_delimited(payload: &mut FieldPayload<impl Buf>) -> Result<Self, DecodeError> {
        read_bytes(payload)
    }
}

impl ProtoString for SecretString {
    fn copy_from_str(value: &str) -> Self {
        value.into()
    }
    fn from_wire(payload: WirePayload<'_>) -> Result<Self, DecodeError> {
        text(copy_bytes(payload.as_slice())?)
    }
    fn decode_length_delimited(payload: &mut FieldPayload<impl Buf>) -> Result<Self, DecodeError> {
        text(read_bytes(payload)?)
    }
}
