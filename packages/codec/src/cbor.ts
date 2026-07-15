// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { ReallyMeCodecError } from "./errors.js";
import {
  ensureBytesInput,
  ensureStringInput,
  ensureStringValue,
  readBooleanProperty,
  readBytesOutput,
  readObjectOutput,
  readProtoResultOutput,
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

const isRecord = (value: unknown): value is Readonly<Record<string, unknown>> =>
  typeof value === "object" && value !== null && !Array.isArray(value);

const i64Min = -(1n << 63n);
const i64Max = (1n << 63n) - 1n;
const decimalIntegerPattern = /^-?(0|[1-9][0-9]*)$/u;
const maxCborContainerDepth = 128;

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
): ReallyMeCborValue => {
  if (!isRecord(value) || typeof value.type !== "string") {
    throw new ReallyMeCodecError("invalid-input");
  }
  if (seen.has(value)) {
    throw new ReallyMeCodecError("invalid-input");
  }
  seen.add(value);
  try {
    switch (value.type) {
      case "null":
        return { type: "null" };
      case "bool":
        if (typeof value.value !== "boolean") {
          throw new ReallyMeCodecError("invalid-input");
        }
        return { type: "bool", value: value.value };
      case "int":
        return { type: "int", value: validateCborInteger(value.value) };
      case "string":
      case "bytes":
        if (typeof value.value !== "string") {
          throw new ReallyMeCodecError("invalid-input");
        }
        return { type: value.type, value: value.value };
      case "array":
        if (!Array.isArray(value.value)) {
          throw new ReallyMeCodecError("invalid-input");
        }
        return {
          type: "array",
          value: value.value.map((entry: unknown): ReallyMeCborValue =>
            validateCborValue(entry, childContainerDepth(depth), seen),
          ),
        };
      case "map":
        if (!Array.isArray(value.value)) {
          throw new ReallyMeCodecError("invalid-input");
        }
        return {
          type: "map",
          value: value.value.map((entry: unknown): ReallyMeCborMapEntry => {
            if (!isRecord(entry) || typeof entry.key !== "string") {
              throw new ReallyMeCodecError("invalid-input");
            }
            return {
              key: entry.key,
              value: validateCborValue(entry.value, childContainerDepth(depth), seen),
            };
          }),
        };
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
  const parsed: unknown = JSON.parse(json);
  return validateCborValue(parsed);
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
  const normalized = validateCborValue(value);
  return readBytesOutput(
    requireReallyMeCodecWasmProvider().dagCborEncode(JSON.stringify(cborValueForJson(normalized))),
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
  ensureStringValue(cid);
  ensureBytesInput(bytes);
  return readBytesOutput(
    requireReallyMeCodecWasmProvider().dagCborVerifyCidProto(cid, bytes),
  );
};

export const dagCborVerifyCidProtoResult = (
  cid: string,
  bytes: Uint8Array,
): ReallyMeCodecProtoResult => {
  ensureStringValue(cid);
  ensureBytesInput(bytes);
  return readProtoResultOutput(
    requireReallyMeCodecWasmProvider().dagCborVerifyCidProtoResult(cid, bytes),
  );
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
