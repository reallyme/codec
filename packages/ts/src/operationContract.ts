// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import type { Message } from "@bufbuild/protobuf";

import {
  MAX_CODEC_FFI_INPUT_BYTES,
  MAX_CODEC_PROTO_JSON_BYTES,
  MAX_CODEC_PROTO_MESSAGE_BYTES,
  requireBoundaryAggregate,
  strictUtf8ByteLength,
  utf8ByteLength,
} from "./boundary.js";
import {
  DETERMINISTIC_CBOR_I64_MIN,
  DETERMINISTIC_CBOR_U64_MAX,
  MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
  MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES,
  MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES,
  MAX_DETERMINISTIC_CBOR_INPUT_LEN,
  MAX_DETERMINISTIC_CBOR_NESTING_DEPTH,
  MAX_DETERMINISTIC_CBOR_NODES,
  MAX_DETERMINISTIC_CBOR_PROTO_RECURSION_DEPTH,
} from "./deterministicCborBoundary.js";
import { ReallyMeCodecError } from "./errors.js";
import type { ReallyMeCodecErrorCode } from "./errors.js";
import {
  CodecErrorOrigin,
  CodecErrorReason,
  CodecOperationResponseSchema,
  CodecOperationRequestSchema,
} from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import type {
  CodecDeterministicCborInteger,
  CodecDeterministicCborMapKey,
  CodecDeterministicCborValue,
  CodecError,
  CodecOperationRequest,
  CodecOperationResult,
} from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import {
  readIndependentBoundedBytesOutput,
  snapshotBoundedBytesInput,
} from "./readOutput.js";
import { requireReallyMeCodecWasmProvider } from "./wasmProvider.js";

const PROTO_REQUEST_OVERHEAD_BUDGET = 64;
const DETERMINISTIC_CBOR_PROTO_NODE_OVERHEAD_BUDGET = 16;
const DAG_CBOR_I64_MAX = (1n << 63n) - 1n;

type GeneratedDeterministicCborBudget = {
  nodes: number;
  textBytes: number;
  byteStringBytes: number;
};

type GeneratedDeterministicCborMapKeyIdentity =
  | Readonly<{ type: "integer"; value: bigint }>
  | Readonly<{ type: "text"; value: string }>;

const requireNoGeneratedUnknownFields = (message: Message): void => {
  if (message.$unknown !== undefined && message.$unknown.length !== 0) {
    throw new ReallyMeCodecError("invalid-input");
  }
};

const hasUnknownFields = (message: Message): boolean =>
  message.$unknown !== undefined && message.$unknown.length !== 0;

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

const protoUtf8Length = (
  value: unknown,
  maximum = MAX_CODEC_FFI_INPUT_BYTES,
): number | undefined => {
  const text = requireProtoString(value);
  if (text.length > maximum) {
    return undefined;
  }
  try {
    return utf8ByteLength(text);
  } catch (error: unknown) {
    if (error instanceof ReallyMeCodecError) {
      return undefined;
    }
    throw error;
  }
};

const deterministicCborProtoUtf8Length = (
  value: unknown,
  maximum = MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES,
): number | undefined => {
  const text = requireProtoString(value);
  if (text.length > maximum) {
    return undefined;
  }
  try {
    return strictUtf8ByteLength(text);
  } catch (error: unknown) {
    if (error instanceof ReallyMeCodecError) {
      return undefined;
    }
    throw error;
  }
};

const consumeGeneratedDeterministicCborBudget = (
  current: number,
  amount: number,
  maximum: number,
): number => {
  if (
    !Number.isSafeInteger(amount) ||
    amount < 0 ||
    current > maximum - amount
  ) {
    throw new ReallyMeCodecError("invalid-input");
  }
  return current + amount;
};

const consumeGeneratedDeterministicCborText = (
  value: unknown,
  budget: GeneratedDeterministicCborBudget,
  lengths: number[],
): void => {
  const text = requireProtoString(value);
  if (
    budget.textBytes >
    MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES - text.length
  ) {
    throw new ReallyMeCodecError("invalid-input");
  }
  const length = deterministicCborProtoUtf8Length(text);
  if (length === undefined) {
    throw new ReallyMeCodecError("invalid-input");
  }
  budget.textBytes = consumeGeneratedDeterministicCborBudget(
    budget.textBytes,
    length,
    MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES,
  );
  lengths.push(length);
};

