// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import {
  MAX_CODEC_FFI_INPUT_BYTES,
  MAX_CODEC_PEM_INPUT_BYTES,
  MAX_CODEC_PROTO_MESSAGE_BYTES,
  requireBoundaryAggregate,
} from "./boundary.js";
import { PROTO_REQUEST_OVERHEAD_BUDGET } from "./boundedOperationResponse.js";
import { MAX_DETERMINISTIC_CBOR_INPUT_LEN } from "./deterministicCborBoundary.js";
import { ReallyMeCodecError } from "./errors.js";
import type { CodecOperationRequest } from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import {
  protoUtf8Length,
  requireNoGeneratedUnknownFields,
  requireProtoBytes,
  validateGeneratedDagCborValue,
  validateGeneratedDeterministicCborValue,
} from "./validateGeneratedCbor.js";

export const validateGeneratedOperationRequest = (
  request: CodecOperationRequest,
): boolean => {
  requireNoGeneratedUnknownFields(request);
  const lengths: number[] = [PROTO_REQUEST_OVERHEAD_BUDGET];
  switch (request.operation.case) {
    case undefined:
      throw new ReallyMeCodecError("invalid-input");
    case "multicodecTable":
      requireNoGeneratedUnknownFields(request.operation.value);
      break;
    case "multicodecPrefixForName":
      {
        requireNoGeneratedUnknownFields(request.operation.value);
        const length = protoUtf8Length(request.operation.value.name);
        if (length === undefined) {
          return false;
        }
        if (length > MAX_CODEC_FFI_INPUT_BYTES) {
          return false;
        }
        lengths.push(length);
      }
      break;
    case "multicodecLookupPrefix": {
      requireNoGeneratedUnknownFields(request.operation.value);
      const length = requireProtoBytes(request.operation.value.value);
      if (length > MAX_CODEC_FFI_INPUT_BYTES) {
        return false;
      }
      lengths.push(length);
      break;
    }
    case "multikeyParse":
      {
        requireNoGeneratedUnknownFields(request.operation.value);
        const length = protoUtf8Length(request.operation.value.multikey);
        if (length === undefined) {
          return false;
        }
        if (length > MAX_CODEC_FFI_INPUT_BYTES) {
          return false;
        }
        lengths.push(length);
      }
      break;
    case "dagCborVerifyCid":
      {
        requireNoGeneratedUnknownFields(request.operation.value);
        const length = protoUtf8Length(request.operation.value.cid);
        if (length === undefined) {
          return false;
        }
        const payloadLength = requireProtoBytes(request.operation.value.payload);
        try {
          requireBoundaryAggregate(
            [length, payloadLength],
            MAX_CODEC_FFI_INPUT_BYTES,
          );
        } catch (error: unknown) {
          if (error instanceof ReallyMeCodecError) {
            return false;
          }
          throw error;
        }
        lengths.push(length, payloadLength);
      }
      break;
    case "dagCborEncode":
      requireNoGeneratedUnknownFields(request.operation.value);
      validateGeneratedDagCborValue(
        request.operation.value.value,
        0,
        { nodes: 0, textBytes: 0, byteStringBytes: 0 },
        lengths,
      );
      break;
    case "dagCborDecode": {
      requireNoGeneratedUnknownFields(request.operation.value);
      const length = requireProtoBytes(request.operation.value.encoded);
      if (length > MAX_DETERMINISTIC_CBOR_INPUT_LEN) {
        return false;
      }
      lengths.push(length);
      break;
    }
    case "deterministicCborEncode":
      requireNoGeneratedUnknownFields(request.operation.value);
      validateGeneratedDeterministicCborValue(
        request.operation.value.value,
        0,
        { nodes: 0, textBytes: 0, byteStringBytes: 0 },
        lengths,
      );
      break;
    case "deterministicCborDecode": {
      requireNoGeneratedUnknownFields(request.operation.value);
      const length = requireProtoBytes(request.operation.value.encoded);
      if (length > MAX_DETERMINISTIC_CBOR_INPUT_LEN) {
        return false;
      }
      lengths.push(length);
      break;
    }
    case "pemDecode": {
      requireNoGeneratedUnknownFields(request.operation.value);
      const pemLength = requireProtoBytes(request.operation.value.pem);
      if (pemLength > MAX_CODEC_PEM_INPUT_BYTES) {
        return false;
      }
      const options = request.operation.value.options;
      let optionsLength = 0;
      if (options !== undefined) {
        requireNoGeneratedUnknownFields(options);
        if (!Array.isArray(options.allowedLabels)) {
          throw new ReallyMeCodecError("invalid-input");
        }
        if (options.allowedLabels.length > MAX_CODEC_PROTO_MESSAGE_BYTES / 5) {
          return false;
        }
        optionsLength = options.allowedLabels.length * 5;
      }
      lengths.push(pemLength, optionsLength);
      break;
    }
    case "pemEncode": {
      requireNoGeneratedUnknownFields(request.operation.value);
      const derLength = requireProtoBytes(request.operation.value.der);
      const options = request.operation.value.options;
      if (options !== undefined) {
        requireNoGeneratedUnknownFields(options);
      }
      try {
        requireBoundaryAggregate([derLength], MAX_CODEC_FFI_INPUT_BYTES);
      } catch (error: unknown) {
        if (error instanceof ReallyMeCodecError) {
          return false;
        }
        throw error;
      }
      lengths.push(derLength);
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
