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
  classifiedField("CodecProtoResultEnvelope", "payload", "bytes", "sensitive"),
  classifiedField("CodecMulticodecPrefixForNameRequest", "name", "string", "public"),
  classifiedField("CodecMulticodecLookupPrefixRequest", "value", "bytes", "sensitive"),
  classifiedField("CodecMultikeyParseRequest", "multikey", "string", "sensitive"),
  classifiedField("CodecDagCborVerifyCidRequest", "cid", "string", "public"),
  classifiedField("CodecDagCborVerifyCidRequest", "payload", "bytes", "sensitive"),
  classifiedField("CodecPemDecodeRequest", "pem", "bytes", "sensitive"),
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

// Owners of sensitive nested oneof messages have no direct scalar field to
// classify, but still require redaction, unknown-field wiping, and Drop logic.
export const codecProtoSensitiveOwnerMessages = Object.freeze([
  "CodecOperationRequest",
]);
