// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::error::StorageErrorReason;
use super::SecretBytes;
use buffa::json_helpers::ProtoBytesJson;

// A zeroizing stack quartet writes directly into the selected serializer.
// The audited base64 policy does not create an immutable intermediate String.
struct Base64Text<'a>(&'a [u8]);

impl ::core::fmt::Display for Base64Text<'_> {
    fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        let mut encoded = ::zeroize::Zeroizing::new([0_u8; 4]);
        for chunk in self.0.chunks(3) {
            let written = ::base64::Engine::encode_slice(
                &::base64::engine::general_purpose::STANDARD,
                chunk,
                encoded.as_mut_slice(),
            )
            .map_err(|_| ::core::fmt::Error)?;
            let text =
                ::core::str::from_utf8(&encoded[..written]).map_err(|_| ::core::fmt::Error)?;
            formatter.write_str(text)?;
            ::zeroize::Zeroize::zeroize(encoded.as_mut_slice());
        }
        Ok(())
    }
}

const BASE64_CONFIG: ::base64::engine::general_purpose::GeneralPurposeConfig =
    ::base64::engine::general_purpose::GeneralPurposeConfig::new()
        .with_decode_allow_trailing_bits(false)
        .with_decode_padding_mode(::base64::engine::DecodePaddingMode::Indifferent);
const BASE64_STANDARD: ::base64::engine::general_purpose::GeneralPurpose =
    ::base64::engine::general_purpose::GeneralPurpose::new(
        &::base64::alphabet::STANDARD,
        BASE64_CONFIG,
    );
const BASE64_URL_SAFE: ::base64::engine::general_purpose::GeneralPurpose =
    ::base64::engine::general_purpose::GeneralPurpose::new(
        &::base64::alphabet::URL_SAFE,
        BASE64_CONFIG,
    );

fn decode_base64(
    value: &str,
) -> ::core::result::Result<::zeroize::Zeroizing<::buffa::alloc::vec::Vec<u8>>, StorageErrorReason>
{
    let groups = value
        .len()
        .checked_add(3)
        .ok_or(StorageErrorReason::LengthOverflow)?
        / 4;
    let capacity = groups
        .checked_mul(3)
        .ok_or(StorageErrorReason::LengthOverflow)?;
    let mut decoded = ::zeroize::Zeroizing::new(::buffa::alloc::vec::Vec::new());
    decoded
        .try_reserve_exact(capacity)
        .map_err(|_| StorageErrorReason::AllocationFailure)?;
    decoded.resize(capacity, 0);

    let written = match ::base64::Engine::decode_slice(
        &BASE64_STANDARD,
        value.as_bytes(),
        decoded.as_mut_slice(),
    ) {
        Ok(written) => written,
        Err(_) => {
            ::zeroize::Zeroize::zeroize(decoded.as_mut_slice());
            ::base64::Engine::decode_slice(
                &BASE64_URL_SAFE,
                value.as_bytes(),
                decoded.as_mut_slice(),
            )
            .map_err(|_| StorageErrorReason::InvalidEncoding)?
        }
    };
    decoded.truncate(written);
    Ok(decoded)
}

impl ProtoBytesJson for SecretBytes {
    fn serialize_json<S: serde::Serializer>(
        value: &[u8],
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer.collect_str(&Base64Text(value))
    }
    fn deserialize_json<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Self, D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = SecretBytes;
            fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                formatter.write_str("a base64 string")
            }
            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<Self::Value, E> {
                let mut bytes = decode_base64(value).map_err(E::custom)?;
                Ok(core::mem::take(&mut *bytes).into())
            }
        }
        deserializer.deserialize_str(Visitor)
    }
}
