// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { create } from "@bufbuild/protobuf";

import {
  MAX_CODEC_BOUNDARY_NODES,
  stringifyBoundaryJson,
} from "./boundary.js";
import { ReallyMeCodecError } from "./errors.js";
import {
  CodecDagCborVerifyCidRequestSchema,
  CodecOperationRequestSchema,
} from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import {
  processGeneratedProtoRequest,
  protoPayloadOrThrow,
} from "./protoProcess.js";
import {
  ensureBytesInput,
  ensureStringInput,
  ensureStringValue,
  readBooleanProperty,
  readBytesOutput,
  readObjectOutput,
  readStringOutput,
  readStringProperty,
  readStringValueProperty,
  readNumberOutput,
} from "./readOutput.js";
import type { ReallyMeCodecProtoResult } from "./readOutput.js";
import { requireReallyMeCodecWasmProvider } from "./wasmProvider.js";

export type ReallyMeCborValue =
  | Readonly<{ type: "null" }>
  | Readonly<{ type: "bool"; value: boolean }>
  | Readonly<{ type: "int"; value: number | bigint }>
  | Readonly<{ type: "string"; value: string }>
  | Readonly<{ type: "bytes"; value: string }>
  | Readonly<{ type: "array"; value: ReadonlyArray<ReallyMeCborValue> }>
  | Readonly<{ type: "map"; value: ReadonlyArray<ReallyMeCborMapEntry> }>;

export type ReallyMeCborMapEntry = Readonly<{
  key: string;
  value: ReallyMeCborValue;
}>;

export type ReallyMeDagCborCidVerification = Readonly<{
  valid: boolean;
  expectedCid: string;
  actualCid: string;
}>;

const invalidCborInput = (): never => {
  throw new ReallyMeCodecError("invalid-input");
};

const isRecord = (value: unknown): value is object => {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    return false;
  }
  const prototype = Object.getPrototypeOf(value);
  return prototype === Object.prototype || prototype === null;
};

const readDataProperty = (value: object, name: string): unknown => {
  const descriptor = Object.getOwnPropertyDescriptor(value, name);
  if (descriptor === undefined || !("value" in descriptor)) {
    return invalidCborInput();
  }
  return descriptor.value;
};

const snapshotArray = (value: unknown): ReadonlyArray<unknown> => {
  if (!Array.isArray(value) || Object.getPrototypeOf(value) !== Array.prototype) {
    return invalidCborInput();
  }
  const rawLength = readDataProperty(value, "length");
  if (typeof rawLength !== "number") {
    return invalidCborInput();
  }
  const length = rawLength;
  if (
    !Number.isSafeInteger(length) ||
    length < 0 ||
    length > MAX_CODEC_BOUNDARY_NODES
  ) {
    return invalidCborInput();
  }
  const entries: unknown[] = [];
  for (let index = 0; index < length; index += 1) {
    const descriptor = Object.getOwnPropertyDescriptor(value, String(index));
    if (descriptor === undefined || !("value" in descriptor)) {
      return invalidCborInput();
    }
    entries.push(descriptor.value);
  }
  return entries;
};

const i64Min = -(1n << 63n);
const i64Max = (1n << 63n) - 1n;
const decimalIntegerPattern = /^-?(0|[1-9][0-9]*)$/u;
const maxCborContainerDepth = 128;

type CborValidationState = {
  nodes: number;
};

const childContainerDepth = (depth: number): number => {
  const nextDepth = depth + 1;
  if (nextDepth > maxCborContainerDepth) {
    throw new ReallyMeCodecError("invalid-input");
  }
  return nextDepth;
};

const validateCborInteger = (value: unknown): number | bigint => {
  if (typeof value === "number") {
    if (!Number.isSafeInteger(value)) {
      throw new ReallyMeCodecError("invalid-input");
    }
    return value;
  }
  if (typeof value === "bigint") {
    if (value < i64Min || value > i64Max) {
      throw new ReallyMeCodecError("invalid-input");
    }
    return value;
  }
  if (typeof value === "string") {
    if (!decimalIntegerPattern.test(value)) {
      throw new ReallyMeCodecError("invalid-input");
    }
    const parsed = BigInt(value);
    if (parsed < i64Min || parsed > i64Max) {
      throw new ReallyMeCodecError("invalid-input");
    }
    return parsed;
  }
  throw new ReallyMeCodecError("invalid-input");
};

