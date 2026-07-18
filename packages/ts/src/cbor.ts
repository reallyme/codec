// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { create } from "@bufbuild/protobuf";
import type { Message } from "@bufbuild/protobuf";

import {
  MAX_CODEC_BOUNDARY_NODES,
  strictUtf8ByteLength,
} from "./boundary.js";
import {
  DETERMINISTIC_CBOR_I64_MIN,
  DETERMINISTIC_CBOR_NEGATIVE_MAX,
  DETERMINISTIC_CBOR_U64_MAX,
  MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
  MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES,
  MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES,
  MAX_DETERMINISTIC_CBOR_INPUT_LEN,
  MAX_DETERMINISTIC_CBOR_NESTING_DEPTH,
  MAX_DETERMINISTIC_CBOR_NODES,
  MAX_DETERMINISTIC_CBOR_OUTPUT_LEN,
} from "./deterministicCborBoundary.js";
import { ReallyMeCodecError } from "./errors.js";
import {
  CodecDagCborDecodeRequestSchema,
  CodecDagCborEncodeRequestSchema,
  CodecDagCborVerifyCidRequestSchema,
  CodecDeterministicCborArraySchema,
  CodecDeterministicCborBoolSchema,
  CodecDeterministicCborBytesSchema,
  CodecDeterministicCborDecodeRequestSchema,
  CodecDeterministicCborEncodeRequestSchema,
  CodecDeterministicCborIntegerSchema,
  CodecDeterministicCborMapEntrySchema,
  CodecDeterministicCborMapKeySchema,
  CodecDeterministicCborMapSchema,
  CodecDeterministicCborNegativeIntegerSchema,
  CodecDeterministicCborNullSchema,
  CodecDeterministicCborTextSchema,
  CodecDeterministicCborUnsignedIntegerSchema,
  CodecDeterministicCborValueSchema,
  CodecOperationRequestSchema,
} from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import type {
  CodecDeterministicCborInteger,
  CodecDeterministicCborMapEntry,
  CodecDeterministicCborMapKey,
  CodecDeterministicCborValue,
} from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import {
  clearGeneratedOperationResult,
  processGeneratedOperationRequest,
} from "./operationContract.js";
import {
  ensureBytesInput,
  ensureStringValue,
  readBytesOutput,
  readStringOutput,
  readNumberOutput,
} from "./readOutput.js";
import { requireReallyMeCodecWasmProvider } from "./wasmProvider.js";

export type ReallyMeCborValue =
  | Readonly<{ type: "null" }>
  | Readonly<{ type: "bool"; value: boolean }>
  | Readonly<{ type: "int"; value: number | bigint }>
  | Readonly<{ type: "string"; value: string }>
  | Readonly<{ type: "bytes"; value: Uint8Array }>
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

export type ReallyMeDeterministicCborInteger =
  | Readonly<{ type: "unsigned"; value: bigint }>
  | Readonly<{ type: "negative"; value: bigint }>;

export type ReallyMeDeterministicCborMapKey =
  | Readonly<{ type: "integer"; value: ReallyMeDeterministicCborInteger }>
  | Readonly<{ type: "text"; value: string }>;

export type ReallyMeDeterministicCborMapEntry = Readonly<{
  key: ReallyMeDeterministicCborMapKey;
  value: ReallyMeDeterministicCborValue;
}>;

export type ReallyMeDeterministicCborValue =
  | Readonly<{ type: "null" }>
  | Readonly<{ type: "bool"; value: boolean }>
  | Readonly<{ type: "integer"; value: ReallyMeDeterministicCborInteger }>
  | Readonly<{ type: "text"; value: string }>
  | Readonly<{ type: "bytes"; value: Uint8Array }>
  | Readonly<{
      type: "array";
      value: ReadonlyArray<ReallyMeDeterministicCborValue>;
    }>
  | Readonly<{
      type: "map";
      value: ReadonlyArray<ReallyMeDeterministicCborMapEntry>;
    }>;

