// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(clippy::unwrap_used)]

use super::contract_core::{decode_pem, encode_dag_cbor_value, verify_dag_cbor_cid};
use super::{
    dag_cbor_verify_cid_result_proto, deterministic_cbor_wire_error, internal_wire_error,
    multicodec_boundary_error, multicodec_table_result_proto, operation_response_from_result,
    pem_decode_result_proto, preflight_binary_request, process_operation_response,
    process_operation_response_json, INTERNAL_ERROR_OPERATION_RESPONSE,
};
use crate::multicodec::{
    supported_table, CodecTag, KeyMaterialKind, MulticodecLength, MulticodecOperationError,
};
use buffa::EnumValue;
use codec_cbor::{CborValue, DeterministicCborError, MAX_DETERMINISTIC_CBOR_NESTING_DEPTH};
use codec_pem::{PemDecodePolicy, PemLabel};
use codec_proto::generated::proto::reallyme::codec::v1::{
    __buffa::oneof::codec_operation_request::Operation as CodecOperation, CodecDagCborDecodeResult,
    CodecDagCborEncodeResult, CodecDagCborVerifyCidResult, CodecDeterministicCborBytes,
    CodecDeterministicCborDecodeResult, CodecDeterministicCborEncodeResult,
    CodecDeterministicCborInteger, CodecDeterministicCborMap, CodecDeterministicCborMapEntry,
    CodecDeterministicCborMapKey, CodecDeterministicCborNegativeInteger,
    CodecDeterministicCborNull, CodecDeterministicCborText, CodecDeterministicCborUnsignedInteger,
    CodecDeterministicCborValue, CodecErrorReason, CodecKeyMaterialKind,
    CodecMulticodecLookupResult, CodecMulticodecSpec, CodecMultikeyParseResult,
    CodecOperationRequest, CodecOperationResponse, CodecPemDecodeOptions, CodecPemDecodeResult,
    CodecPemEncodeOptions, CodecPemLabel, CodecPemLineEnding, CodecTag as ProtoCodecTag,
};
use codec_proto::{decode_protobuf, encode_protobuf, CodecWireError, CodecWireErrorBranch};
use zeroize::Zeroizing;

include!("tests/support.rs");
include!("tests/dispatch_multiformat.rs");
include!("tests/multikey_documents_pem.rs");
include!("tests/deterministic_cbor.rs");
include!("tests/helpers.rs");
