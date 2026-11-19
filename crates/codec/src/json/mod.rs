// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Guarded RFC 8259 decoding through the protobuf-defined JSON contract.
//!
//! Framing uses Serde's borrowed raw-value reader; decoded strings never enter
//! its private string scratch. Every decoded node and pending key has a wiping
//! owner. Callers retain ownership of input bytes and any exported copies.
//! Typed projections must use suitable owners for their own sensitive fields.

mod decode;
mod error;
mod limits;
mod project;
mod project_access;
mod project_key;
mod string;

pub use codec_proto::generated::proto::reallyme::codec::v1::{
    CodecJsonDecodeRequest as DecodeRequest, CodecJsonDecodeResult as DecodeResult,
    CodecJsonLimits as Limits, CodecJsonValue as Value,
};
pub use decode::{decode, from_slice};
pub use error::{ErrorReason, JsonError};
