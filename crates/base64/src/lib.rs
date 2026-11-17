// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Standard base64 (RFC 4648) encode/decode with canonical padding required on decode.

mod decode;
mod encode;
mod error;

pub use codec_proto::generated::proto::reallyme::codec::v1::CodecBase64DecodeLimits as Base64DecodeLimits;
pub use decode::{
    base64_to_bytes, base64_to_bytes_with_limits, DEFAULT_BASE64_DECODED_OUTPUT_BYTES,
};
mod limits;
pub use encode::bytes_to_base64;
pub use error::Base64Error;
pub use limits::MAX_BASE64_INPUT_LEN;
