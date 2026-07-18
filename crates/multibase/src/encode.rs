// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use base64::{encoded_len, engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use zeroize::Zeroizing;

use crate::{Base58Error, MultibaseError, MAX_BASE58BTC_INPUT_LEN};

const MULTIBASE_BASE58BTC_PREFIX: u8 = b'z';
const MULTIBASE_BASE64URL_PREFIX: u8 = b'u';

/// Encode bytes using multibase base58btc with the `z` prefix.
pub fn bytes_to_multibase58btc(bytes: &[u8]) -> Result<String, Base58Error> {
    if bytes.len() > MAX_BASE58BTC_INPUT_LEN {
        return Err(Base58Error::InputTooLarge);
    }
    let mut output = Zeroizing::new(Vec::with_capacity(1));
    output.push(MULTIBASE_BASE58BTC_PREFIX);
    bs58::encode(bytes)
        .onto(&mut *output)
        .map_err(|_| Base58Error::BufferTooSmall)?;
    match String::from_utf8(core::mem::take(&mut *output)) {
        Ok(encoded) => Ok(encoded),
        Err(error) => {
            let _bytes = Zeroizing::new(error.into_bytes());
            Err(Base58Error::DecodeFailed)
        }
    }
}

/// Encode bytes using multibase base64url with the `u` prefix.
pub fn bytes_to_multibase_base64url(bytes: &[u8]) -> Result<String, MultibaseError> {
    let encoded_length = encoded_len(bytes.len(), false).ok_or(MultibaseError::LengthOverflow)?;
    let output_length = encoded_length
        .checked_add(1)
        .ok_or(MultibaseError::LengthOverflow)?;
    let mut output = Zeroizing::new(vec![0_u8; output_length]);
    output[0] = MULTIBASE_BASE64URL_PREFIX;
    let written = URL_SAFE_NO_PAD
        .encode_slice(bytes, &mut output[1..])
        .map_err(|_| MultibaseError::LengthOverflow)?;
    if written != encoded_length {
        return Err(MultibaseError::LengthOverflow);
    }
    match String::from_utf8(core::mem::take(&mut *output)) {
        Ok(encoded) => Ok(encoded),
        Err(error) => {
            let _bytes = Zeroizing::new(error.into_bytes());
            Err(MultibaseError::LengthOverflow)
        }
    }
}