export const ReallyMeDeterministicCbor = {
  null(): ReallyMeDeterministicCborValue {
    return { type: "null" };
  },

  bool(value: boolean): ReallyMeDeterministicCborValue {
    if (typeof value !== "boolean") {
      return invalidCborInput();
    }
    return { type: "bool", value };
  },

  unsigned(value: bigint): ReallyMeDeterministicCborValue {
    if (typeof value !== "bigint" || value < 0n || value > DETERMINISTIC_CBOR_U64_MAX) {
      return invalidCborInput();
    }
    return { type: "integer", value: { type: "unsigned", value } };
  },

  negative(value: bigint): ReallyMeDeterministicCborValue {
    if (
      typeof value !== "bigint" ||
      value < DETERMINISTIC_CBOR_I64_MIN ||
      value > DETERMINISTIC_CBOR_NEGATIVE_MAX
    ) {
      return invalidCborInput();
    }
    return { type: "integer", value: { type: "negative", value } };
  },

  text(value: string): ReallyMeDeterministicCborValue {
    ensureStringValue(value);
    return { type: "text", value };
  },

  bytes(value: Uint8Array): ReallyMeDeterministicCborValue {
    ensureBytesInput(value);
    return { type: "bytes", value: value.slice() };
  },

  array(
    value: ReadonlyArray<ReallyMeDeterministicCborValue>,
  ): ReallyMeDeterministicCborValue {
    return { type: "array", value: [...value] };
  },

  mapInt(
    entries: ReadonlyArray<readonly [bigint, ReallyMeDeterministicCborValue]>,
  ): ReallyMeDeterministicCborValue {
    const seen = new Set<bigint>();
    return {
      type: "map",
      value: entries.map(([key, value]): ReallyMeDeterministicCborMapEntry => {
        if (typeof key !== "bigint" || key < 0n || key > DETERMINISTIC_CBOR_U64_MAX) {
          return invalidCborInput();
        }
        if (seen.has(key)) {
          return invalidCborInput();
        }
        seen.add(key);
        return {
          key: { type: "integer", value: { type: "unsigned", value: key } },
          value,
        };
      }),
    };
  },

  mapText(
    entries: ReadonlyArray<readonly [string, ReallyMeDeterministicCborValue]>,
  ): ReallyMeDeterministicCborValue {
    const seen = new Set<string>();
    return {
      type: "map",
      value: entries.map(([key, value]): ReallyMeDeterministicCborMapEntry => {
        ensureStringValue(key);
        if (seen.has(key)) {
          return invalidCborInput();
        }
        seen.add(key);
        return { key: { type: "text", value: key }, value };
      }),
    };
  },

  intKey(value: bigint): ReallyMeDeterministicCborMapKey {
    if (typeof value !== "bigint") {
      return invalidCborInput();
    }
    if (value >= 0n && value <= DETERMINISTIC_CBOR_U64_MAX) {
      return { type: "integer", value: { type: "unsigned", value } };
    }
    if (value >= DETERMINISTIC_CBOR_I64_MIN && value <= DETERMINISTIC_CBOR_NEGATIVE_MAX) {
      return { type: "integer", value: { type: "negative", value } };
    }
    return invalidCborInput();
  },

  textKey(value: string): ReallyMeDeterministicCborMapKey {
    ensureStringValue(value);
    return { type: "text", value };
  },

  entry(
    key: ReallyMeDeterministicCborMapKey,
    value: ReallyMeDeterministicCborValue,
  ): ReallyMeDeterministicCborMapEntry {
    return { key, value };
  },
} as const;

export const ReallyMeDagCbor = {
  null(): ReallyMeCborValue {
    return { type: "null" };
  },

  bool(value: boolean): ReallyMeCborValue {
    if (typeof value !== "boolean") {
      return invalidCborInput();
    }
    return { type: "bool", value };
  },

  int(value: number | bigint): ReallyMeCborValue {
    return { type: "int", value: validateCborInteger(value) };
  },

  unsigned(value: number | bigint): ReallyMeCborValue {
    const integer = validateCborInteger(value);
    if (
      (typeof integer === "bigint" && integer < 0n) ||
      (typeof integer === "number" && integer < 0)
    ) {
      return invalidCborInput();
    }
    return { type: "int", value: integer };
  },

  negative(value: number | bigint): ReallyMeCborValue {
    const integer = validateCborInteger(value);
    if (
      (typeof integer === "bigint" && integer >= 0n) ||
      (typeof integer === "number" && integer >= 0)
    ) {
      return invalidCborInput();
    }
    return { type: "int", value: integer };
  },

  text(value: string): ReallyMeCborValue {
    ensureStringValue(value);
    return { type: "string", value };
  },

  bytes(value: Uint8Array): ReallyMeCborValue {
    ensureBytesInput(value);
    return { type: "bytes", value: value.slice() };
  },

  array(value: ReadonlyArray<ReallyMeCborValue>): ReallyMeCborValue {
    return { type: "array", value: [...value] };
  },

  mapText(
    entries: ReadonlyArray<readonly [string, ReallyMeCborValue]>,
  ): ReallyMeCborValue {
    const seen = new Set<string>();
    return {
      type: "map",
      value: entries.map(([key, value]): ReallyMeCborMapEntry => {
        ensureStringValue(key);
        if (seen.has(key)) {
          return invalidCborInput();
        }
        seen.add(key);
        return { key, value };
      }),
    };
  },
} as const;

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

