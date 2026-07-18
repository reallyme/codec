// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

fn try_deterministic_vec<T>(capacity: usize) -> Result<Vec<T>, CodecWireError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(capacity)
        .map_err(|_| internal_wire_error())?;
    Ok(values)
}

fn try_copy_deterministic_bytes(value: &[u8]) -> Result<Vec<u8>, CodecWireError> {
    let mut copy = try_deterministic_vec(value.len())?;
    copy.extend_from_slice(value);
    Ok(copy)
}

fn try_copy_deterministic_text(value: &str) -> Result<String, CodecWireError> {
    let mut copy = String::new();
    copy.try_reserve_exact(value.len())
        .map_err(|_| internal_wire_error())?;
    copy.push_str(value);
    Ok(copy)
}

fn option_limit(value: u32, default: usize) -> Result<usize, CodecWireError> {
    if value == 0 {
        return Ok(default);
    }
    usize::try_from(value).map_err(|_| {
        wire_error(
            CodecWireErrorBranch::Boundary,
            CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED,
        )
    })
}

fn pem_label(label: Option<CodecPemLabel>) -> Result<PemLabel, CodecWireError> {
    match label {
        Some(CodecPemLabel::CODEC_PEM_LABEL_PRIVATE_KEY) => Ok(PemLabel::PrivateKey),
        Some(CodecPemLabel::CODEC_PEM_LABEL_EC_PRIVATE_KEY) => Ok(PemLabel::EcPrivateKey),
        Some(CodecPemLabel::CODEC_PEM_LABEL_PUBLIC_KEY) => Ok(PemLabel::PublicKey),
        Some(CodecPemLabel::CODEC_PEM_LABEL_UNSPECIFIED) | None => Err(wire_error(
            CodecWireErrorBranch::Pem,
            CodecErrorReason::CODEC_ERROR_REASON_PEM_UNSUPPORTED_LABEL,
        )),
    }
}
