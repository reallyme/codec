// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Standard base64 (RFC 4648) encode/decode with canonical padding required on decode.

mod decode;
mod encode;
mod error;

pub use decode::base64_to_bytes;
pub use encode::bytes_to_base64;
pub use error::Base64Error;