const snapshotArray = (
  value: unknown,
  maximumLength = MAX_CODEC_BOUNDARY_NODES,
): ReadonlyArray<unknown> => {
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
    length > maximumLength
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
const maxDagCborInputLength = 1024 * 1024;

type CborValidationState = {
  nodes: number;
  byteStringBytes: number;
};

const wipeDagCborValueBytes = (value: ReallyMeCborValue | undefined): void => {
  if (value === undefined) {
    return;
  }
  switch (value.type) {
    case "bytes":
      value.value.fill(0);
      break;
    case "array":
      for (const child of value.value) {
        wipeDagCborValueBytes(child);
      }
      break;
    case "map":
      for (const entry of value.value) {
        wipeDagCborValueBytes(entry.value);
      }
      break;
    case "null":
    case "bool":
    case "int":
    case "string":
      break;
  }
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
  state: CborValidationState = { nodes: 0, byteStringBytes: 0 },
): ReallyMeCborValue => {
  if (state.nodes >= MAX_DETERMINISTIC_CBOR_NODES) {
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
      case "string": {
        const child = readDataProperty(value, "value");
        if (typeof child !== "string") {
          throw new ReallyMeCodecError("invalid-input");
        }
        return { type: "string", value: child };
      }
      case "bytes": {
        const child = readDataProperty(value, "value");
        if (!(child instanceof Uint8Array)) {
          throw new ReallyMeCodecError("invalid-input");
        }
        ensureBytesInput(child);
        if (
          child.length >
          MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES - state.byteStringBytes
        ) {
          throw new ReallyMeCodecError("invalid-input");
        }
        state.byteStringBytes += child.length;
        return { type: "bytes", value: child.slice() };
      }
      case "array": {
        const children = snapshotArray(
          readDataProperty(value, "value"),
          MAX_DETERMINISTIC_CBOR_NODES - state.nodes,
        );
        const normalizedChildren: ReallyMeCborValue[] = [];
        try {
          for (const entry of children) {
            normalizedChildren.push(
              validateCborValue(entry, childContainerDepth(depth), seen, state),
            );
          }
          return {
            type: "array",
            value: normalizedChildren,
          };
        } catch (error: unknown) {
          for (const child of normalizedChildren) {
            wipeDagCborValueBytes(child);
          }
          throw error;
        }
      }
      case "map": {
        const entries = snapshotArray(
          readDataProperty(value, "value"),
          MAX_DETERMINISTIC_CBOR_NODES - state.nodes,
        );
        state.nodes += entries.length;
        const keys = new Set<string>();
        const normalizedEntries: ReallyMeCborMapEntry[] = [];
        try {
          for (const entry of entries) {
            if (!isRecord(entry)) {
              throw new ReallyMeCodecError("invalid-input");
            }
            const key = readDataProperty(entry, "key");
            if (typeof key !== "string" || keys.has(key)) {
              throw new ReallyMeCodecError("invalid-input");
            }
            keys.add(key);
            normalizedEntries.push({
              key,
              value: validateCborValue(
                readDataProperty(entry, "value"),
                childContainerDepth(depth),
                seen,
                state,
              ),
            });
          }
          return {
            type: "map",
            value: normalizedEntries,
          };
        } catch (error: unknown) {
          for (const entry of normalizedEntries) {
            wipeDagCborValueBytes(entry.value);
          }
          throw error;
        }
      }
      default:
        throw new ReallyMeCodecError("invalid-input");
    }
  } finally {
    seen.delete(value);
  }
};

type DeterministicCborValidationState = {
  nodes: number;
  textBytes: number;
  byteStringBytes: number;
  ownedBytes: Uint8Array[];
};

type DeterministicCborMapKeySet = {
  integerKeys: Set<bigint>;
  textKeys: Set<string>;
};

const deterministicCborMapKeySet = (): DeterministicCborMapKeySet => ({
  integerKeys: new Set<bigint>(),
  textKeys: new Set<string>(),
});

const recordDeterministicCborMapKey = (
  key: ReallyMeDeterministicCborMapKey,
  keys: DeterministicCborMapKeySet,
  duplicate: () => never,
): void => {
  switch (key.type) {
    case "integer":
      // The supported positive and negative domains do not overlap, so the
      // bigint itself is the exact semantic integer-key identity.
      if (keys.integerKeys.has(key.value.value)) {
        duplicate();
      }
      keys.integerKeys.add(key.value.value);
      break;
    case "text":
      // Valid JavaScript scalar strings compare equal exactly when their
      // UTF-8 encodings do. Deliberately do not normalize or locale-fold PII.
      if (keys.textKeys.has(key.value)) {
        duplicate();
      }
      keys.textKeys.add(key.value);
      break;
  }
};

const deterministicCborValidationState =
  (): DeterministicCborValidationState => ({
    nodes: 0,
    textBytes: 0,
    byteStringBytes: 0,
    ownedBytes: [],
  });

const wipeOwnedByteSnapshots = (
  state: Pick<DeterministicCborValidationState, "ownedBytes">,
): void => {
  for (const value of state.ownedBytes) {
    value.fill(0);
  }
};

const consumeDeterministicCborBudget = (
  current: number,
  amount: number,
  maximum: number,
): number => {
  if (
    !Number.isSafeInteger(amount) ||
    amount < 0 ||
    current > maximum - amount
  ) {
    return invalidCborInput();
  }
  return current + amount;
};