const validateGeneratedDeterministicCborInteger = (
  integer: CodecDeterministicCborInteger,
): bigint => {
  requireNoGeneratedUnknownFields(integer);
  switch (integer.value.case) {
    case "unsignedValue":
      requireNoGeneratedUnknownFields(integer.value.value);
      if (
        integer.value.value.value < 0n ||
        integer.value.value.value > DETERMINISTIC_CBOR_U64_MAX
      ) {
        throw new ReallyMeCodecError("invalid-input");
      }
      return integer.value.value.value;
    case "negativeValue":
      requireNoGeneratedUnknownFields(integer.value.value);
      if (
        integer.value.value.value < DETERMINISTIC_CBOR_I64_MIN ||
        integer.value.value.value >= 0n
      ) {
        throw new ReallyMeCodecError("invalid-input");
      }
      return integer.value.value.value;
    case undefined:
      throw new ReallyMeCodecError("invalid-input");
  }
};

const validateGeneratedDeterministicCborMapKey = (
  key: CodecDeterministicCborMapKey | undefined,
  budget: GeneratedDeterministicCborBudget,
  lengths: number[],
): GeneratedDeterministicCborMapKeyIdentity => {
  if (key === undefined) {
    throw new ReallyMeCodecError("invalid-input");
  }
  requireNoGeneratedUnknownFields(key);
  budget.nodes = consumeGeneratedDeterministicCborBudget(
    budget.nodes,
    1,
    MAX_DETERMINISTIC_CBOR_NODES,
  );
  lengths.push(DETERMINISTIC_CBOR_PROTO_NODE_OVERHEAD_BUDGET);
  switch (key.key.case) {
    case "integerKey":
      return {
        type: "integer",
        value: validateGeneratedDeterministicCborInteger(key.key.value),
      };
    case "textKey": {
      requireNoGeneratedUnknownFields(key.key.value);
      consumeGeneratedDeterministicCborText(
        key.key.value.value,
        budget,
        lengths,
      );
      return { type: "text", value: key.key.value.value };
    }
    case undefined:
      throw new ReallyMeCodecError("invalid-input");
  }
};

const validateGeneratedDeterministicCborValue = (
  value: CodecDeterministicCborValue | undefined,
  depth: number,
  budget: GeneratedDeterministicCborBudget,
  lengths: number[],
): void => {
  if (value === undefined) {
    throw new ReallyMeCodecError("invalid-input");
  }
  requireNoGeneratedUnknownFields(value);
  budget.nodes = consumeGeneratedDeterministicCborBudget(
    budget.nodes,
    1,
    MAX_DETERMINISTIC_CBOR_NODES,
  );
  lengths.push(DETERMINISTIC_CBOR_PROTO_NODE_OVERHEAD_BUDGET);
  switch (value.value.case) {
    case "nullValue":
      requireNoGeneratedUnknownFields(value.value.value);
      break;
    case "boolValue":
      requireNoGeneratedUnknownFields(value.value.value);
      break;
    case "integerValue":
      validateGeneratedDeterministicCborInteger(value.value.value);
      break;
    case "textValue": {
      requireNoGeneratedUnknownFields(value.value.value);
      consumeGeneratedDeterministicCborText(
        value.value.value.value,
        budget,
        lengths,
      );
      break;
    }
    case "bytesValue": {
      requireNoGeneratedUnknownFields(value.value.value);
      const length = requireProtoBytes(value.value.value.value);
      budget.byteStringBytes = consumeGeneratedDeterministicCborBudget(
        budget.byteStringBytes,
        length,
        MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
      );
      lengths.push(length);
      break;
    }
    case "arrayValue": {
      requireNoGeneratedUnknownFields(value.value.value);
      const values = value.value.value.values;
      if (
        !Array.isArray(values) ||
        values.length > MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES ||
        depth >= MAX_DETERMINISTIC_CBOR_NESTING_DEPTH
      ) {
        throw new ReallyMeCodecError("invalid-input");
      }
      for (const entry of values) {
        validateGeneratedDeterministicCborValue(entry, depth + 1, budget, lengths);
      }
      break;
    }
    case "mapValue": {
      requireNoGeneratedUnknownFields(value.value.value);
      const entries = value.value.value.entries;
      if (
        !Array.isArray(entries) ||
        entries.length > MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES ||
        depth >= MAX_DETERMINISTIC_CBOR_NESTING_DEPTH
      ) {
        throw new ReallyMeCodecError("invalid-input");
      }
      const integerKeys = new Set<bigint>();
      const textKeys = new Set<string>();
      for (const entry of entries) {
        requireNoGeneratedUnknownFields(entry);
        const key = validateGeneratedDeterministicCborMapKey(
          entry.key,
          budget,
          lengths,
        );
        if (key.type === "integer") {
          if (integerKeys.has(key.value)) {
            throw new ReallyMeCodecError("invalid-input");
          }
          integerKeys.add(key.value);
        } else {
          if (textKeys.has(key.value)) {
            throw new ReallyMeCodecError("invalid-input");
          }
          textKeys.add(key.value);
        }
        validateGeneratedDeterministicCborValue(
          entry.value,
          depth + 1,
          budget,
          lengths,
        );
      }
      break;
    }
    case undefined:
      throw new ReallyMeCodecError("invalid-input");
  }
};

