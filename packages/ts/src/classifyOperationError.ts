// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import type { ReallyMeCodecErrorCode } from "./errors.js";
import { ReallyMeCodecError } from "./errors.js";
import { CodecErrorOrigin, CodecErrorReason } from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import type { CodecError } from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import { hasUnknownFields } from "./validateGeneratedCbor.js";

export const errorForCodecErrorMessage = (
  error: CodecError,
): ReallyMeCodecError => {
  if (
    hasUnknownFields(error) ||
    (error.error.value !== undefined && hasUnknownFields(error.error.value))
  ) {
    return new ReallyMeCodecError("provider-failure");
  }
  const reason = error.error.value?.reason;
  // Protobuf enums are open on the wire; generated decoders can preserve
  // numeric values that this SDK version does not understand. Treat those as
  // provider failures so unknown future reasons never downgrade to caller
  // input errors.
  const reasonIsKnown =
    reason !== undefined &&
    Object.values(CodecErrorReason).some((candidate) => candidate === reason);
  if (!reasonIsKnown) {
    return new ReallyMeCodecError("provider-failure");
  }
  let expectedOrigin: CodecErrorOrigin;
  switch (error.error.case) {
    case "baseEncoding":
      if (reason < 100 || reason > 199) return new ReallyMeCodecError("provider-failure");
      expectedOrigin = CodecErrorOrigin.CALLER;
      break;
    case "pem":
      if (reason < 200 || reason > 299) return new ReallyMeCodecError("provider-failure");
      expectedOrigin = CodecErrorOrigin.CALLER;
      break;
    case "multiformat":
      if (reason < 300 || reason > 399) return new ReallyMeCodecError("provider-failure");
      expectedOrigin = CodecErrorOrigin.CALLER;
      break;
    case "canonicalization":
      if (reason < 400 || reason > 499) return new ReallyMeCodecError("provider-failure");
      expectedOrigin = reason === CodecErrorReason.CANONICAL_INTERNAL
        ? CodecErrorOrigin.PROVIDER
        : CodecErrorOrigin.CALLER;
      break;
    case "backend":
      if (reason < 500 || reason > 599) return new ReallyMeCodecError("provider-failure");
      expectedOrigin = CodecErrorOrigin.PROVIDER;
      break;
    case "boundary":
      if (reason < 600 || reason > 699) return new ReallyMeCodecError("provider-failure");
      expectedOrigin = CodecErrorOrigin.CALLER;
      break;
    case undefined:
      return new ReallyMeCodecError("provider-failure");
  }
  if (error.origin !== expectedOrigin) {
    return new ReallyMeCodecError("provider-failure");
  }
  if (error.origin !== CodecErrorOrigin.CALLER) {
    return new ReallyMeCodecError("provider-failure", reason, error.origin);
  }
  switch (reason) {
    case CodecErrorReason.MULTIFORMAT_UNKNOWN_MULTICODEC:
    case CodecErrorReason.MULTIFORMAT_INVALID_MULTICODEC_PREFIX:
      return new ReallyMeCodecError("unsupported-codec", reason, error.origin);
    case CodecErrorReason.CANONICAL_UNSUPPORTED_IPLD_VALUE:
      return new ReallyMeCodecError("unsupported-ipld-value", reason, error.origin);
    case CodecErrorReason.BASE_NON_CANONICAL_HEX:
    case CodecErrorReason.CANONICAL_NON_CANONICAL_CBOR:
    case CodecErrorReason.CANONICAL_NON_CANONICAL_JSON:
    case CodecErrorReason.CANONICAL_NON_MINIMAL_CBOR_INTEGER:
    case CodecErrorReason.CANONICAL_DUPLICATE_CBOR_MAP_KEY:
    case CodecErrorReason.CANONICAL_CBOR_MAP_KEYS_OUT_OF_ORDER:
    case CodecErrorReason.CANONICAL_CBOR_TRAILING_BYTES:
      return new ReallyMeCodecError("non-canonical", reason, error.origin);
    default:
      return new ReallyMeCodecError("invalid-input", reason, error.origin);
  }
};

export const errorCodeForCodecErrorMessage = (error: CodecError): ReallyMeCodecErrorCode =>
  errorForCodecErrorMessage(error).code;
