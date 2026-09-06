// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use serde::{Deserialize, Deserializer, Serializer};
use zeroize::Zeroizing;

use crate::{base64url_to_bytes, bytes_to_base64url};

/// Serialize bytes as an unpadded base64url string.
pub fn serialize<S>(bytes: &[u8], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&Zeroizing::new(bytes_to_base64url(bytes)))
}

/// Deserialize an unpadded base64url string into bytes.
pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
where
    D: Deserializer<'de>,
{
    let encoded = Zeroizing::new(String::deserialize(deserializer)?);
    base64url_to_bytes(&encoded).map_err(serde::de::Error::custom)
}
