// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::encode::encode_generated;
use super::encode_contract::float;
use super::error::{error, CborError, ErrorReason};
use super::{EncodedCbor, Limits, Profile};
use codec_proto::generated::proto::reallyme::codec::v1 as proto;

impl TryFrom<&proto::CodecGeneralCborOptions> for Profile {
    type Error = CborError;
    fn try_from(value: &proto::CodecGeneralCborOptions) -> Result<Self, Self::Error> {
        let invalid = || error(ErrorReason::UnsupportedPolicy);
        Self::new(
            value.serialization.as_known().ok_or_else(invalid)?,
            value.lengths.as_known().ok_or_else(invalid)?,
            value.floats.as_known().ok_or_else(invalid)?,
            value.ordering.as_known().ok_or_else(invalid)?,
        )
    }
}

impl TryFrom<&proto::CodecGeneralCborOptions> for Limits {
    type Error = CborError;
    fn try_from(value: &proto::CodecGeneralCborOptions) -> Result<Self, Self::Error> {
        let mut limits = Self::default();
        // Absence and zero have different meanings; do not use numeric defaults
        // to represent optional wire presence.
        macro_rules! apply {
            ($field:ident, $setter:ident) => {
                if let Some(value) = value.$field {
                    limits = limits.$setter(
                        usize::try_from(value).map_err(|_| error(ErrorReason::InvalidLimit))?,
                    )?;
                }
            };
        }
        apply!(input_bytes, with_input_bytes);
        apply!(output_bytes, with_output_bytes);
        apply!(depth, with_depth);
        apply!(nodes, with_nodes);
        apply!(entries, with_entries);
        apply!(byte_string_bytes, with_byte_string_bytes);
        apply!(text_bytes, with_text_bytes);
        apply!(work, with_work);
        Ok(limits)
    }
}

/// Encode a generated value through the native bounded, zeroizing writer.
pub fn encode_value(
    value: &proto::CodecGeneralCborValue,
    profile: Profile,
    limits: Limits,
) -> Result<EncodedCbor, CborError> {
    encode_generated(value, profile, limits)
}

/// Inspect IEEE representation properties without converting NaN payloads to JSON numbers.
pub fn float_properties(
    value: &proto::CodecGeneralCborFloat,
) -> Result<proto::CodecGeneralCborFloatProperties, CborError> {
    let native = float(value)?;
    Ok(proto::CodecGeneralCborFloatProperties {
        value: Some(proto::CodecGeneralCborFloat {
            width: native.width().into(),
            bits: native.bits(),
        })
        .into(),
        shortest_width: native.shortest().into(),
        finite: native.is_finite(),
        nan: native.is_nan(),
        widened_binary64_bits: native.to_f64().to_bits(),
    })
}

/// Compare the basic-model float value while preserving signed-zero and NaN semantics.
pub fn float_equivalent(
    left: &proto::CodecGeneralCborFloat,
    right: &proto::CodecGeneralCborFloat,
) -> Result<bool, CborError> {
    Ok(float(left)?.equivalent(&float(right)?))
}