const consumeDeterministicCborText = (
  state: DeterministicCborValidationState,
  value: string,
): void => {
  // Every UTF-16 code unit requires at least one UTF-8 byte. Reject an
  // impossible aggregate before walking an attacker-sized managed string.
  if (
    state.textBytes >
    MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES - value.length
  ) {
    invalidCborInput();
  }
  state.textBytes = consumeDeterministicCborBudget(
    state.textBytes,
    strictUtf8ByteLength(value),
    MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES,
  );
};

const requireDeterministicCborObject = (
  value: unknown,
  allowedKeys: ReadonlyArray<string>,
): Readonly<{
  owner: object;
  properties: ReadonlyMap<string, unknown>;
}> => {
  if (!isRecord(value)) {
    return invalidCborInput();
  }
  const properties = new Map<string, unknown>();
  for (const key of Reflect.ownKeys(value)) {
    if (typeof key !== "string" || !allowedKeys.includes(key)) {
      return invalidCborInput();
    }
    const descriptor = Object.getOwnPropertyDescriptor(value, key);
    if (descriptor === undefined || !("value" in descriptor)) {
      return invalidCborInput();
    }
    if (properties.has(key)) {
      return invalidCborInput();
    }
    properties.set(key, descriptor.value);
  }
  return { owner: value, properties };
};

const readDeterministicCborDataProperty = (
  properties: ReadonlyMap<string, unknown>,
  key: string,
): unknown => {
  if (!properties.has(key)) {
    return invalidCborInput();
  }
  return properties.get(key);
};

const snapshotDeterministicCborArray = (
  value: unknown,
): ReadonlyArray<unknown> => {
  if (!Array.isArray(value) || Object.getPrototypeOf(value) !== Array.prototype) {
    return invalidCborInput();
  }
  const properties = new Map<string, unknown>();
  for (const key of Reflect.ownKeys(value)) {
    if (typeof key !== "string") {
      return invalidCborInput();
    }
    const descriptor = Object.getOwnPropertyDescriptor(value, key);
    if (descriptor === undefined || !("value" in descriptor) || properties.has(key)) {
      return invalidCborInput();
    }
    properties.set(key, descriptor.value);
  }
  const rawLength = readDeterministicCborDataProperty(properties, "length");
  if (typeof rawLength !== "number") {
    return invalidCborInput();
  }
  const length = rawLength;
  if (
    !Number.isSafeInteger(length) ||
    length < 0 ||
    length > MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES
  ) {
    return invalidCborInput();
  }
  if (properties.size !== length + 1) {
    return invalidCborInput();
  }
  const entries: unknown[] = [];
  for (let index = 0; index < length; index += 1) {
    entries.push(
      readDeterministicCborDataProperty(properties, String(index)),
    );
  }
  return entries;
};

const uint8ArraySubarray = Uint8Array.prototype.subarray;
const uint8ArraySet = Uint8Array.prototype.set;

/**
 * Copies one bounded, fixed-length view and then revalidates the source view.
 * This avoids allocating from a second attacker-controlled length observation
 * when a length-tracking resizable buffer changes during validation.
 */
const snapshotDeterministicCborBytes = (
  value: Uint8Array,
  maximum: number,
): Uint8Array => {
  const expectedLength = value.length;
  if (expectedLength > maximum) {
    return invalidCborInput();
  }
  let snapshot: Uint8Array | undefined;
  try {
    const boundedView = uint8ArraySubarray.call(value, 0, expectedLength);
    if (boundedView.length !== expectedLength) {
      return invalidCborInput();
    }
    snapshot = new Uint8Array(expectedLength);
    uint8ArraySet.call(snapshot, boundedView, 0);
    if (
      value.length !== expectedLength ||
      boundedView.length !== expectedLength ||
      snapshot.length !== expectedLength
    ) {
      snapshot.fill(0);
      return invalidCborInput();
    }
    return snapshot;
  } catch (error: unknown) {
    snapshot?.fill(0);
    if (error instanceof ReallyMeCodecError) {
      throw error;
    }
    return invalidCborInput();
  }
};

const requireDeterministicCborDepth = (depth: number): number => {
  const nextDepth = depth + 1;
  if (nextDepth > MAX_DETERMINISTIC_CBOR_NESTING_DEPTH) {
    return invalidCborInput();
  }
  return nextDepth;
};

const validateDeterministicCborInteger = (
  value: unknown,
): ReallyMeDeterministicCborInteger => {
  const object = requireDeterministicCborObject(value, ["type", "value"]);
  const type = readDeterministicCborDataProperty(object.properties, "type");
  const rawValue = readDeterministicCborDataProperty(
    object.properties,
    "value",
  );
  if (typeof type !== "string" || typeof rawValue !== "bigint") {
    return invalidCborInput();
  }
  switch (type) {
    case "unsigned":
      if (rawValue < 0n || rawValue > DETERMINISTIC_CBOR_U64_MAX) {
        return invalidCborInput();
      }
      return { type: "unsigned", value: rawValue };
    case "negative":
      if (
        rawValue < DETERMINISTIC_CBOR_I64_MIN ||
        rawValue > DETERMINISTIC_CBOR_NEGATIVE_MAX
      ) {
        return invalidCborInput();
      }
      return { type: "negative", value: rawValue };
    default:
      return invalidCborInput();
  }
};

