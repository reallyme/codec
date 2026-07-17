// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { create, fromBinary, toBinary } from "@bufbuild/protobuf";

import {
  MAX_CODEC_PROTO_JSON_BYTES,
  MAX_CODEC_PROTO_MESSAGE_BYTES,
  requireBoundaryAggregate,
  utf8ByteLength,
} from "./boundary.js";
import { ReallyMeCodecError } from "./errors.js";
import type { ReallyMeCodecErrorCode } from "./errors.js";
import {
  CodecBoundaryErrorSchema,
  CodecErrorReason,
  CodecErrorSchema,
  CodecOperationRequestSchema,
  CodecProtoResultEnvelopeSchema,
  CodecProtoResultStatus,
} from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import type { CodecOperationRequest } from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import {
  ensureBytesInput,
  readIndependentBoundedBytesOutput,
} from "./readOutput.js";
import type { ReallyMeCodecProtoResult } from "./readOutput.js";
import { requireReallyMeCodecWasmProvider } from "./wasmProvider.js";

const MAX_CODEC_PROTO_RESULT_ENVELOPE_BYTES = 1_048_592;
const PROTO_REQUEST_OVERHEAD_BUDGET = 64;

const requireProtoBytes = (value: unknown): number => {
  if (!(value instanceof Uint8Array)) {
    throw new ReallyMeCodecError("invalid-input");
  }
  return value.length;
};

const requireProtoString = (value: unknown): string => {
  if (typeof value !== "string") {
    throw new ReallyMeCodecError("invalid-input");
  }
  return value;
};

const protoUtf8Length = (value: unknown): number | undefined => {
  const text = requireProtoString(value);
  try {
    return utf8ByteLength(text);
  } catch (error: unknown) {
    if (error instanceof ReallyMeCodecError) {
      return undefined;
    }
    throw error;
  }
};

const validateGeneratedProtoRequest = (
  request: CodecOperationRequest,
): boolean => {
  const lengths: number[] = [PROTO_REQUEST_OVERHEAD_BUDGET];
  switch (request.operation.case) {
    case undefined:
    case "multicodecTable":
      break;
    case "multicodecPrefixForName":
      {
        const length = protoUtf8Length(request.operation.value.name);
        if (length === undefined) {
          return false;
        }
        lengths.push(length);
      }
      break;
    case "multicodecLookupPrefix":
      lengths.push(requireProtoBytes(request.operation.value.value));
      break;
    case "multikeyParse":
      {
        const length = protoUtf8Length(request.operation.value.multikey);
        if (length === undefined) {
          return false;
        }
        lengths.push(length);
      }
      break;
    case "dagCborVerifyCid":
      {
        const length = protoUtf8Length(request.operation.value.cid);
        if (length === undefined) {
          return false;
        }
        lengths.push(length);
      }
      lengths.push(requireProtoBytes(request.operation.value.payload));
      break;
    case "pemDecode": {
      lengths.push(requireProtoBytes(request.operation.value.pem));
      const options = request.operation.value.options;
      if (options !== undefined) {
        if (!Array.isArray(options.allowedLabels)) {
          throw new ReallyMeCodecError("invalid-input");
        }
        if (options.allowedLabels.length > MAX_CODEC_PROTO_MESSAGE_BYTES / 5) {
          return false;
        }
        lengths.push(options.allowedLabels.length * 5);
      }
      break;
    }
    default:
      throw new ReallyMeCodecError("invalid-input");
  }
  try {
    requireBoundaryAggregate(lengths, MAX_CODEC_PROTO_MESSAGE_BYTES);
    return true;
  } catch (error: unknown) {
    if (error instanceof ReallyMeCodecError) {
      return false;
    }
    throw error;
  }
};

const boundaryResourceLimitResult = (): ReallyMeCodecProtoResult => {
  try {
    return {
      status: "codec-error",
      bytes: toBinary(
        CodecErrorSchema,
        create(CodecErrorSchema, {
          error: {
            case: "boundary",
            value: create(CodecBoundaryErrorSchema, {
              reason: CodecErrorReason.BOUNDARY_RESOURCE_LIMIT_EXCEEDED,
            }),
          },
        }),
      ),
      isCodecError: true,
    };
  } catch (_error: unknown) {
    throw new ReallyMeCodecError("provider-failure");
  }
};

const boundaryResourceLimitEnvelope = (): Uint8Array => {
  const result = boundaryResourceLimitResult();
  try {
    return toBinary(
      CodecProtoResultEnvelopeSchema,
      create(CodecProtoResultEnvelopeSchema, {
        status: CodecProtoResultStatus.CODEC_ERROR,
        payload: result.bytes,
      }),
    );
  } catch (_error: unknown) {
    throw new ReallyMeCodecError("provider-failure");
  } finally {
    result.bytes.fill(0);
  }
};

