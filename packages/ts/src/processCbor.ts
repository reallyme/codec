// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import type {
  ReallyMeCborValue,
  ReallyMeDagCborCidVerification,
  ReallyMeDeterministicCborValue,
} from "./cborValues.js";
import { wipeDeterministicCborValueBytes, wipeProtoDeterministicCborValueBytes } from "./clearCbor.js";
import { dagCborValueFromDeterministic, dagCborValueToProto } from "./convertDagCbor.js";
import { MAX_DETERMINISTIC_CBOR_INPUT_LEN, MAX_DETERMINISTIC_CBOR_OUTPUT_LEN } from "./deterministicCborBoundary.js";
import { ReallyMeCodecError } from "./errors.js";
import { clearGeneratedOperationResult, processGeneratedOperationRequest } from "./operationContract.js";
import {
  CodecDagCborDecodeRequestSchema,
  CodecDagCborEncodeRequestSchema,
  CodecDagCborVerifyCidRequestSchema,
  CodecDeterministicCborDecodeRequestSchema,
  CodecDeterministicCborEncodeRequestSchema,
  CodecOperationRequestSchema,
} from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import type { CodecDeterministicCborValue, CodecOperationRequest } from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import {
  deterministicCborValueFromProto,
  providerFailure,
  requireNoProviderUnknownFields,
} from "./readDeterministicCborProto.js";
import {
  ensureBytesInput,
  ensureStringValue,
  readBytesOutput,
  readNumberOutput,
  readStringOutput,
  snapshotProviderBytes,
} from "./readOutput.js";
import {
  deterministicCborValidationState,
  snapshotDeterministicCborBytes,
  wipeOwnedByteSnapshots,
} from "./snapshotDeterministicCbor.js";
import {
  invalidCborInput,
  maxDagCborInputLength,
  validateCborValue,
  wipeDagCborValueBytes,
} from "./validateDagCbor.js";
import { normalizeDeterministicCborValue } from "./validateDeterministicCbor.js";
import { requireReallyMeCodecWasmProvider } from "./wasmProvider.js";
import { deterministicCborValueToProto } from "./writeDeterministicCborProto.js";
import { create } from "@bufbuild/protobuf";

export const dagCborEncode = (value: ReallyMeCborValue): Uint8Array => {
  let normalized: ReallyMeCborValue | undefined;
  try {
    normalized = validateCborValue(value);
  } catch (error: unknown) {
    if (error instanceof ReallyMeCodecError) {
      throw error;
    }
    return invalidCborInput();
  }
  let protoValue: CodecDeterministicCborValue | undefined;
  try {
    protoValue = dagCborValueToProto(normalized);
    const operationResult = processGeneratedOperationRequest(
      create(CodecOperationRequestSchema, {
        operation: {
          case: "dagCborEncode",
          value: create(CodecDagCborEncodeRequestSchema, {
            value: protoValue,
          }),
        },
      }),
    );
    if (operationResult.result.case !== "dagCborEncode") {
      clearGeneratedOperationResult(operationResult);
      throw new ReallyMeCodecError("provider-failure");
    }
    const result = operationResult.result.value;
    try {
      requireNoProviderUnknownFields(result);
      if (
        result.encoded.length === 0 ||
        result.encoded.length > maxDagCborInputLength
      ) {
        return providerFailure();
      }
      return snapshotProviderBytes(result.encoded, maxDagCborInputLength);
    } catch (error: unknown) {
      if (error instanceof ReallyMeCodecError) {
        throw error;
      }
      throw new ReallyMeCodecError("provider-failure");
    } finally {
      result.encoded.fill(0);
    }
  } finally {
    wipeProtoDeterministicCborValueBytes(protoValue);
    wipeDagCborValueBytes(normalized);
  }
};