const validateDeterministicCborMapKey = (
  value: unknown,
  state: DeterministicCborValidationState,
): ReallyMeDeterministicCborMapKey => {
  state.nodes = consumeDeterministicCborBudget(
    state.nodes,
    1,
    MAX_DETERMINISTIC_CBOR_NODES,
  );
  const object = requireDeterministicCborObject(value, ["type", "value"]);
  const type = readDeterministicCborDataProperty(object.properties, "type");
  if (typeof type !== "string") {
    return invalidCborInput();
  }
  switch (type) {
    case "integer":
      return {
        type: "integer",
        value: validateDeterministicCborInteger(
          readDeterministicCborDataProperty(object.properties, "value"),
        ),
      };
    case "text": {
      const text = readDeterministicCborDataProperty(
        object.properties,
        "value",
      );
      if (typeof text !== "string") {
        return invalidCborInput();
      }
      consumeDeterministicCborText(state, text);
      return { type: "text", value: text };
    }
    default:
      return invalidCborInput();
  }
};

const validateDeterministicCborValue = (
  value: unknown,
  depth: number,
  seen: WeakSet<object>,
  state: DeterministicCborValidationState,
): ReallyMeDeterministicCborValue => {
  state.nodes = consumeDeterministicCborBudget(
    state.nodes,
    1,
    MAX_DETERMINISTIC_CBOR_NODES,
  );
  const object = requireDeterministicCborObject(value, ["type", "value"]);
  const type = readDeterministicCborDataProperty(object.properties, "type");
  if (typeof type !== "string" || seen.has(object.owner)) {
    return invalidCborInput();
  }
  seen.add(object.owner);
  try {
    switch (type) {
      case "null":
        if (object.properties.has("value")) {
          return invalidCborInput();
        }
        return { type: "null" };
      case "bool": {
        const child = readDeterministicCborDataProperty(
          object.properties,
          "value",
        );
        if (typeof child !== "boolean") {
          return invalidCborInput();
        }
        return { type: "bool", value: child };
      }
      case "integer":
        return {
          type: "integer",
          value: validateDeterministicCborInteger(
            readDeterministicCborDataProperty(object.properties, "value"),
          ),
        };
      case "text": {
        const child = readDeterministicCborDataProperty(
          object.properties,
          "value",
        );
        if (typeof child !== "string") {
          return invalidCborInput();
        }
        consumeDeterministicCborText(state, child);
        return { type: "text", value: child };
      }
      case "bytes": {
        const child = readDeterministicCborDataProperty(
          object.properties,
          "value",
        );
        if (!(child instanceof Uint8Array)) {
          return invalidCborInput();
        }
        const copy = snapshotDeterministicCborBytes(
          child,
          MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
        );
        // Register ownership before any subsequent budget check can fail so
        // the outer cleanup scope wipes this snapshot on every error path.
        state.ownedBytes.push(copy);
        state.byteStringBytes = consumeDeterministicCborBudget(
          state.byteStringBytes,
          copy.length,
          MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
        );
        return { type: "bytes", value: copy };
      }
      case "array": {
        const children = snapshotDeterministicCborArray(
          readDeterministicCborDataProperty(object.properties, "value"),
        );
        return {
          type: "array",
          value: children.map(
            (entry: unknown): ReallyMeDeterministicCborValue =>
              validateDeterministicCborValue(
                entry,
                requireDeterministicCborDepth(depth),
                seen,
                state,
              ),
          ),
        };
      }
      case "map": {
        const entries = snapshotDeterministicCborArray(
          readDeterministicCborDataProperty(object.properties, "value"),
        );
        const keys = deterministicCborMapKeySet();
        return {
          type: "map",
          value: entries.map(
            (entry: unknown): ReallyMeDeterministicCborMapEntry => {
              const mapEntry = requireDeterministicCborObject(entry, [
                "key",
                "value",
              ]);
              const key = validateDeterministicCborMapKey(
                readDeterministicCborDataProperty(
                  mapEntry.properties,
                  "key",
                ),
                state,
              );
              recordDeterministicCborMapKey(key, keys, invalidCborInput);
              return {
                key,
                value: validateDeterministicCborValue(
                  readDeterministicCborDataProperty(
                    mapEntry.properties,
                    "value",
                  ),
                  requireDeterministicCborDepth(depth),
                  seen,
                  state,
                ),
              };
            },
          ),
        };
      }
      default:
        return invalidCborInput();
    }
  } finally {
    seen.delete(object.owner);
  }
};

const normalizeDeterministicCborValue = (
  value: unknown,
): ReallyMeDeterministicCborValue => {
  const state = deterministicCborValidationState();
  try {
    return validateDeterministicCborValue(value, 0, new WeakSet<object>(), state);
  } catch (error: unknown) {
    wipeOwnedByteSnapshots(state);
    if (error instanceof ReallyMeCodecError) {
      throw error;
    }
    return invalidCborInput();
  }
};

