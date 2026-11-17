// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::error::StorageErrorReason;
use super::SecretString;
use buffa::json_helpers::ProtoStringJson;
use zeroize::Zeroizing;

impl ProtoStringJson for SecretString {
    fn serialize_json<S: serde::Serializer>(text: &str, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(text)
    }

    fn deserialize_json<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = SecretString;
            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("a UTF-8 string")
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                let mut text = Zeroizing::new(String::new());
                text.try_reserve_exact(value.len())
                    .map_err(|_| E::custom(StorageErrorReason::AllocationFailure))?;
                text.push_str(value);
                Ok(core::mem::take(&mut *text).into())
            }
            fn visit_string<E: serde::de::Error>(self, value: String) -> Result<Self::Value, E> {
                // Adopt an unescaped parser-owned string without making a copy.
                // The parser's own scratch storage remains a transport boundary.
                Ok(value.into())
            }
            fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<Self::Value, E> {
                let _value = zeroize::Zeroizing::new(value);
                Err(E::custom(StorageErrorReason::InvalidJsonType))
            }
            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<Self::Value, E> {
                let _value = zeroize::Zeroizing::new(value);
                Err(E::custom(StorageErrorReason::InvalidJsonType))
            }
            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<Self::Value, E> {
                let _value = zeroize::Zeroizing::new(value);
                Err(E::custom(StorageErrorReason::InvalidJsonType))
            }
            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<Self::Value, E> {
                let _value = zeroize::Zeroizing::new(value);
                Err(E::custom(StorageErrorReason::InvalidJsonType))
            }
            fn visit_unit<E: serde::de::Error>(self) -> Result<Self::Value, E> {
                Err(E::custom(StorageErrorReason::InvalidJsonType))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                _: A,
            ) -> Result<Self::Value, A::Error> {
                Err(serde::de::Error::custom(
                    StorageErrorReason::InvalidJsonType,
                ))
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                _: M,
            ) -> Result<Self::Value, M::Error> {
                Err(serde::de::Error::custom(
                    StorageErrorReason::InvalidJsonType,
                ))
            }
            fn visit_byte_buf<E: serde::de::Error>(self, value: Vec<u8>) -> Result<Self::Value, E> {
                let _value = Zeroizing::new(value);
                Err(E::custom(StorageErrorReason::InvalidJsonType))
            }
        }
        // Visit wrong JSON scalar types before Serde can copy their received
        // values into an owned invalid-type diagnostic.
        deserializer.deserialize_any(Visitor)
    }
}