const decodeResultEnvelope = (
  envelopeBytes: Uint8Array,
): ReallyMeCodecProtoResult => {
  try {
    const envelope = fromBinary(CodecProtoResultEnvelopeSchema, envelopeBytes);
    try {
      switch (envelope.status) {
        case CodecProtoResultStatus.RESULT:
          return {
            status: "result",
            bytes: envelope.payload.slice(),
            isCodecError: false,
          };
        case CodecProtoResultStatus.CODEC_ERROR:
          return {
            status: "codec-error",
            bytes: envelope.payload.slice(),
            isCodecError: true,
          };
        case CodecProtoResultStatus.UNSPECIFIED:
          throw new ReallyMeCodecError("provider-failure");
      }
    } finally {
      envelope.payload.fill(0);
    }
  } catch (error: unknown) {
    if (error instanceof ReallyMeCodecError) {
      throw error;
    }
    throw new ReallyMeCodecError("provider-failure");
  }
};

/**
 * Execute one binary generated-protobuf request.
 *
 * The returned bytes are always a `CodecProtoResultEnvelope`; malformed or
 * unsupported requests are represented inside that envelope.
 */
export const processProto = (requestBytes: Uint8Array): Uint8Array => {
  ensureBytesInput(requestBytes);
  if (requestBytes.length > MAX_CODEC_PROTO_MESSAGE_BYTES) {
    return boundaryResourceLimitEnvelope();
  }
  return readIndependentBoundedBytesOutput(
    requireReallyMeCodecWasmProvider().processProto(requestBytes),
    requestBytes,
    MAX_CODEC_PROTO_RESULT_ENVELOPE_BYTES,
  );
};

/**
 * Execute one generated ProtoJSON request and return the same binary envelope
 * used by `processProto`.
 */
export const processProtoJson = (requestJson: Uint8Array): Uint8Array => {
  ensureBytesInput(requestJson);
  if (requestJson.length > MAX_CODEC_PROTO_JSON_BYTES) {
    return boundaryResourceLimitEnvelope();
  }
  return readIndependentBoundedBytesOutput(
    requireReallyMeCodecWasmProvider().processProtoJson(requestJson),
    requestJson,
    MAX_CODEC_PROTO_RESULT_ENVELOPE_BYTES,
  );
};

export const processGeneratedProtoRequest = (
  request: CodecOperationRequest,
): ReallyMeCodecProtoResult => {
  let withinBoundary: boolean;
  try {
    withinBoundary = validateGeneratedProtoRequest(request);
  } catch (error: unknown) {
    if (error instanceof ReallyMeCodecError) {
      throw error;
    }
    throw new ReallyMeCodecError("invalid-input");
  }
  if (!withinBoundary) {
    return boundaryResourceLimitResult();
  }
  let requestBytes: Uint8Array;
  try {
    requestBytes = toBinary(CodecOperationRequestSchema, request);
  } catch (_error: unknown) {
    throw new ReallyMeCodecError("provider-failure");
  }
  try {
    if (requestBytes.length > MAX_CODEC_PROTO_MESSAGE_BYTES) {
      throw new ReallyMeCodecError("invalid-input");
    }
    const envelopeBytes = processProto(requestBytes);
    try {
      return decodeResultEnvelope(envelopeBytes);
    } finally {
      envelopeBytes.fill(0);
    }
  } finally {
    requestBytes.fill(0);
  }
};

const errorCodeForCodecError = (
  bytes: Uint8Array,
): ReallyMeCodecErrorCode => {
  let error;
  try {
    error = fromBinary(CodecErrorSchema, bytes);
  } catch (_error: unknown) {
    return "provider-failure";
  }
  const reason = error.error.value?.reason;
  const reasonIsKnown =
    reason !== undefined &&
    Object.values(CodecErrorReason).some((candidate) => candidate === reason);
  if (!reasonIsKnown) {
    return "provider-failure";
  }
  switch (error.error.case) {
    case "baseEncoding":
      return reason >= 100 && reason <= 199
        ? "invalid-input"
        : "provider-failure";
    case "pem":
      return reason >= 200 && reason <= 299
        ? "invalid-input"
        : "provider-failure";
    case "multiformat":
      return reason >= 300 && reason <= 399
        ? "invalid-input"
        : "provider-failure";
    case "canonicalization":
      if (reason === CodecErrorReason.CANONICAL_INTERNAL) {
        return "provider-failure";
      }
      return reason >= 400 && reason <= 499
        ? "invalid-input"
        : "provider-failure";
    case "backend":
      return "provider-failure";
    case "boundary":
      // This is the only boundary error the generated facade creates from
      // caller input. Malformed/missing generated requests indicate provider
      // corruption or skew, not bad input supplied to the high-level API.
      return reason === CodecErrorReason.BOUNDARY_RESOURCE_LIMIT_EXCEEDED
        ? "invalid-input"
        : "provider-failure";
    case undefined:
      return "provider-failure";
  }
};

export const protoPayloadOrThrow = (
  result: ReallyMeCodecProtoResult,
): Uint8Array => {
  if (result.isCodecError) {
    const code = errorCodeForCodecError(result.bytes);
    result.bytes.fill(0);
    throw new ReallyMeCodecError(code);
  }
  return result.bytes;
};