const deterministicCborIntegerToProto = (
  integer: ReallyMeDeterministicCborInteger,
): CodecDeterministicCborInteger => {
  switch (integer.type) {
    case "unsigned":
      return create(CodecDeterministicCborIntegerSchema, {
        value: {
          case: "unsignedValue",
          value: create(CodecDeterministicCborUnsignedIntegerSchema, {
            value: integer.value,
          }),
        },
      });
    case "negative":
      return create(CodecDeterministicCborIntegerSchema, {
        value: {
          case: "negativeValue",
          value: create(CodecDeterministicCborNegativeIntegerSchema, {
            value: integer.value,
          }),
        },
      });
  }
};

const deterministicCborMapKeyToProto = (
  key: ReallyMeDeterministicCborMapKey,
): CodecDeterministicCborMapKey => {
  switch (key.type) {
    case "integer":
      return create(CodecDeterministicCborMapKeySchema, {
        key: {
          case: "integerKey",
          value: deterministicCborIntegerToProto(key.value),
        },
      });
    case "text":
      return create(CodecDeterministicCborMapKeySchema, {
        key: {
          case: "textKey",
          value: create(CodecDeterministicCborTextSchema, { value: key.value }),
        },
      });
  }
};

const deterministicCborValueToProto = (
  value: ReallyMeDeterministicCborValue,
): CodecDeterministicCborValue => {
  switch (value.type) {
    case "null":
      return create(CodecDeterministicCborValueSchema, {
        value: {
          case: "nullValue",
          value: create(CodecDeterministicCborNullSchema),
        },
      });
    case "bool":
      return create(CodecDeterministicCborValueSchema, {
        value: {
          case: "boolValue",
          value: create(CodecDeterministicCborBoolSchema, { value: value.value }),
        },
      });
    case "integer":
      return create(CodecDeterministicCborValueSchema, {
        value: {
          case: "integerValue",
          value: deterministicCborIntegerToProto(value.value),
        },
      });
    case "text":
      return create(CodecDeterministicCborValueSchema, {
        value: {
          case: "textValue",
          value: create(CodecDeterministicCborTextSchema, { value: value.value }),
        },
      });
    case "bytes":
      return create(CodecDeterministicCborValueSchema, {
        value: {
          case: "bytesValue",
          value: create(CodecDeterministicCborBytesSchema, { value: value.value }),
        },
      });
    case "array":
      return create(CodecDeterministicCborValueSchema, {
        value: {
          case: "arrayValue",
          value: create(CodecDeterministicCborArraySchema, {
            values: value.value.map(deterministicCborValueToProto),
          }),
        },
      });
    case "map":
      return create(CodecDeterministicCborValueSchema, {
        value: {
          case: "mapValue",
          value: create(CodecDeterministicCborMapSchema, {
            entries: value.value.map((entry) =>
              create(CodecDeterministicCborMapEntrySchema, {
                key: deterministicCborMapKeyToProto(entry.key),
                value: deterministicCborValueToProto(entry.value),
              }),
            ),
          }),
        },
      });
  }
};

const providerFailure = (): never => {
  throw new ReallyMeCodecError("provider-failure");
};

const requireNoProviderUnknownFields = (message: Message): void => {
  if (message.$unknown !== undefined && message.$unknown.length !== 0) {
    providerFailure();
  }
};

const consumeProviderDeterministicCborBudget = (
  current: number,
  amount: number,
  maximum: number,
): number => {
  if (
    !Number.isSafeInteger(amount) ||
    amount < 0 ||
    current > maximum - amount
  ) {
    return providerFailure();
  }
  return current + amount;
};

const providerDeterministicCborUtf8ByteLength = (value: string): number => {
  try {
    return strictUtf8ByteLength(value);
  } catch (_error: unknown) {
    return providerFailure();
  }
};

const consumeProviderDeterministicCborText = (
  state: DeterministicCborValidationState,
  value: string,
): void => {
  if (
    state.textBytes >
    MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES - value.length
  ) {
    providerFailure();
  }
  state.textBytes = consumeProviderDeterministicCborBudget(
    state.textBytes,
    providerDeterministicCborUtf8ByteLength(value),
    MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES,
  );
};

const deterministicCborIntegerFromProto = (
  integer: CodecDeterministicCborInteger,
): ReallyMeDeterministicCborInteger => {
  requireNoProviderUnknownFields(integer);
  switch (integer.value.case) {
    case "unsignedValue": {
      requireNoProviderUnknownFields(integer.value.value);
      const value = integer.value.value.value;
      if (value < 0n || value > DETERMINISTIC_CBOR_U64_MAX) {
        return providerFailure();
      }
      return { type: "unsigned", value };
    }
    case "negativeValue": {
      requireNoProviderUnknownFields(integer.value.value);
      const value = integer.value.value.value;
      if (
        value < DETERMINISTIC_CBOR_I64_MIN ||
        value > DETERMINISTIC_CBOR_NEGATIVE_MAX
      ) {
        return providerFailure();
      }
      return { type: "negative", value };
    }
    case undefined:
      return providerFailure();
  }
};

