// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! URL-safe base64 without padding (RFC 4648 §5), rejecting non-canonical trailing bits.

mod decode;
mod encode;
mod error;
/// Serde adapter for required byte fields encoded as unpadded base64url strings.
#[cfg(feature = "serde")]
pub mod serde_bytes;
/// Serde adapter for optional byte fields encoded as unpadded base64url strings.
#[cfg(feature = "serde")]
pub mod serde_option_bytes;

pub use decode::{base64url_bytes_to_bytes, base64url_to_bytes};
pub use encode::bytes_to_base64url;
pub use error::Base64UrlError;
