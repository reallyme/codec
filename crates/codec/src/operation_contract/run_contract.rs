// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Generated protobuf operation contract lane.
//!
//! Native Rust callers should continue to use the typed codec modules. This
//! contract exists for FFI, WASM, generated SDK, CLI, and transport boundaries
//! that use the expanded generated operation envelope. The original
//! [`CodecOperationRequest`] and [`CodecOperationResponse`] remain wire-compatible
//! facades with their existing Rust source shapes.
//!
//! [`CodecOperationRequest`]: codec_proto::generated::proto::reallyme::codec::v1::CodecOperationRequest
//! [`CodecOperationResponse`]: codec_proto::generated::proto::reallyme::codec::v1::CodecOperationResponse

#![cfg_attr(test, allow(clippy::unwrap_used))]

#[path = "core/mod.rs"]
mod contract_core;

#[path = "dispatch_api.rs"]
mod dispatch_api;
#[path = "execute_der.rs"]
mod execute_der;
#[path = "execute_general_cbor.rs"]
mod execute_general_cbor;
#[path = "execute_tlv.rs"]
mod execute_tlv;
use codec_proto::generated::proto::reallyme::codec::v1::CodecApiOperationRequest as ApiRequest;
use dispatch_api::{api_response_from_result, encode_api_operation_response, execute_api_request};

#[path = "execute_scalars.rs"]
mod execute_scalars;
#[path = "inspect_general_cbor.rs"]
mod inspect_general_cbor;
/// Borrowed compatibility entry points shared by generated dispatch and SDK boundaries.
#[doc(hidden)]
#[path = "execute_scalar_native.rs"]
pub mod scalar_ops;
#[path = "shape_general_cbor.rs"]
mod shape_general_cbor;

use self::contract_core::{
    decode_dag_cbor_value, decode_deterministic_cbor_value, decode_pem, encode_dag_cbor_value,
    encode_deterministic_cbor_value, encode_pem, parse_multikey, verify_dag_cbor_cid,
    DagCborCidVerification, DagCborOperationError, DecodedPem, EncodedPem, MultikeyOperationError,
    PemOperationError, SemanticParsedMultikey,
};
use crate::multicodec::{
    lookup_prefix, prefix_for_name, supported_table, CodecTag, KeyMaterialKind, MulticodecLength,
    MulticodecLookup, MulticodecOperationError, MulticodecSpec, MulticodecTable,
};
use buffa::EnumValue;
use codec_cbor::{
    CborError, CborValue, DeterministicCborError, DeterministicCborInteger,
    DeterministicCborMapEntry, DeterministicCborMapKey, DeterministicCborValue,
    MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
    MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES, MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES,
    MAX_DETERMINISTIC_CBOR_NESTING_DEPTH, MAX_DETERMINISTIC_CBOR_NODES,
};
use codec_pem::{
    preflight_pem_encoded_length, PemDecodePolicy, PemEncodeOptions, PemLabel, PemLineEnding,
};
use codec_proto::generated::proto::reallyme::codec::v1::{
    __buffa::oneof::codec_operation_request::Operation as CodecOperation,
    codec_deterministic_cbor_integer, codec_deterministic_cbor_map_key,
    codec_deterministic_cbor_value, CodecDagCborDecodeResult, CodecDagCborEncodeResult,
    CodecDagCborVerifyCidResult, CodecDeterministicCborArray, CodecDeterministicCborBool,
    CodecDeterministicCborBytes, CodecDeterministicCborDecodeResult,
    CodecDeterministicCborEncodeResult, CodecDeterministicCborInteger, CodecDeterministicCborMap,
    CodecDeterministicCborMapEntry, CodecDeterministicCborMapKey,
    CodecDeterministicCborNegativeInteger, CodecDeterministicCborNull, CodecDeterministicCborText,
    CodecDeterministicCborUnsignedInteger, CodecDeterministicCborValue, CodecErrorReason,
    CodecKeyMaterialKind, CodecMulticodecLookupResult, CodecMulticodecSpec,
    CodecMulticodecTableResult, CodecMultikeyParseResult, CodecOperationRequest,
    CodecOperationResponse, CodecOperationResult, CodecPemDecodeOptions, CodecPemDecodeResult,
    CodecPemEncodeOptions, CodecPemEncodeResult, CodecPemLabel, CodecPemLineEnding,
    CodecTag as ProtoCodecTag,
};
use codec_proto::{
    codec_error, decode_json, decode_protobuf, encode_protobuf, CodecWireError,
    CodecWireErrorBranch, MAX_CODEC_PROTO_ERROR_ENVELOPE_BYTES, MAX_CODEC_PROTO_MESSAGE_BYTES,
};
use zeroize::Zeroizing;

include!("dispatch.rs");
include!("preflight_pem_decode.rs");
include!("execute_multiformat.rs");
include!("execute_documents.rs");
include!("validate_documents.rs");
include!("decode_documents.rs");
include!("encode_documents.rs");
include!("copy_limits.rs");
include!("shape_results.rs");
include!("map_errors.rs");

#[cfg(test)]
#[path = "contract_tests.rs"]
mod tests;