const deterministicCborMapKeyFromProto = (
  key: CodecDeterministicCborMapKey,
  state: DeterministicCborValidationState,
): ReallyMeDeterministicCborMapKey => {
  requireNoProviderUnknownFields(key);
  state.nodes = consumeProviderDeterministicCborBudget(
    state.nodes,
    1,
    MAX_DETERMINISTIC_CBOR_NODES,
  );
  switch (key.key.case) {
    case "integerKey":
      return {
        type: "integer",
        value: deterministicCborIntegerFromProto(key.key.value),
      };
    case "textKey": {
      requireNoProviderUnknownFields(key.key.value);
      consumeProviderDeterministicCborText(state, key.key.value.value);
      return { type: "text", value: key.key.value.value };
    }
    case undefined:
      return providerFailure();
  }
};

const deterministicCborValueFromProto = (
  value: CodecDeterministicCborValue | undefined,
  depth: number,
  state: DeterministicCborValidationState,
): ReallyMeDeterministicCborValue => {
  if (value === undefined) {
    return providerFailure();
  }
  requireNoProviderUnknownFields(value);
  state.nodes = consumeProviderDeterministicCborBudget(
    state.nodes,
    1,
    MAX_DETERMINISTIC_CBOR_NODES,
  );
  switch (value.value.case) {
    case "nullValue":
      requireNoProviderUnknownFields(value.value.value);
      return { type: "null" };
    case "boolValue":
      requireNoProviderUnknownFields(value.value.value);
      return { type: "bool", value: value.value.value.value };
    case "integerValue":
      return {
        type: "integer",
        value: deterministicCborIntegerFromProto(value.value.value),
      };
    case "textValue": {
      requireNoProviderUnknownFields(value.value.value);
      const text = value.value.value.value;
      consumeProviderDeterministicCborText(state, text);
      return { type: "text", value: text };
    }
    case "bytesValue": {
      requireNoProviderUnknownFields(value.value.value);
      const bytes = value.value.value.value;
      state.byteStringBytes = consumeProviderDeterministicCborBudget(
        state.byteStringBytes,
        bytes.length,
        MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
      );
      const copy = bytes.slice();
      state.ownedBytes.push(copy);
      return { type: "bytes", value: copy };
    }
    case "arrayValue": {
      requireNoProviderUnknownFields(value.value.value);
      const values = value.value.value.values;
      if (
        values.length > MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES ||
        depth >= MAX_DETERMINISTIC_CBOR_NESTING_DEPTH
      ) {
        return providerFailure();
      }
      return {
        type: "array",
        value: values.map((entry): ReallyMeDeterministicCborValue =>
          deterministicCborValueFromProto(entry, depth + 1, state),
        ),
      };
    }
    case "mapValue": {
      requireNoProviderUnknownFields(value.value.value);
      const entries = value.value.value.entries;
      if (
        entries.length > MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES ||
        depth >= MAX_DETERMINISTIC_CBOR_NESTING_DEPTH
      ) {
        return providerFailure();
      }
      const keys = deterministicCborMapKeySet();
      return {
        type: "map",
        value: entries.map((entry): ReallyMeDeterministicCborMapEntry => {
          requireNoProviderUnknownFields(entry);
          if (entry.key === undefined) {
            return providerFailure();
          }
          const key = deterministicCborMapKeyFromProto(entry.key, state);
          recordDeterministicCborMapKey(key, keys, providerFailure);
          return {
            key,
            value: deterministicCborValueFromProto(
              entry.value,
              depth + 1,
              state,
            ),
          };
        }),
      };
    }
    case undefined:
      return providerFailure();
  }
};

const wipeDeterministicCborValueBytes = (
  value: ReallyMeDeterministicCborValue,
): void => {
  switch (value.type) {
    case "bytes":
      value.value.fill(0);
      break;
    case "array":
      for (const child of value.value) {
        wipeDeterministicCborValueBytes(child);
      }
      break;
    case "map":
      for (const entry of value.value) {
        wipeDeterministicCborValueBytes(entry.value);
      }
      break;
    case "null":
    case "bool":
    case "integer":
    case "text":
      break;
  }
};

