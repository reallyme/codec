// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Generated protobuf operation contract lane.
//!
//! Native Rust callers should continue to use the typed codec modules. This
//! contract exists for FFI, WASM, generated SDK, CLI, and transport boundaries
//! that need one self-describing [`CodecOperationRequest`] and one fully
//! discriminated binary [`CodecOperationResponse`].
//!
//! [`CodecOperationRequest`]: codec_proto::generated::proto::reallyme::codec::v1::CodecOperationRequest
//! [`CodecOperationResponse`]: codec_proto::generated::proto::reallyme::codec::v1::CodecOperationResponse

#![cfg_attr(test, allow(clippy::unwrap_used))]

#[path = "core/mod.rs"]
mod contract_core;

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
use codec_pem::{PemDecodePolicy, PemEncodeOptions, PemLabel, PemLineEnding};
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
    CodecWireErrorBranch,
};
use zeroize::Zeroizing;

include!("dispatch.rs");
include!("execute_multiformat.rs");
include!("execute_documents.rs");
include!("validate_documents.rs");
include!("decode_documents.rs");
include!("encode_documents.rs");
include!("copy_limits.rs");
include!("shape_results.rs");
include!("map_errors.rs");

#[cfg(test)]
include!("tests/support.rs");
#[cfg(test)]
include!("tests/dispatch_multiformat.rs");
#[cfg(test)]
include!("tests/multikey_documents_pem.rs");
#[cfg(test)]
include!("tests/deterministic_cbor.rs");
#[cfg(test)]
include!("tests/helpers.rs");
