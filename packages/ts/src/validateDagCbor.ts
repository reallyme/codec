// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { MAX_CODEC_BOUNDARY_NODES } from "./boundary.js";
import type { ReallyMeCborMapEntry, ReallyMeCborValue } from "./cborValues.js";
import {
  MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
  MAX_DETERMINISTIC_CBOR_NODES,
} from "./deterministicCborBoundary.js";
import { ReallyMeCodecError } from "./errors.js";
import { ensureBytesInput, snapshotBoundedBytesInput } from "./readOutput.js";

export const invalidCborInput = (): never => {
  throw new ReallyMeCodecError("invalid-input");
};

export const isRecord = (value: unknown): value is object => {
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

export const i64Min = -(1n << 63n);

export const i64Max = (1n << 63n) - 1n;

const decimalIntegerPattern = /^-?(0|[1-9][0-9]*)$/u;

const maxCborContainerDepth = 128;

export const maxDagCborInputLength = 1024 * 1024;

type CborValidationState = {
  nodes: number;
  byteStringBytes: number;
};

export const wipeDagCborValueBytes = (value: ReallyMeCborValue | undefined): void => {
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

export const validateCborInteger = (value: unknown): number | bigint => {
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

export const validateCborValue = (
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
        return { type: "bytes", value: snapshotBoundedBytesInput(child) };
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
