// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

/// Convert semantic multicodec metadata into the generated result contract.
///
/// Direct scalar adapters use this hidden helper so generated ProtoJSON and
/// binary protobuf share one result-shaping authority.
pub fn codec_spec_proto(spec: &MulticodecSpec<'_>) -> Result<CodecMulticodecSpec, CodecWireError> {
    let (fixed_length, variable_length) = match spec.length() {
        MulticodecLength::Fixed(length) => (usize_to_u32(length)?, false),
        MulticodecLength::Variable => (0, true),
        MulticodecLength::NotApplicable => (0, false),
    };
    Ok(CodecMulticodecSpec {
        name: try_copy_result_string(spec.name())?,
        code: try_copy_result_bytes(spec.code())?,
        prefix: try_copy_result_bytes(spec.prefix())?,
        tag: EnumValue::from(codec_tag_proto(spec.tag())?),
        key_material_kind: EnumValue::from(key_material_kind_proto(spec.key_material())?),
        fixed_length,
        variable_length,
        algorithm_name: try_copy_result_string(spec.algorithm_name())?,
        __buffa_unknown_fields: Default::default(),
    })
}

/// Convert a semantic multicodec lookup into the generated result contract.
#[doc(hidden)]
pub fn multicodec_lookup_result_proto(
    found: &MulticodecLookup<'_>,
) -> Result<CodecMulticodecLookupResult, CodecWireError> {
    Ok(CodecMulticodecLookupResult {
        name: try_copy_result_string(found.name())?,
        prefix_length: usize_to_u32(found.prefix_length())?,
        metadata: codec_spec_proto(found.metadata())?.into(),
        __buffa_unknown_fields: Default::default(),
    })
}

/// Convert the semantic multicodec table into the generated result contract.
#[doc(hidden)]
pub fn multicodec_table_result_proto(
    table: &MulticodecTable<'_>,
) -> Result<CodecMulticodecTableResult, CodecWireError> {
    let mut entries = Vec::new();
    entries
        .try_reserve(table.entries().len())
        .map_err(|_| internal_wire_error())?;
    for spec in table.entries() {
        entries.push(codec_spec_proto(spec)?);
    }
    Ok(CodecMulticodecTableResult {
        entries,
        __buffa_unknown_fields: Default::default(),
    })
}

/// Convert a semantic multikey parse result into the generated result contract.
#[doc(hidden)]
pub fn multikey_parse_result_proto(
    parsed: SemanticParsedMultikey,
) -> Result<CodecMultikeyParseResult, CodecWireError> {
    let variable_public_key_length = parsed.variable_public_key_length();
    let expected_public_key_length = match parsed.expected_public_key_length() {
        Some(length) => usize_to_u32(length)?,
        None => 0,
    };
    Ok(CodecMultikeyParseResult {
        codec_name: try_copy_result_string(parsed.codec_name())?,
        algorithm_name: try_copy_result_string(parsed.algorithm_name())?,
        public_key: parsed.into_public_key(),
        expected_public_key_length,
        variable_public_key_length,
        __buffa_unknown_fields: Default::default(),
    })
}

/// Convert a semantic DAG-CBOR CID verification into the generated contract.
#[doc(hidden)]
#[must_use]
pub fn dag_cbor_verify_cid_result_proto(
    verification: DagCborCidVerification,
) -> CodecDagCborVerifyCidResult {
    let (valid, expected_cid, actual_cid) = verification.into_parts();
    CodecDagCborVerifyCidResult {
        valid,
        expected_cid,
        actual_cid,
        __buffa_unknown_fields: Default::default(),
    }
}

/// Convert a semantic PEM decode result into the generated result contract.
#[doc(hidden)]
pub fn pem_decode_result_proto(
    decoded: DecodedPem,
) -> Result<CodecPemDecodeResult, CodecWireError> {
    // Allocate the non-secret label before transferring DER. If allocation
    // fails, `decoded` remains the zeroizing owner and wipes the secret bytes.
    let label = try_copy_result_string(decoded.label().as_str())?;
    let der = decoded.into_der();
    Ok(CodecPemDecodeResult {
        label,
        der,
        __buffa_unknown_fields: Default::default(),
    })
}

fn try_copy_result_bytes(value: &[u8]) -> Result<Vec<u8>, CodecWireError> {
    let mut copy = Vec::new();
    copy.try_reserve_exact(value.len())
        .map_err(|_| internal_wire_error())?;
    copy.extend_from_slice(value);
    Ok(copy)
}

fn try_copy_result_string(value: &str) -> Result<String, CodecWireError> {
    let mut copy = String::new();
    copy.try_reserve_exact(value.len())
        .map_err(|_| internal_wire_error())?;
    copy.push_str(value);
    Ok(copy)
}

fn codec_tag_proto(tag: CodecTag) -> Result<ProtoCodecTag, CodecWireError> {
    let tag = match tag {
        CodecTag::Encryption => ProtoCodecTag::CODEC_TAG_ENCRYPTION,
        CodecTag::Key => ProtoCodecTag::CODEC_TAG_KEY,
        CodecTag::Hash => ProtoCodecTag::CODEC_TAG_HASH,
        CodecTag::Multihash => ProtoCodecTag::CODEC_TAG_MULTIHASH,
        CodecTag::Multikey => ProtoCodecTag::CODEC_TAG_MULTIKEY,
    };
    Ok(tag)
}

fn key_material_kind_proto(kind: KeyMaterialKind) -> Result<CodecKeyMaterialKind, CodecWireError> {
    let kind = match kind {
        KeyMaterialKind::PublicKey => CodecKeyMaterialKind::CODEC_KEY_MATERIAL_KIND_PUBLIC_KEY,
        KeyMaterialKind::PrivateKey => CodecKeyMaterialKind::CODEC_KEY_MATERIAL_KIND_PRIVATE_KEY,
        KeyMaterialKind::SymmetricKey => {
            CodecKeyMaterialKind::CODEC_KEY_MATERIAL_KIND_SYMMETRIC_KEY
        }
        KeyMaterialKind::NotKey => CodecKeyMaterialKind::CODEC_KEY_MATERIAL_KIND_NOT_KEY,
    };
    Ok(kind)
}

fn usize_to_u32(value: usize) -> Result<u32, CodecWireError> {
    u32::try_from(value).map_err(|_| {
        wire_error(
            CodecWireErrorBranch::Backend,
            CodecErrorReason::CODEC_ERROR_REASON_BACKEND_INTERNAL,
        )
    })
}
