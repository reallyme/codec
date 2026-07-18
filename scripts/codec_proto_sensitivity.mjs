// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

// This manifest is the single source of truth for protobuf scalar sensitivity.
// Every schema `bytes` and `string` field must appear exactly once. Generation
// and release-readiness independently compare this closed list with the schema,
// so a newly added field cannot silently inherit unsafe generated behavior.
const classifiedField = (message, field, kind, sensitivity) => Object.freeze({
  message,
  field,
  kind,
  sensitivity,
});

export const codecProtoScalarFieldClassifications = Object.freeze([
  classifiedField("CodecMulticodecPrefixForNameRequest", "name", "string", "public"),
  classifiedField("CodecMulticodecLookupPrefixRequest", "value", "bytes", "sensitive"),
  classifiedField("CodecMultikeyParseRequest", "multikey", "string", "sensitive"),
  classifiedField("CodecDagCborVerifyCidRequest", "cid", "string", "public"),
  classifiedField("CodecDagCborVerifyCidRequest", "payload", "bytes", "sensitive"),
  classifiedField("CodecDagCborEncodeResult", "encoded", "bytes", "sensitive"),
  classifiedField("CodecDagCborDecodeRequest", "encoded", "bytes", "sensitive"),
  classifiedField("CodecPemDecodeRequest", "pem", "bytes", "sensitive"),
  classifiedField("CodecPemEncodeRequest", "der", "bytes", "sensitive"),
  classifiedField("CodecPemEncodeResult", "pem", "bytes", "sensitive"),
  classifiedField("CodecDeterministicCborText", "value", "string", "sensitive"),
  classifiedField("CodecDeterministicCborBytes", "value", "bytes", "sensitive"),
  classifiedField("CodecDeterministicCborEncodeResult", "encoded", "bytes", "sensitive"),
  classifiedField("CodecDeterministicCborDecodeRequest", "encoded", "bytes", "sensitive"),
  classifiedField("CodecMulticodecSpec", "name", "string", "public"),
  classifiedField("CodecMulticodecSpec", "code", "bytes", "public"),
  classifiedField("CodecMulticodecSpec", "prefix", "bytes", "public"),
  classifiedField("CodecMulticodecSpec", "algorithm_name", "string", "public"),
  classifiedField("CodecMulticodecLookupResult", "name", "string", "public"),
  classifiedField("CodecMultikeyParseResult", "codec_name", "string", "public"),
  classifiedField("CodecMultikeyParseResult", "algorithm_name", "string", "public"),
  classifiedField("CodecMultikeyParseResult", "public_key", "bytes", "sensitive"),
  classifiedField("CodecDagCborVerifyCidResult", "expected_cid", "string", "public"),
  classifiedField("CodecDagCborVerifyCidResult", "actual_cid", "string", "public"),
  classifiedField("CodecPemDecodeResult", "label", "string", "public"),
  classifiedField("CodecPemDecodeResult", "der", "bytes", "sensitive"),
]);

// Primitive and repeated-message fields can also carry identity data even
// though they are outside the closed bytes/string inventory above. Keeping
// these classifications explicit makes generated clear and Drop hardening
// fail closed when the deterministic-CBOR schema evolves.
export const codecProtoSensitiveNonTextFieldClassifications = Object.freeze([
  classifiedField("CodecDeterministicCborBool", "value", "bool", "sensitive"),
  classifiedField(
    "CodecDeterministicCborUnsignedInteger",
    "value",
    "uint64",
    "sensitive",
  ),
  classifiedField(
    "CodecDeterministicCborNegativeInteger",
    "value",
    "sint64",
    "sensitive",
  ),
  classifiedField(
    "CodecDeterministicCborArray",
    "values",
    "repeated-message",
    "sensitive",
  ),
  classifiedField(
    "CodecDeterministicCborMap",
    "entries",
    "repeated-message",
    "sensitive",
  ),
  classifiedField(
    "CodecDeterministicCborMapEntry",
    "key",
    "message-field",
    "sensitive",
  ),
  classifiedField(
    "CodecDeterministicCborMapEntry",
    "value",
    "message-field",
    "sensitive",
  ),
  classifiedField(
    "CodecDeterministicCborEncodeRequest",
    "value",
    "message-field",
    "sensitive",
  ),
  classifiedField(
    "CodecDeterministicCborDecodeResult",
    "value",
    "message-field",
    "sensitive",
  ),
  classifiedField(
    "CodecDagCborEncodeRequest",
    "value",
    "message-field",
    "sensitive",
  ),
  classifiedField(
    "CodecDagCborDecodeResult",
    "value",
    "message-field",
    "sensitive",
  ),
]);

// Owners of sensitive nested messages have no direct scalar field to classify,
// but still require redaction and zeroizing generated clear paths. Oneof owners
// that use generated manual Deserialize also receive compatible Drop logic.
export const codecProtoSensitiveOwnerMessages = Object.freeze([
  "CodecOperationRequest",
  "CodecOperationResult",
  "CodecOperationResponse",
  "CodecDeterministicCborNull",
  "CodecDeterministicCborBool",
  "CodecDeterministicCborUnsignedInteger",
  "CodecDeterministicCborNegativeInteger",
  "CodecDeterministicCborInteger",
  "CodecDeterministicCborMapEntry",
  "CodecDeterministicCborMapKey",
  "CodecDeterministicCborArray",
  "CodecDeterministicCborMap",
  "CodecDeterministicCborValue",
  "CodecDeterministicCborEncodeRequest",
  "CodecDeterministicCborDecodeResult",
  "CodecDagCborEncodeRequest",
  "CodecDagCborEncodeResult",
  "CodecDagCborDecodeRequest",
  "CodecDagCborDecodeResult",
  "CodecPemEncodeRequest",
  "CodecPemEncodeResult",
]);

// Java Lite does not expose unknown-field storage. SDK adapters need a
// content-free generated predicate for every provider result they decode so
// schema skew cannot be silently accepted while mapping into public types.
// This is intentionally independent of sensitivity: public metadata still
// requires fail-closed provider validation.
export const codecProtoProviderOutputMessages = Object.freeze([
  "CodecError",
  "CodecBaseEncodingError",
  "CodecPemError",
  "CodecMultiformatError",
  "CodecCanonicalizationError",
  "CodecBackendError",
  "CodecBoundaryError",
  "CodecOperationResult",
  "CodecOperationResponse",
  "CodecMulticodecSpec",
  "CodecMulticodecLookupResult",
  "CodecMulticodecTableResult",
  "CodecMultikeyParseResult",
  "CodecDagCborVerifyCidResult",
  "CodecPemDecodeResult",
  "CodecPemEncodeResult",
  "CodecDeterministicCborEncodeResult",
  "CodecDeterministicCborDecodeResult",
  "CodecDagCborEncodeResult",
  "CodecDagCborDecodeResult",
]);
