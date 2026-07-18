// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

fn process_multicodec_prefix_for_name(name: &str) -> Result<CodecMulticodecSpec, CodecWireError> {
    let spec = prefix_for_name(name).map_err(multicodec_boundary_error)?;
    codec_spec_proto(&spec)
}

fn process_multicodec_lookup_prefix(
    value: &[u8],
) -> Result<CodecMulticodecLookupResult, CodecWireError> {
    let found = lookup_prefix(value).map_err(multicodec_boundary_error)?;
    multicodec_lookup_result_proto(&found)
}

fn process_multicodec_table() -> Result<CodecMulticodecTableResult, CodecWireError> {
    let table = supported_table().map_err(multicodec_boundary_error)?;
    multicodec_table_result_proto(&table)
}

fn process_multikey_parse(multikey: &str) -> Result<CodecMultikeyParseResult, CodecWireError> {
    let parsed = parse_multikey(multikey).map_err(multikey_boundary_error)?;
    multikey_parse_result_proto(parsed)
}

fn process_dag_cbor_verify_cid(
    cid: &str,
    payload: &[u8],
) -> Result<CodecDagCborVerifyCidResult, CodecWireError> {
    let verification = verify_dag_cbor_cid(cid, payload).map_err(dag_cbor_boundary_error)?;
    Ok(dag_cbor_verify_cid_result_proto(verification))
}
