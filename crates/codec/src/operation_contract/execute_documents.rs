// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

fn process_dag_cbor_encode<P: buffa::ProtoBox<CodecDeterministicCborValue>>(
    value: &buffa::MessageField<CodecDeterministicCborValue, P>,
) -> Result<CodecDagCborEncodeResult, CodecWireError> {
    let Some(proto_value) = value.as_option() else {
        return Err(malformed_request_wire_error());
    };
    let mut limits = DeterministicProtoLimits::default();
    validate_dag_cbor_proto_value(proto_value, 0, &mut limits)?;
    let value = dag_cbor_value_from_field(value)?;
    let encoded = encode_dag_cbor_value(&value).map_err(dag_cbor_cbor_wire_error)?;
    Ok(CodecDagCborEncodeResult {
        encoded: try_copy_deterministic_bytes(encoded.as_slice())?,
        __buffa_unknown_fields: Default::default(),
    })
}

fn process_dag_cbor_decode(encoded: &[u8]) -> Result<CodecDagCborDecodeResult, CodecWireError> {
    let value = decode_dag_cbor_value(encoded).map_err(dag_cbor_cbor_wire_error)?;
    Ok(CodecDagCborDecodeResult {
        value: buffa::MessageField::some(dag_cbor_value_proto(&value)?),
        __buffa_unknown_fields: Default::default(),
    })
}

fn process_pem_decode(
    pem: &[u8],
    options: Option<&CodecPemDecodeOptions>,
) -> Result<CodecPemDecodeResult, CodecWireError> {
    let input = core::str::from_utf8(pem).map_err(|_| {
        wire_error(
            CodecWireErrorBranch::Pem,
            CodecErrorReason::CODEC_ERROR_REASON_PEM_INVALID_BODY,
        )
    })?;
    let defaults = PemDecodePolicy::default();
    let mut labels = Vec::new();
    if let Some(options) = options {
        labels
            .try_reserve(options.allowed_labels.len())
            .map_err(|_| internal_wire_error())?;
        for label in &options.allowed_labels {
            labels.push(pem_label(label.as_known())?);
        }
    }
    let allowed_labels = if labels.is_empty() {
        defaults.allowed_labels
    } else {
        labels.as_slice()
    };
    let max_input_len = option_limit(
        options.map_or(0, |value| value.max_input_len),
        defaults.max_input_len,
    )?;
    let max_der_len = option_limit(
        options.map_or(0, |value| value.max_der_len),
        defaults.max_der_len,
    )?;
    let decoded = decode_pem(
        input,
        PemDecodePolicy {
            allowed_labels,
            max_input_len,
            max_der_len,
        },
    )
    .map_err(pem_boundary_error)?;
    pem_decode_result_proto(decoded)
}

fn process_pem_encode(
    label: Option<CodecPemLabel>,
    der: &[u8],
    options: Option<&CodecPemEncodeOptions>,
) -> Result<CodecPemEncodeResult, CodecWireError> {
    let defaults = PemEncodeOptions::default();
    let max_der_len = option_limit(
        options.map_or(0, |value| value.max_der_len),
        defaults.max_der_len,
    )?;
    let line_width = option_limit(
        options.map_or(0, |value| value.line_width),
        defaults.line_width,
    )?;
    let line_ending = match options.and_then(|value| value.line_ending.as_known()) {
        None | Some(CodecPemLineEnding::CODEC_PEM_LINE_ENDING_UNSPECIFIED) => defaults.line_ending,
        Some(CodecPemLineEnding::CODEC_PEM_LINE_ENDING_LF) => PemLineEnding::Lf,
        Some(CodecPemLineEnding::CODEC_PEM_LINE_ENDING_CRLF) => PemLineEnding::Crlf,
    };
    let encoded = encode_pem(
        pem_label(label)?,
        der,
        PemEncodeOptions {
            max_der_len,
            line_width,
            line_ending,
        },
    )
    .map_err(pem_boundary_error)?;
    Ok(pem_encode_result_proto(encoded))
}

fn pem_encode_result_proto(encoded: EncodedPem) -> CodecPemEncodeResult {
    CodecPemEncodeResult {
        pem: encoded.into_bytes(),
        __buffa_unknown_fields: Default::default(),
    }
}

fn process_deterministic_cbor_encode<P: buffa::ProtoBox<CodecDeterministicCborValue>>(
    value: &buffa::MessageField<CodecDeterministicCborValue, P>,
) -> Result<CodecDeterministicCborEncodeResult, CodecWireError> {
    let Some(proto_value) = value.as_option() else {
        return Err(malformed_request_wire_error());
    };
    let mut limits = DeterministicProtoLimits::default();
    validate_deterministic_value(proto_value, 0, &mut limits)?;
    let value = deterministic_value_from_field(value)?;
    let encoded = encode_deterministic_cbor_value(&value).map_err(deterministic_cbor_wire_error)?;
    Ok(CodecDeterministicCborEncodeResult {
        encoded: try_copy_deterministic_bytes(encoded.as_slice())?,
        __buffa_unknown_fields: Default::default(),
    })
}