export const dagCborDecode = (bytes: Uint8Array): ReallyMeCborValue => {
  ensureBytesInput(bytes);
  if (bytes.length > maxDagCborInputLength) {
    throw new ReallyMeCodecError("invalid-input");
  }
  const requestBytes = snapshotDeterministicCborBytes(bytes, maxDagCborInputLength);
  try {
    const operationResult = processGeneratedOperationRequest(
      create(CodecOperationRequestSchema, {
        operation: {
          case: "dagCborDecode",
          value: create(CodecDagCborDecodeRequestSchema, {
            encoded: requestBytes,
          }),
        },
      }),
    );
    if (operationResult.result.case !== "dagCborDecode") {
      clearGeneratedOperationResult(operationResult);
      throw new ReallyMeCodecError("provider-failure");
    }
    const result = operationResult.result.value;
    const state = deterministicCborValidationState();
    let normalized: ReallyMeDeterministicCborValue | undefined;
    try {
      requireNoProviderUnknownFields(result);
      normalized = deterministicCborValueFromProto(result.value, 0, state);
      return dagCborValueFromDeterministic(normalized);
    } catch (error: unknown) {
      if (error instanceof ReallyMeCodecError) {
        throw error;
      }
      throw new ReallyMeCodecError("provider-failure");
    } finally {
      wipeOwnedByteSnapshots(state);
      if (normalized !== undefined) {
        wipeDeterministicCborValueBytes(normalized);
      }
      wipeProtoDeterministicCborValueBytes(result.value);
    }
  } finally {
    requestBytes.fill(0);
  }
};

export const deterministicCborEncode = (value: unknown): Uint8Array => {
  const normalized = normalizeDeterministicCborValue(value);
  let protoValue: CodecDeterministicCborValue | undefined;
  try {
    protoValue = deterministicCborValueToProto(normalized);
    const operationResult = processGeneratedOperationRequest(
      create(CodecOperationRequestSchema, {
        operation: {
          case: "deterministicCborEncode",
          value: create(CodecDeterministicCborEncodeRequestSchema, {
            value: protoValue,
          }),
        },
      }),
    );
    if (operationResult.result.case !== "deterministicCborEncode") {
      clearGeneratedOperationResult(operationResult);
      throw new ReallyMeCodecError("provider-failure");
    }
    const result = operationResult.result.value;
    try {
      requireNoProviderUnknownFields(result);
      if (
        result.encoded.length === 0 ||
        result.encoded.length > MAX_DETERMINISTIC_CBOR_OUTPUT_LEN
      ) {
        throw new ReallyMeCodecError("provider-failure");
      }
      return snapshotProviderBytes(result.encoded, MAX_DETERMINISTIC_CBOR_OUTPUT_LEN);
    } finally {
      result.encoded.fill(0);
    }
  } finally {
    wipeProtoDeterministicCborValueBytes(protoValue);
    wipeDeterministicCborValueBytes(normalized);
  }
};

export const deterministicCborDecode = (
  bytes: Uint8Array,
): ReallyMeDeterministicCborValue => {
  ensureBytesInput(bytes);
  if (bytes.length > MAX_DETERMINISTIC_CBOR_INPUT_LEN) {
    throw new ReallyMeCodecError("invalid-input");
  }
  const requestBytes = snapshotDeterministicCborBytes(
    bytes,
    MAX_DETERMINISTIC_CBOR_INPUT_LEN,
  );
  try {
    const operationResult = processGeneratedOperationRequest(
      create(CodecOperationRequestSchema, {
        operation: {
          case: "deterministicCborDecode",
          value: create(CodecDeterministicCborDecodeRequestSchema, {
            encoded: requestBytes,
          }),
        },
      }),
    );
    if (operationResult.result.case !== "deterministicCborDecode") {
      clearGeneratedOperationResult(operationResult);
      throw new ReallyMeCodecError("provider-failure");
    }
    const result = operationResult.result.value;
    const state = deterministicCborValidationState();
    try {
      requireNoProviderUnknownFields(result);
      return deterministicCborValueFromProto(result.value, 0, state);
    } catch (error: unknown) {
      wipeOwnedByteSnapshots(state);
      throw error;
    } finally {
      wipeProtoDeterministicCborValueBytes(result.value);
    }
  } finally {
    requestBytes.fill(0);
  }
};