const validateGeneratedDagCborInteger = (
  integer: CodecDeterministicCborInteger,
): bigint => {
  const value = validateGeneratedDeterministicCborInteger(integer);
  if (value > DAG_CBOR_I64_MAX) {
    throw new ReallyMeCodecError("invalid-input");
  }
  return value;
};

const validateGeneratedDagCborMapKey = (
  key: CodecDeterministicCborMapKey | undefined,
  budget: GeneratedDeterministicCborBudget,
  lengths: number[],
): string => {
  if (key === undefined) {
    throw new ReallyMeCodecError("invalid-input");
  }
  requireNoGeneratedUnknownFields(key);
  budget.nodes = consumeGeneratedDeterministicCborBudget(
    budget.nodes,
    1,
    MAX_DETERMINISTIC_CBOR_NODES,
  );
  lengths.push(DETERMINISTIC_CBOR_PROTO_NODE_OVERHEAD_BUDGET);
  switch (key.key.case) {
    case "textKey":
      requireNoGeneratedUnknownFields(key.key.value);
      consumeGeneratedDeterministicCborText(
        key.key.value.value,
        budget,
        lengths,
      );
      return key.key.value.value;
    case "integerKey":
    case undefined:
      throw new ReallyMeCodecError("invalid-input");
  }
};

const validateGeneratedDagCborValue = (
  value: CodecDeterministicCborValue | undefined,
  depth: number,
  budget: GeneratedDeterministicCborBudget,
  lengths: number[],
): void => {
  if (value === undefined) {
    throw new ReallyMeCodecError("invalid-input");
  }
  requireNoGeneratedUnknownFields(value);
  budget.nodes = consumeGeneratedDeterministicCborBudget(
    budget.nodes,
    1,
    MAX_DETERMINISTIC_CBOR_NODES,
  );
  lengths.push(DETERMINISTIC_CBOR_PROTO_NODE_OVERHEAD_BUDGET);
  switch (value.value.case) {
    case "nullValue":
      requireNoGeneratedUnknownFields(value.value.value);
      break;
    case "boolValue":
      requireNoGeneratedUnknownFields(value.value.value);
      break;
    case "integerValue":
      validateGeneratedDagCborInteger(value.value.value);
      break;
    case "textValue":
      requireNoGeneratedUnknownFields(value.value.value);
      consumeGeneratedDeterministicCborText(
        value.value.value.value,
        budget,
        lengths,
      );
      break;
    case "bytesValue": {
      requireNoGeneratedUnknownFields(value.value.value);
      const length = requireProtoBytes(value.value.value.value);
      budget.byteStringBytes = consumeGeneratedDeterministicCborBudget(
        budget.byteStringBytes,
        length,
        MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
      );
      lengths.push(length);
      break;
    }
    case "arrayValue": {
      requireNoGeneratedUnknownFields(value.value.value);
      const values = value.value.value.values;
      if (
        !Array.isArray(values) ||
        values.length > MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES ||
        depth >= MAX_DETERMINISTIC_CBOR_NESTING_DEPTH
      ) {
        throw new ReallyMeCodecError("invalid-input");
      }
      for (const entry of values) {
        validateGeneratedDagCborValue(entry, depth + 1, budget, lengths);
      }
      break;
    }
    case "mapValue": {
      requireNoGeneratedUnknownFields(value.value.value);
      const entries = value.value.value.entries;
      if (
        !Array.isArray(entries) ||
        entries.length > MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES ||
        depth >= MAX_DETERMINISTIC_CBOR_NESTING_DEPTH
      ) {
        throw new ReallyMeCodecError("invalid-input");
      }
      const textKeys = new Set<string>();
      for (const entry of entries) {
        requireNoGeneratedUnknownFields(entry);
        const key = validateGeneratedDagCborMapKey(
          entry.key,
          budget,
          lengths,
        );
        if (textKeys.has(key)) {
          throw new ReallyMeCodecError("invalid-input");
        }
        textKeys.add(key);
        validateGeneratedDagCborValue(entry.value, depth + 1, budget, lengths);
      }
      break;
    }
    case undefined:
      throw new ReallyMeCodecError("invalid-input");
  }
};

