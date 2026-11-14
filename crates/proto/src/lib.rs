// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! ReallyMe codec protobuf error envelopes with generated Buffa bindings.

/// Native wipe-owning field representations selected by the generator.
#[cfg(feature = "generated")]
pub mod guarded;

/// Generated protobuf boundary.
pub mod generated;

mod provider_limits;
pub use provider_limits::{
    CODEC_PROVIDER_ABI_VERSION, MAX_CODEC_PROVIDER_INPUT_BYTES, MAX_CODEC_PROVIDER_OUTPUT_BYTES,
};

#[cfg(feature = "generated")]
mod error;
#[cfg(feature = "generated")]
mod limits;
#[cfg(feature = "generated")]
mod wire;

#[cfg(feature = "generated")]
pub use error::{
    codec_api_error, codec_error, CodecWireError, CodecWireErrorBranch,
    CodecWireErrorConstructionError, CodecWireErrorOrigin, CodecWireResult,
};
#[cfg(feature = "generated")]
pub use limits::{
    MAX_CODEC_PROTO_ERROR_ENVELOPE_BYTES, MAX_CODEC_PROTO_JSON_BYTES,
    MAX_CODEC_PROTO_MESSAGE_BYTES, MAX_CODEC_PROTO_STRUCTURED_MESSAGE_BYTES,
    MAX_CODEC_TLV_INPUT_BYTES,
};
#[cfg(feature = "generated")]
pub use wire::{decode_json, decode_protobuf, encode_protobuf};
