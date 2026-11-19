// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import test from "node:test";
import { create } from "@bufbuild/protobuf";
import { ReallyMeCodecError, CodecErrorOrigin, CodecErrorReason } from "../dist/errors.js";
import { errorForCodecErrorMessage } from "../dist/classifyOperationError.js";
import { CodecErrorSchema } from "../dist/proto/generated/reallyme/codec/v1/codec_pb.js";

const wireError = (family, reason, origin) => create(CodecErrorSchema, {
  origin, error: { case: family, value: { reason } },
});

test("SDK errors retain the exact generated reason and origin in every legacy error family", () => {
  for (const [family, reason, origin, code] of [
    ["baseEncoding", CodecErrorReason.BASE_INVALID_BASE64, CodecErrorOrigin.CALLER, "invalid-input"],
    ["baseEncoding", CodecErrorReason.BASE_ALLOCATION_FAILURE, CodecErrorOrigin.PROVIDER, "provider-failure"],
    ["baseEncoding", CodecErrorReason.BASE_LENGTH_OVERFLOW, CodecErrorOrigin.PROVIDER, "provider-failure"],
    ["pem", CodecErrorReason.PEM_RESOURCE_LIMIT_EXCEEDED, CodecErrorOrigin.CALLER, "invalid-input"],
    ["multiformat", CodecErrorReason.MULTIFORMAT_UNKNOWN_MULTICODEC, CodecErrorOrigin.CALLER, "unsupported-codec"],
    ["canonicalization", CodecErrorReason.CANONICAL_NON_CANONICAL_JSON, CodecErrorOrigin.CALLER, "non-canonical"],
    ["canonicalization", CodecErrorReason.CANONICAL_UNSUPPORTED_IPLD_VALUE, CodecErrorOrigin.CALLER, "unsupported-ipld-value"],
    ["canonicalization", CodecErrorReason.CANONICAL_INTERNAL, CodecErrorOrigin.PROVIDER, "provider-failure"],
    ["canonicalization", CodecErrorReason.CANONICAL_JSON_ALLOCATION_FAILURE, CodecErrorOrigin.PROVIDER, "provider-failure"],
    ["canonicalization", CodecErrorReason.CANONICAL_JSON_ARITHMETIC_OVERFLOW, CodecErrorOrigin.PROVIDER, "provider-failure"],
    ["backend", CodecErrorReason.BACKEND_INTERNAL, CodecErrorOrigin.PROVIDER, "provider-failure"],
    ["boundary", CodecErrorReason.BOUNDARY_RESOURCE_LIMIT_EXCEEDED, CodecErrorOrigin.CALLER, "invalid-input"],
  ]) {
    const error = errorForCodecErrorMessage(wireError(family, reason, origin));
    assert.ok(error instanceof ReallyMeCodecError);
    assert.equal(error.code, code);
    assert.equal(error.message, code);
    assert.equal(error.reason, reason);
    assert.equal(error.origin, origin);
  }
});

test("malformed error envelopes retain no untrusted protobuf context", () => {
  for (const error of [
    wireError("baseEncoding", CodecErrorReason.BASE_ALLOCATION_FAILURE, CodecErrorOrigin.CALLER),
    wireError("baseEncoding", CodecErrorReason.BASE_LENGTH_OVERFLOW, CodecErrorOrigin.CALLER),
    wireError("boundary", 699, CodecErrorOrigin.CALLER),
    wireError("boundary", CodecErrorReason.BASE_INVALID_ENCODING, CodecErrorOrigin.CALLER),
    wireError("boundary", CodecErrorReason.BOUNDARY_MALFORMED_JSON, CodecErrorOrigin.PROVIDER),
    wireError("canonicalization", CodecErrorReason.CANONICAL_JSON_ALLOCATION_FAILURE, CodecErrorOrigin.CALLER),
    wireError("canonicalization", CodecErrorReason.CANONICAL_JSON_ARITHMETIC_OVERFLOW, CodecErrorOrigin.CALLER),
    wireError("boundary", CodecErrorReason.BOUNDARY_MALFORMED_JSON, 99),
    create(CodecErrorSchema),
  ]) {
    const mapped = errorForCodecErrorMessage(error);
    assert.equal(mapped.code, "provider-failure");
    assert.equal(mapped.reason, undefined);
    assert.equal(mapped.origin, undefined);
  }
});

test("local errors and invalid JavaScript enum arguments have no fabricated wire context", () => {
  for (const error of [
    new ReallyMeCodecError("invalid-input"),
    new ReallyMeCodecError("provider-failure"),
    new ReallyMeCodecError("invalid-input", 999999, CodecErrorOrigin.CALLER),
    new ReallyMeCodecError("invalid-input", CodecErrorReason.BASE_INVALID_BASE64, 99),
    new ReallyMeCodecError("invalid-input", CodecErrorReason.UNSPECIFIED, CodecErrorOrigin.CALLER),
  ]) {
    assert.equal(error.reason, undefined);
    assert.equal(error.origin, undefined);
  }
});