const validateGeneratedOperationRequest = (
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
      try {
        requireBoundaryAggregate(
          [pemLength, optionsLength],
          MAX_CODEC_FFI_INPUT_BYTES,
        );
      } catch (error: unknown) {
        if (error instanceof ReallyMeCodecError) {
          return false;
        }
        throw error;
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

/**
 * Execute one binary generated-protobuf request and return the fully
 * discriminated binary `CodecOperationResponse`.
 *
 * Oversized requests are rejected before the provider call. Malformed and
 * unsupported in-limit requests are represented by the response error oneof
 * rather than a boundary-local exception.
 */
export const processOperation = (requestBytes: Uint8Array): Uint8Array => {
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
    requestBytes = toBinary(CodecOperationRequestSchema, request);
  } catch (_error: unknown) {
    throw new ReallyMeCodecError("provider-failure");
  }
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

// Buf's binary decoder may expose byte fields as views into the serialized
// response owner. Detach every byte field before that owner is wiped so the
// returned generated result remains valid and has independent cleanup.
const detachGeneratedOperationResultBytes = (
  result: CodecOperationResult,
): void => {
  switch (result.result.case) {
    case "multicodecPrefixForName":
      result.result.value.prefix = result.result.value.prefix.slice();
      break;
    case "multicodecLookupPrefix":
      if (result.result.value.metadata !== undefined) {
        result.result.value.metadata.prefix =
          result.result.value.metadata.prefix.slice();
      }
      break;
    case "multicodecTable":
      for (const entry of result.result.value.entries) {
        entry.prefix = entry.prefix.slice();
      }
      break;
    case "multikeyParse":
      result.result.value.publicKey = result.result.value.publicKey.slice();
      break;
    case "dagCborVerifyCid":
      break;
    case "dagCborEncode":
      result.result.value.encoded = result.result.value.encoded.slice();
      break;
    case "dagCborDecode":
      detachGeneratedCborValueBytes(result.result.value.value);
      break;
    case "pemDecode":
      result.result.value.der = result.result.value.der.slice();
      break;
    case "pemEncode":
      result.result.value.pem = result.result.value.pem.slice();
      break;
    case "deterministicCborEncode":
      result.result.value.encoded = result.result.value.encoded.slice();
      break;
    case "deterministicCborDecode":
      detachGeneratedCborValueBytes(result.result.value.value);
      break;
    case undefined:
      break;
  }
};

const detachGeneratedCborValueBytes = (
  value: CodecDeterministicCborValue | undefined,
): void => {
  if (value === undefined) {
    return;
  }
  switch (value.value.case) {
    case "bytesValue":
      value.value.value.value = value.value.value.value.slice();
      break;
    case "arrayValue":
      for (const child of value.value.value.values) {
        detachGeneratedCborValueBytes(child);
      }
      break;
    case "mapValue":
      for (const entry of value.value.value.entries) {
        detachGeneratedCborValueBytes(entry.value);
      }
      break;
    case "nullValue":
    case "boolValue":
    case "integerValue":
    case "textValue":
    case undefined:
      break;
  }
};

/**
 * Clears mutable byte owners held by a generated operation result.
 *
 * Callers invoke this before rejecting an unexpected valid oneof variant and
 * after copying a sensitive result into its public SDK owner. JavaScript
 * strings remain subject to the managed-runtime memory model, but every
 * mutable `Uint8Array` created by this adapter is released promptly.
 */
export const clearGeneratedOperationResult = (
  result: CodecOperationResult,
): void => {
  switch (result.result.case) {
    case "multicodecPrefixForName":
      result.result.value.prefix.fill(0);
      break;
    case "multicodecLookupPrefix":
      result.result.value.metadata?.prefix.fill(0);
      break;
    case "multicodecTable":
      for (const entry of result.result.value.entries) {
        entry.prefix.fill(0);
      }
      break;
    case "multikeyParse":
      result.result.value.publicKey.fill(0);
      break;
    case "dagCborVerifyCid":
      break;
    case "dagCborEncode":
      result.result.value.encoded.fill(0);
      break;
    case "dagCborDecode":
      wipeGeneratedCborValue(result.result.value.value);
      break;
    case "pemDecode":
      result.result.value.der.fill(0);
      break;
    case "pemEncode":
      result.result.value.pem.fill(0);
      break;
    case "deterministicCborEncode":
      result.result.value.encoded.fill(0);
      break;
    case "deterministicCborDecode":
      wipeGeneratedCborValue(result.result.value.value);
      break;
    case undefined:
      break;
  }
  result.result = { case: undefined };
};

const wipeGeneratedCborValue = (
  value: CodecDeterministicCborValue | undefined,
): void => {
  if (value === undefined) {
    return;
  }
  switch (value.value.case) {
    case "bytesValue":
      value.value.value.value.fill(0);
      break;
    case "arrayValue":
      for (const child of value.value.value.values) {
        wipeGeneratedCborValue(child);
      }
      break;
    case "mapValue":
      for (const entry of value.value.value.entries) {
        wipeGeneratedCborValue(entry.value);
      }
      break;
    case "nullValue":
    case "boolValue":
    case "integerValue":
    case "textValue":
    case undefined:
      break;
  }
  value.value = { case: undefined };
};

const errorCodeForCodecErrorMessage = (
  error: CodecError,
): ReallyMeCodecErrorCode => {
  if (
    hasUnknownFields(error) ||
    (error.error.value !== undefined && hasUnknownFields(error.error.value))
  ) {
    return "provider-failure";
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
    return "provider-failure";
  }
  let expectedOrigin: CodecErrorOrigin;
  switch (error.error.case) {
    case "baseEncoding":
      if (reason < 100 || reason > 199) return "provider-failure";
      expectedOrigin = CodecErrorOrigin.CALLER;
      break;
    case "pem":
      if (reason < 200 || reason > 299) return "provider-failure";
      expectedOrigin = CodecErrorOrigin.CALLER;
      break;
    case "multiformat":
      if (reason < 300 || reason > 399) return "provider-failure";
      expectedOrigin = CodecErrorOrigin.CALLER;
      break;
    case "canonicalization":
      if (reason < 400 || reason > 499) return "provider-failure";
      expectedOrigin = reason === CodecErrorReason.CANONICAL_INTERNAL
        ? CodecErrorOrigin.PROVIDER
        : CodecErrorOrigin.CALLER;
      break;
    case "backend":
      if (reason < 500 || reason > 599) return "provider-failure";
      expectedOrigin = CodecErrorOrigin.PROVIDER;
      break;
    case "boundary":
      if (reason < 600 || reason > 699) return "provider-failure";
      expectedOrigin = CodecErrorOrigin.CALLER;
      break;
    case undefined:
      return "provider-failure";
  }
  if (error.origin !== expectedOrigin) {
    return "provider-failure";
  }
  return error.origin === CodecErrorOrigin.CALLER
    ? "invalid-input"
    : "provider-failure";
};
