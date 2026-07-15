// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Multibase encode/decode. Decoding is panic-free on arbitrary input, including strings whose first character is multi-byte.

mod base58btc;
mod decode;
mod encode;
mod error;

pub use base58btc::{base58btc_decode, base58btc_encode, Base58Error, MAX_BASE58BTC_INPUT_LEN};
pub use decode::multibase_to_bytes;
pub use encode::{bytes_to_multibase58btc, bytes_to_multibase_base64url};
pub use error::MultibaseError;
