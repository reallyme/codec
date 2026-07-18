// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! ReallyMe codec protobuf error envelopes with generated Buffa bindings.

/// Generated protobuf boundary.
pub mod generated;

#[cfg(feature = "generated")]
mod error;
#[cfg(feature = "generated")]
mod limits;
#[cfg(feature = "generated")]
mod wire;

#[cfg(feature = "generated")]
pub use error::{
    codec_error, CodecWireError, CodecWireErrorBranch, CodecWireErrorConstructionError,
    CodecWireErrorOrigin, CodecWireResult,
};
#[cfg(feature = "generated")]
pub use limits::{
    MAX_CODEC_PROTO_ERROR_ENVELOPE_BYTES, MAX_CODEC_PROTO_JSON_BYTES, MAX_CODEC_PROTO_MESSAGE_BYTES,
};
#[cfg(feature = "generated")]
pub use wire::{decode_json, decode_protobuf, encode_protobuf};
