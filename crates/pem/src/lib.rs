// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! PEM text armor parsing and encoding.
//!
//! This crate owns only the RFC 7468-style text envelope: BEGIN/END labels,
//! base64 body, line ending normalization, size limits, and strict label
//! matching. It deliberately does not interpret DER or cryptographic key
//! structure.

mod decode;
mod decode_bundle;
mod encode;
mod error;
mod frame_bundle;
mod label;
mod policy;
mod types;

pub use codec_proto::generated::proto::reallyme::codec::v1::{
    CodecPemBundleBlock, CodecPemBundleDecodeOptions, CodecPemBundleDecodeRequest,
    CodecPemBundleDecodeResult, CodecPemDecodeScope, CodecPemSyntaxProfile,
};
pub use decode::decode_pem;
pub use decode_bundle::decode_pem_bundle;
pub use encode::{encode_pem, preflight_pem_encoded_length};
pub use error::PemError;
pub use label::PemLabel;
pub use policy::{PemDecodePolicy, PemEncodeOptions, PemLineEnding};
pub use types::PemDocument;
