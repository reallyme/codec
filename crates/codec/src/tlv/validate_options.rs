// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{ErrorReason, Options, Profile, TlvError};

/// Hard ceiling on actual source bytes, including any unconsumed suffix.
pub use codec_proto::MAX_CODEC_TLV_INPUT_BYTES as MAX_INPUT_BYTES;
/// Default source limit; larger inputs require an explicit caller limit.
pub const DEFAULT_INPUT_BYTES: usize = 1_048_576;
/// Hard ceiling on a complete encoded identifier, without numeric reinterpretation.
pub const MAX_IDENTIFIER_OCTETS: usize = 32;
/// Hard ceiling on long-form length payload octets, representing a uint64.
pub const MAX_LENGTH_OCTETS: usize = 8;
/// Default identifier limit, including its initial octet.
pub const DEFAULT_IDENTIFIER_OCTETS: usize = 4;
/// Default long-form length payload limit, excluding its discriminator.
pub const DEFAULT_LENGTH_OCTETS: usize = 4;

pub(super) struct ValidatedOptions {
    pub(super) der: bool,
    input_bytes: usize,
    pub(super) identifier_octets: usize,
    pub(super) length_octets: usize,
}

impl ValidatedOptions {
    pub(super) fn for_input(input: &[u8], options: &Options) -> Result<Self, TlvError> {
        let options = Self::try_from(options)?;
        if input.len() > options.input_bytes {
            return Err(TlvError::new(ErrorReason::InputLimit));
        }
        Ok(options)
    }
}

fn limit(value: Option<u64>, default: usize, maximum: usize) -> Result<usize, TlvError> {
    let value = match value {
        None => default,
        Some(value) => {
            usize::try_from(value).map_err(|_| TlvError::new(ErrorReason::InvalidLimits))?
        }
    };
    if value == 0 || value > maximum {
        return Err(TlvError::new(ErrorReason::InvalidLimits));
    }
    Ok(value)
}

impl TryFrom<&Options> for ValidatedOptions {
    type Error = TlvError;

    fn try_from(options: &Options) -> Result<Self, Self::Error> {
        // This is the only policy conversion. Unknown or omitted profiles never
        // silently acquire BER's more permissive identifier/length rules.
        let der = match options.profile.as_known() {
            Some(Profile::CODEC_TLV_PROFILE_ISO7816_DEFINITE_BER) => false,
            Some(Profile::CODEC_TLV_PROFILE_ASN1_DER_FRAMING) => true,
            _ => return Err(TlvError::new(ErrorReason::InvalidProfile)),
        };
        let limits = options.limits.as_option();
        Ok(ValidatedOptions {
            der,
            input_bytes: limit(
                limits.and_then(|value| value.input_bytes),
                DEFAULT_INPUT_BYTES,
                MAX_INPUT_BYTES,
            )?,
            identifier_octets: limit(
                limits.and_then(|value| value.identifier_octets),
                DEFAULT_IDENTIFIER_OCTETS,
                MAX_IDENTIFIER_OCTETS,
            )?,
            length_octets: limit(
                limits.and_then(|value| value.length_octets),
                DEFAULT_LENGTH_OCTETS,
                MAX_LENGTH_OCTETS,
            )?,
        })
    }
}