const wipeProtoDeterministicCborValueBytes = (
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
        wipeProtoDeterministicCborValueBytes(child);
      }
      break;
    case "mapValue":
      for (const entry of value.value.value.entries) {
        wipeProtoDeterministicCborValueBytes(entry.value);
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

const dagCborIntegerToProto = (
  value: number | bigint,
): CodecDeterministicCborInteger => {
  const integer = typeof value === "bigint" ? value : BigInt(value);
  if (integer < i64Min || integer > i64Max) {
    return invalidCborInput();
  }
  if (integer < 0n) {
    return create(CodecDeterministicCborIntegerSchema, {
      value: {
        case: "negativeValue",
        value: create(CodecDeterministicCborNegativeIntegerSchema, {
          value: integer,
        }),
      },
    });
  }
  return create(CodecDeterministicCborIntegerSchema, {
    value: {
      case: "unsignedValue",
      value: create(CodecDeterministicCborUnsignedIntegerSchema, {
        value: integer,
      }),
    },
  });
};

const dagCborValueToProto = (
  value: ReallyMeCborValue,
): CodecDeterministicCborValue => {
  switch (value.type) {
    case "null":
      return create(CodecDeterministicCborValueSchema, {
        value: {
          case: "nullValue",
          value: create(CodecDeterministicCborNullSchema),
        },
      });
    case "bool":
      return create(CodecDeterministicCborValueSchema, {
        value: {
          case: "boolValue",
          value: create(CodecDeterministicCborBoolSchema, { value: value.value }),
        },
      });
    case "int":
      return create(CodecDeterministicCborValueSchema, {
        value: {
          case: "integerValue",
          value: dagCborIntegerToProto(value.value),
        },
      });
    case "string":
      return create(CodecDeterministicCborValueSchema, {
        value: {
          case: "textValue",
          value: create(CodecDeterministicCborTextSchema, { value: value.value }),
        },
      });
    case "bytes": {
      const bytes = value.value.slice();
      try {
        const bytesValue = create(CodecDeterministicCborBytesSchema, { value: bytes });
        return create(CodecDeterministicCborValueSchema, {
          value: {
            case: "bytesValue",
            value: bytesValue,
          },
        });
      } catch (error: unknown) {
        bytes.fill(0);
        throw error;
      }
    }
    case "array": {
      const values: CodecDeterministicCborValue[] = [];
      try {
        for (const child of value.value) {
          values.push(dagCborValueToProto(child));
        }
        return create(CodecDeterministicCborValueSchema, {
          value: {
            case: "arrayValue",
            value: create(CodecDeterministicCborArraySchema, { values }),
          },
        });
      } catch (error: unknown) {
        for (const child of values) {
          wipeProtoDeterministicCborValueBytes(child);
        }
        throw error;
      }
    }
    case "map": {
      const entries: CodecDeterministicCborMapEntry[] = [];
      try {
        for (const entry of value.value) {
          const entryValue = dagCborValueToProto(entry.value);
          try {
            entries.push(
              create(CodecDeterministicCborMapEntrySchema, {
                key: create(CodecDeterministicCborMapKeySchema, {
                  key: {
                    case: "textKey",
                    value: create(CodecDeterministicCborTextSchema, {
                      value: entry.key,
                    }),
                  },
                }),
                value: entryValue,
              }),
            );
          } catch (error: unknown) {
            wipeProtoDeterministicCborValueBytes(entryValue);
            throw error;
          }
        }
        return create(CodecDeterministicCborValueSchema, {
          value: {
            case: "mapValue",
            value: create(CodecDeterministicCborMapSchema, { entries }),
          },
        });
      } catch (error: unknown) {
        for (const entry of entries) {
          wipeProtoDeterministicCborValueBytes(entry.value);
        }
        throw error;
      }
    }
  }
};

const dagCborValueFromDeterministic = (
  value: ReallyMeDeterministicCborValue,
): ReallyMeCborValue => {
  switch (value.type) {
    case "null":
      return { type: "null" };
    case "bool":
      return { type: "bool", value: value.value };
    case "integer": {
      const integer = value.value.value;
      if (integer < i64Min || integer > i64Max) {
        return providerFailure();
      }
      return { type: "int", value: integer };
    }
    case "text":
      return { type: "string", value: value.value };
    case "bytes":
      return { type: "bytes", value: value.value.slice() };
    case "array": {
      const values: ReallyMeCborValue[] = [];
      try {
        for (const child of value.value) {
          values.push(dagCborValueFromDeterministic(child));
        }
        return {
          type: "array",
          value: values,
        };
      } catch (error: unknown) {
        for (const child of values) {
          wipeDagCborValueBytes(child);
        }
        throw error;
      }
    }
    case "map": {
      const entries: ReallyMeCborMapEntry[] = [];
      try {
        for (const entry of value.value) {
          if (entry.key.type !== "text") {
            return providerFailure();
          }
          entries.push({
            key: entry.key.value,
            value: dagCborValueFromDeterministic(entry.value),
          });
        }
        return {
          type: "map",
          value: entries,
        };
      } catch (error: unknown) {
        for (const entry of entries) {
          wipeDagCborValueBytes(entry.value);
        }
        throw error;
      }
    }
  }
};

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
      return result.encoded.slice();
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
      return result.encoded.slice();
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
  return readStringOutput(requireReallyMeCodecWasmProvider().dagCborComputeCid(bytes));
};

export const dagCborVerifyCid = (
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

const dagCborVerifyCidRequest = (
  cid: string,
  bytes: Uint8Array,
) => create(CodecOperationRequestSchema, {
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
