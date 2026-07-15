// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use base64::{engine::general_purpose::STANDARD, Engine as _};

/// Encode bytes as standard padded Base64 from RFC 4648.
pub fn bytes_to_base64(bytes: &[u8]) -> String {
    STANDARD.encode(bytes)
}