const validateCborValue = (
  value: unknown,
  depth = 0,
  seen: WeakSet<object> = new WeakSet<object>(),
  state: CborValidationState = { nodes: 0 },
): ReallyMeCborValue => {
  if (state.nodes >= MAX_CODEC_BOUNDARY_NODES) {
    throw new ReallyMeCodecError("invalid-input");
  }
  state.nodes += 1;
  if (!isRecord(value)) {
    throw new ReallyMeCodecError("invalid-input");
  }
  const type = readDataProperty(value, "type");
  if (typeof type !== "string") {
    throw new ReallyMeCodecError("invalid-input");
  }
  if (seen.has(value)) {
    throw new ReallyMeCodecError("invalid-input");
  }
  seen.add(value);
  try {
    switch (type) {
      case "null":
        return { type: "null" };
      case "bool": {
        const child = readDataProperty(value, "value");
        if (typeof child !== "boolean") {
          throw new ReallyMeCodecError("invalid-input");
        }
        return { type: "bool", value: child };
      }
      case "int":
        return {
          type: "int",
          value: validateCborInteger(readDataProperty(value, "value")),
        };
      case "string":
      case "bytes": {
        const child = readDataProperty(value, "value");
        if (typeof child !== "string") {
          throw new ReallyMeCodecError("invalid-input");
        }
        return { type, value: child };
      }
      case "array": {
        const children = snapshotArray(readDataProperty(value, "value"));
        if (children.length > MAX_CODEC_BOUNDARY_NODES - state.nodes) {
          throw new ReallyMeCodecError("invalid-input");
        }
        return {
          type: "array",
          value: children.map((entry: unknown): ReallyMeCborValue =>
            validateCborValue(entry, childContainerDepth(depth), seen, state),
          ),
        };
      }
      case "map": {
        const entries = snapshotArray(readDataProperty(value, "value"));
        if (entries.length > MAX_CODEC_BOUNDARY_NODES - state.nodes) {
          throw new ReallyMeCodecError("invalid-input");
        }
        state.nodes += entries.length;
        const keys = new Set<string>();
        return {
          type: "map",
          value: entries.map((entry: unknown): ReallyMeCborMapEntry => {
            if (!isRecord(entry)) {
              throw new ReallyMeCodecError("invalid-input");
            }
            const key = readDataProperty(entry, "key");
            if (typeof key !== "string" || keys.has(key)) {
              throw new ReallyMeCodecError("invalid-input");
            }
            keys.add(key);
            return {
              key,
              value: validateCborValue(
                readDataProperty(entry, "value"),
                childContainerDepth(depth),
                seen,
                state,
              ),
            };
          }),
        };
      }
      default:
        throw new ReallyMeCodecError("invalid-input");
    }
  } finally {
    seen.delete(value);
  }
};

const cborValueForJson = (value: ReallyMeCborValue): unknown => {
  switch (value.type) {
    case "null":
      return { type: "null" };
    case "bool":
    case "string":
    case "bytes":
      return value;
    case "int":
      return {
        type: "int",
        value: typeof value.value === "bigint" ? value.value.toString() : value.value,
      };
    case "array":
      return { type: "array", value: value.value.map(cborValueForJson) };
    case "map":
      return {
        type: "map",
        value: value.value.map((entry) => ({
          key: entry.key,
          value: cborValueForJson(entry.value),
        })),
      };
  }
};

const readCborValue = (json: string): ReallyMeCborValue => {
  try {
    const parsed: unknown = JSON.parse(json);
    return validateCborValue(parsed);
  } catch (_error: unknown) {
    throw new ReallyMeCodecError("provider-failure");
  }
};

const readCidVerification = (value: unknown): ReallyMeDagCborCidVerification => {
  const object = readObjectOutput(value);
  return {
    valid: readBooleanProperty(object, "valid"),
    expectedCid: readStringProperty(object, "expectedCid"),
    actualCid: readStringValueProperty(object, "actualCid"),
  };
};

export const dagCborEncode = (value: ReallyMeCborValue): Uint8Array => {
  let normalized: ReallyMeCborValue;
  try {
    normalized = validateCborValue(value);
  } catch (error: unknown) {
    if (error instanceof ReallyMeCodecError) {
      throw error;
    }
    return invalidCborInput();
  }
  return readBytesOutput(
    requireReallyMeCodecWasmProvider().dagCborEncode(
      stringifyBoundaryJson(cborValueForJson(normalized)),
    ),
  );
};

export const dagCborDecode = (bytes: Uint8Array): ReallyMeCborValue => {
  ensureBytesInput(bytes);
  return readCborValue(readStringOutput(requireReallyMeCodecWasmProvider().dagCborDecode(bytes)));
};

export const dagCborComputeCid = (bytes: Uint8Array): string => {
  ensureBytesInput(bytes);
  return readStringOutput(requireReallyMeCodecWasmProvider().dagCborComputeCid(bytes));
};

export const dagCborVerifyCid = (
  cid: string,
  bytes: Uint8Array,
): ReallyMeDagCborCidVerification => {
  ensureStringValue(cid);
  ensureBytesInput(bytes);
  return readCidVerification(requireReallyMeCodecWasmProvider().dagCborVerifyCid(cid, bytes));
};

export const dagCborVerifyCidProto = (cid: string, bytes: Uint8Array): Uint8Array => {
  return protoPayloadOrThrow(dagCborVerifyCidProtoResult(cid, bytes));
};

export const dagCborVerifyCidProtoResult = (
  cid: string,
  bytes: Uint8Array,
): ReallyMeCodecProtoResult => {
  ensureStringValue(cid);
  ensureBytesInput(bytes);
  return processGeneratedProtoRequest(create(CodecOperationRequestSchema, {
    operation: {
      case: "dagCborVerifyCid",
      value: create(CodecDagCborVerifyCidRequestSchema, {
        cid,
        payload: bytes,
      }),
    },
  }));
};

export const dagCborSha256ContentHash = (bytes: Uint8Array): Uint8Array => {
  ensureBytesInput(bytes);
  return readBytesOutput(requireReallyMeCodecWasmProvider().dagCborSha256ContentHash(bytes));
};

export const dagCborMultihash = (bytes: Uint8Array): Uint8Array => {
  ensureBytesInput(bytes);
  return readBytesOutput(requireReallyMeCodecWasmProvider().dagCborMultihash(bytes));
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
  ensureStringValue(cid);
  const value = requireReallyMeCodecWasmProvider().tryParseCid(cid);
  if (value === undefined) {
    return undefined;
  }
  return readStringOutput(value);
};

export const dagCborCodecCode = (): number =>
  readNumberOutput(requireReallyMeCodecWasmProvider().dagCborCodecCode());
