// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::error::{error, CborError, ErrorReason};
use super::float::{Float, FloatWidth};
use codec_proto::generated::proto::reallyme::codec::v1 as proto;

/// Convert only representable IEEE widths; high bits are never truncated.
pub(super) fn float(value: &proto::CodecGeneralCborFloat) -> Result<Float, CborError> {
    match value.width.as_known() {
        Some(FloatWidth::Binary16) => Ok(Float::binary16(
            u16::try_from(value.bits).map_err(|_| error(ErrorReason::WrongType))?,
        )),
        Some(FloatWidth::Binary32) => Ok(Float::binary32(
            u32::try_from(value.bits).map_err(|_| error(ErrorReason::WrongType))?,
        )),
        Some(FloatWidth::Binary64) => Ok(Float::binary64(value.bits)),
        _ => Err(error(ErrorReason::UnsupportedPolicy)),
    }
}
