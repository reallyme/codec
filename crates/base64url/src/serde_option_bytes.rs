// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use serde::{Deserialize, Deserializer, Serializer};

use crate::{base64url_to_bytes, bytes_to_base64url};

/// Serialize optional bytes as an unpadded base64url string or `null`.
pub fn serialize<S>(bytes: &Option<Vec<u8>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match bytes {
        Some(value) => serializer.serialize_some(&bytes_to_base64url(value)),
        None => serializer.serialize_none(),
    }
}

/// Deserialize an optional unpadded base64url string.
pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<u8>>, D::Error>
where
    D: Deserializer<'de>,
{
    let encoded = Option::<String>::deserialize(deserializer)?;
    match encoded {
        Some(value) => base64url_to_bytes(&value)
            .map(Some)
            .map_err(serde::de::Error::custom),
        None => Ok(None),
    }
}
