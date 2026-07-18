// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! PEM text armor parsing and encoding.
//!
//! This crate owns only the RFC 7468-style text envelope: BEGIN/END labels,
//! base64 body, line ending normalization, size limits, and strict label
//! matching. It deliberately does not interpret DER or cryptographic key
//! structure.

mod decode;
mod encode;
mod error;
mod label;
mod policy;
mod types;

pub use decode::decode_pem;
pub use encode::encode_pem;
pub use error::PemError;
pub use label::PemLabel;
pub use policy::{PemDecodePolicy, PemEncodeOptions, PemLineEnding};
pub use types::PemDocument;
