// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};

/// Encode bytes as RFC 4648 section 5 Base64URL without padding.
pub fn bytes_to_base64url(bytes: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}