export const dagCborComputeCid = (bytes: Uint8Array): string => {
  ensureBytesInput(bytes);
  const snapshot = snapshotDeterministicCborBytes(bytes, maxDagCborInputLength);
  try {
    return readStringOutput(requireReallyMeCodecWasmProvider().dagCborComputeCid(snapshot));
  } finally {
    snapshot.fill(0);
  }
};

export const dagCborVerifyCidDetails = (
  cid: string,
  bytes: Uint8Array,
): ReallyMeDagCborCidVerification => {
  ensureStringValue(cid);
  ensureBytesInput(bytes);
  const payloadSnapshot = snapshotDeterministicCborBytes(bytes, maxDagCborInputLength);
  try {
    const operationResult = processGeneratedOperationRequest(
      dagCborVerifyCidRequest(cid, payloadSnapshot),
    );
    if (operationResult.result.case !== "dagCborVerifyCid") {
      clearGeneratedOperationResult(operationResult);
      throw new ReallyMeCodecError("provider-failure");
    }
    const result = operationResult.result.value;
    requireNoProviderUnknownFields(result);
    return {
      valid: result.valid,
      expectedCid: result.expectedCid,
      actualCid: result.actualCid,
    };
  } finally {
    payloadSnapshot.fill(0);
  }
};

/** Verify one CID and return a boolean that is safe to use directly in a condition. */
export const dagCborVerifyCid = (cid: string, bytes: Uint8Array): boolean =>
  dagCborVerifyCidDetails(cid, bytes).valid;

/** Alias for callers that prefer an explicit comparison name. */
export const dagCborCidMatches = dagCborVerifyCid;

const dagCborVerifyCidRequest = (
  cid: string,
  bytes: Uint8Array,
): CodecOperationRequest => create(CodecOperationRequestSchema, {
  operation: {
    case: "dagCborVerifyCid",
    value: create(CodecDagCborVerifyCidRequestSchema, {
      cid,
      payload: bytes,
    }),
  },
});

export const dagCborSha256ContentHash = (bytes: Uint8Array): Uint8Array => {
  ensureBytesInput(bytes);
  const snapshot = snapshotDeterministicCborBytes(bytes, maxDagCborInputLength);
  try {
    return readBytesOutput(requireReallyMeCodecWasmProvider().dagCborSha256ContentHash(snapshot));
  } finally {
    snapshot.fill(0);
  }
};

export const dagCborMultihash = (bytes: Uint8Array): Uint8Array => {
  ensureBytesInput(bytes);
  const snapshot = snapshotDeterministicCborBytes(bytes, maxDagCborInputLength);
  try {
    return readBytesOutput(requireReallyMeCodecWasmProvider().dagCborMultihash(snapshot));
  } finally {
    snapshot.fill(0);
  }
};

export const isValidCidString = (cid: string): boolean => {
  ensureStringValue(cid);
  const value = requireReallyMeCodecWasmProvider().isValidCidString(cid);
  if (typeof value !== "boolean") {
    throw new ReallyMeCodecError("provider-failure");
  }
  return value;
};

export const tryParseCid = (cid: string): string | undefined => {
  try {
    ensureStringValue(cid);
    const value = requireReallyMeCodecWasmProvider().tryParseCid(cid);
    if (value === undefined) {
      return undefined;
    }
    return readStringOutput(value);
  } catch (error: unknown) {
    if (error instanceof ReallyMeCodecError && error.code === "invalid-input") {
      return undefined;
    }
    throw error;
  }
};

export const dagCborCodecCode = (): number =>
  readNumberOutput(requireReallyMeCodecWasmProvider().dagCborCodecCode());
