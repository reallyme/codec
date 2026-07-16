// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Canonical lowercase hexadecimal encoding and decoding.

#![forbid(unsafe_code)]

mod decode;
mod encode;
mod error;

pub use decode::lower_hex_to_bytes;
pub use encode::{bytes_to_lower_hex, write_lower_hex};
pub use error::HexError;
