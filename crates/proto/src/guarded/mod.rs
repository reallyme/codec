// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Wipe-owning storage selected by the native protobuf generator.
//!
//! These types enforce allocation ownership, not key lengths or application
//! validity. Generated messages remain untrusted until semantic admission.
//! Explicitly exported copies remain the receiving caller's responsibility.

mod bytes;
mod decode;
mod error;
mod json_bytes;
mod json_string;
mod string;

pub use bytes::SecretBytes;
pub use string::SecretString;
