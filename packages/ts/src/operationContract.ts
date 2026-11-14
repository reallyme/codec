// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { MAX_CODEC_PROTO_JSON_BYTES, MAX_CODEC_PROTO_MESSAGE_BYTES } from "./boundary.js";
import { resourceLimitOperationResponse } from "./boundedOperationResponse.js";
import { errorCodeForCodecErrorMessage } from "./classifyOperationError.js";
import { clearGeneratedOperationResult, detachGeneratedOperationResultBytes } from "./clearGeneratedOperation.js";
import { MAX_DETERMINISTIC_CBOR_PROTO_RECURSION_DEPTH } from "./deterministicCborBoundary.js";
import { ReallyMeCodecError } from "./errors.js";
import { CodecOperationRequestSchema, CodecOperationResponseSchema } from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import type { CodecOperationRequest, CodecOperationResult } from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import {
  ensureBytesInput,
  readIndependentBoundedBytesOutput,
  snapshotBoundedBytesInput,
} from "./readOutput.js";
import { serializeExactBinary } from "./serializeExactBinary.js";
import { hasUnknownFields } from "./validateGeneratedCbor.js";
import { validateGeneratedOperationRequest } from "./validateGeneratedOperation.js";
import { requireReallyMeCodecWasmProvider } from "./wasmProvider.js";
import { fromBinary } from "@bufbuild/protobuf";

/**
 * Execute one binary generated-protobuf request and return the fully
 * discriminated binary `CodecOperationResponse`.
 *
 * Oversized and malformed requests are represented by the response error
 * oneof. Caller-owned oversized bytes are not copied into WASM.
 */
export const processOperation = (requestBytes: Uint8Array): Uint8Array => {
  ensureBytesInput(requestBytes);
  if (requestBytes.length > MAX_CODEC_PROTO_MESSAGE_BYTES) {
    return resourceLimitOperationResponse();
  }
  const requestSnapshot = snapshotBoundedBytesInput(
    requestBytes,
    MAX_CODEC_PROTO_MESSAGE_BYTES,
  );
  try {
    return readIndependentBoundedBytesOutput(
      requireReallyMeCodecWasmProvider().processOperation(requestSnapshot),
      requestSnapshot,
      MAX_CODEC_PROTO_MESSAGE_BYTES,
    );
  } finally {
    requestSnapshot.fill(0);
  }
};

/**
 * Execute one generated ProtoJSON request and return the same discriminated
 * binary response used by `processOperation`.
 */
export const processOperationJson = (requestJson: Uint8Array): Uint8Array => {
  ensureBytesInput(requestJson);
  if (requestJson.length > MAX_CODEC_PROTO_JSON_BYTES) {
    return resourceLimitOperationResponse();
  }
  const requestSnapshot = snapshotBoundedBytesInput(
    requestJson,
    MAX_CODEC_PROTO_JSON_BYTES,
  );
  try {
    return readIndependentBoundedBytesOutput(
      requireReallyMeCodecWasmProvider().processOperationJson(requestSnapshot),
      requestSnapshot,
      MAX_CODEC_PROTO_MESSAGE_BYTES,
    );
  } finally {
    requestSnapshot.fill(0);
  }
};

/**
 * Execute a generated request through the fully discriminated response lane.
 *
 * Structured convenience methods use this boundary so a provider cannot
 * return a valid payload of the wrong operation-specific result type.
 */
export const processGeneratedOperationRequest = (
  request: CodecOperationRequest,
): CodecOperationResult => {
  let withinBoundary: boolean;
  try {
    withinBoundary = validateGeneratedOperationRequest(request);
  } catch (error: unknown) {
    if (error instanceof ReallyMeCodecError) {
      throw error;
    }
    throw new ReallyMeCodecError("invalid-input");
  }
  if (!withinBoundary) {
    throw new ReallyMeCodecError("invalid-input");
  }

  let requestBytes: Uint8Array;
  try {
    // Generated serialization through the descriptor, into one exactly sized
    // buffer: the protobuf-es writer's abandoned growth buffers would keep
    // copies of sensitive text and byte fields that cannot be wiped.
    requestBytes = serializeExactBinary(CodecOperationRequestSchema, request);
  } catch (error: unknown) {
    if (error instanceof ReallyMeCodecError) {
      throw error;
    }
    throw new ReallyMeCodecError("provider-failure");
  }
  return processOperationRequestBytes(requestBytes);
};

/**
 * Execute a serialized request through the fully discriminated response lane.
 *
 * Takes ownership of `requestBytes` and wipes it on every exit path. Sensitive
 * operations use descriptor-driven serialization so no serializer copy survives.
 */
export const processOperationRequestBytes = (
  requestBytes: Uint8Array,
): CodecOperationResult => {
  try {
    if (requestBytes.length > MAX_CODEC_PROTO_MESSAGE_BYTES) {
      throw new ReallyMeCodecError("invalid-input");
    }
    const responseBytes = processOperation(requestBytes);
    try {
      const response = fromBinary(CodecOperationResponseSchema, responseBytes, {
        readUnknownFields: true,
        recursionLimit: MAX_DETERMINISTIC_CBOR_PROTO_RECURSION_DEPTH,
      });
      if (hasUnknownFields(response)) {
        throw new ReallyMeCodecError("provider-failure");
      }
      switch (response.outcome.case) {
        case "result":
          if (hasUnknownFields(response.outcome.value)) {
            clearGeneratedOperationResult(response.outcome.value);
            throw new ReallyMeCodecError("provider-failure");
          }
          try {
            detachGeneratedOperationResultBytes(response.outcome.value);
          } catch (_error: unknown) {
            // Detachment can allocate recursively. If an allocation or a
            // future generated-shape invariant fails after only some fields
            // were detached, clear both the detached owners and any remaining
            // views before the serialized response owner is wiped below.
            clearGeneratedOperationResult(response.outcome.value);
            throw new ReallyMeCodecError("provider-failure");
          }
          return response.outcome.value;
        case "error":
          throw new ReallyMeCodecError(
            errorCodeForCodecErrorMessage(response.outcome.value),
          );
        case undefined:
          throw new ReallyMeCodecError("provider-failure");
      }
    } catch (error: unknown) {
      if (error instanceof ReallyMeCodecError) {
        throw error;
      }
      throw new ReallyMeCodecError("provider-failure");
    } finally {
      responseBytes.fill(0);
    }
  } finally {
    requestBytes.fill(0);
  }
};

export {
  clearGeneratedOperationResult,
} from "./clearGeneratedOperation.js";
