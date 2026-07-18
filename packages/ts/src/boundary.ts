// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { ReallyMeCodecError } from "./errors.js";

export const MAX_CODEC_FFI_INPUT_BYTES = 1_048_576;
export const MAX_CODEC_FFI_OUTPUT_BYTES = 67_108_864;
const MAX_CODEC_PROTO_SENSITIVE_PAYLOAD_BYTES = 2_097_152;
const MAX_CODEC_PROTO_SEMANTIC_NODES = 65_536;
const MAX_CODEC_PROTO_STRUCTURAL_BYTES_PER_NODE = 128;
const MAX_CODEC_PROTO_FIXED_OPERATION_BYTES = 4_096;
const MAX_CODEC_PROTO_JSON_TEXT_BYTES = 6_291_456;
const MAX_CODEC_PROTO_JSON_BYTE_STRING_BYTES = 1_398_104;
export const MAX_CODEC_PROTO_MESSAGE_BYTES =
  MAX_CODEC_PROTO_SENSITIVE_PAYLOAD_BYTES +
  MAX_CODEC_PROTO_SEMANTIC_NODES * MAX_CODEC_PROTO_STRUCTURAL_BYTES_PER_NODE +
  MAX_CODEC_PROTO_FIXED_OPERATION_BYTES;
export const MAX_CODEC_PROTO_JSON_BYTES =
  MAX_CODEC_PROTO_JSON_TEXT_BYTES +
  MAX_CODEC_PROTO_JSON_BYTE_STRING_BYTES +
  MAX_CODEC_PROTO_SEMANTIC_NODES * MAX_CODEC_PROTO_STRUCTURAL_BYTES_PER_NODE +
  MAX_CODEC_PROTO_FIXED_OPERATION_BYTES;
// Every JSON node needs at least one serialized byte. Deriving this traversal
// guard from the wire limit prevents hostile in-memory graphs from causing
// unbounded work without rejecting a document the byte-bounded Rust lane accepts.
export const MAX_CODEC_BOUNDARY_NODES = MAX_CODEC_FFI_INPUT_BYTES;
const MAX_JSON_CONTAINER_DEPTH = 128;

type JsonBudget = {
  bytes: number;
  nodes: number;
};

const invalidInput = (): never => {
  throw new ReallyMeCodecError("invalid-input");
};

const consumeBudget = (
  budget: JsonBudget,
  field: "bytes" | "nodes",
  amount: number,
  maximum: number,
): void => {
  if (!Number.isSafeInteger(amount) || amount < 0 || budget[field] > maximum - amount) {
    invalidInput();
  }
  budget[field] += amount;
};

export const requireBoundaryAggregate = (
  lengths: ReadonlyArray<number>,
  maximum = MAX_CODEC_FFI_INPUT_BYTES,
): void => {
  let aggregate = 0;
  for (const length of lengths) {
    if (
      !Number.isSafeInteger(length) ||
      length < 0 ||
      aggregate > maximum - length
    ) {
      invalidInput();
    }
    aggregate += length;
  }
};

const utf8ByteLengthWithPolicy = (
  value: string,
  rejectUnpairedSurrogates: boolean,
): number => {
  if (typeof value !== "string") {
    invalidInput();
  }
  let length = 0;
  for (let index = 0; index < value.length; index += 1) {
    const codeUnit = value.charCodeAt(index);
    let increment: number;
    if (codeUnit <= 0x7f) {
      increment = 1;
    } else if (codeUnit <= 0x7ff) {
      increment = 2;
    } else if (
      codeUnit >= 0xd800 &&
      codeUnit <= 0xdbff &&
      index + 1 < value.length
    ) {
      const next = value.charCodeAt(index + 1);
      if (next >= 0xdc00 && next <= 0xdfff) {
        increment = 4;
        index += 1;
      } else {
        if (rejectUnpairedSurrogates) {
          invalidInput();
        }
        increment = 3;
      }
    } else if (codeUnit >= 0xd800 && codeUnit <= 0xdfff) {
      if (rejectUnpairedSurrogates) {
        invalidInput();
      }
      increment = 3;
    } else {
      increment = 3;
    }
    if (length > Number.MAX_SAFE_INTEGER - increment) {
      invalidInput();
    }
    length += increment;
  }
  return length;
};

/** Returns UTF-8 length without allocating an encoded copy. */
export const utf8ByteLength = (value: string): number =>
  utf8ByteLengthWithPolicy(value, false);

/**
 * Returns UTF-8 length while rejecting strings that cannot be represented as
 * Unicode scalar values. TextEncoder and protobuf runtimes otherwise replace
 * lone UTF-16 surrogates, silently changing deterministic-CBOR semantics.
 */
export const strictUtf8ByteLength = (value: string): number =>
  utf8ByteLengthWithPolicy(value, true);

export const requireBoundaryUtf8String = (
  value: string,
  allowEmpty = true,
  maximum = MAX_CODEC_FFI_INPUT_BYTES,
): number => {
  if (typeof value !== "string" || (!allowEmpty && value.length === 0)) {
    invalidInput();
  }
  // Every UTF-16 code unit contributes at least one UTF-8 byte. This rejects
  // obviously oversized strings before walking their full contents.
  if (value.length > maximum) {
    invalidInput();
  }
  const length = utf8ByteLength(value);
  requireBoundaryAggregate([length], maximum);
  return length;
};

const jsonStringByteLength = (value: string): number => {
  let length = 2;
  for (let index = 0; index < value.length; index += 1) {
    const codeUnit = value.charCodeAt(index);
    let increment: number;
    if (codeUnit === 0x22 || codeUnit === 0x5c) {
      increment = 2;
    } else if (codeUnit <= 0x1f) {
      increment =
        codeUnit === 0x08 ||
        codeUnit === 0x09 ||
        codeUnit === 0x0a ||
        codeUnit === 0x0c ||
        codeUnit === 0x0d
          ? 2
          : 6;
    } else if (codeUnit <= 0x7f) {
      increment = 1;
    } else if (codeUnit <= 0x7ff) {
      increment = 2;
    } else if (codeUnit >= 0xd800 && codeUnit <= 0xdbff) {
      if (index + 1 < value.length) {
        const next = value.charCodeAt(index + 1);
        if (next >= 0xdc00 && next <= 0xdfff) {
          increment = 4;
          index += 1;
        } else {
          increment = 6;
        }
      } else {
        increment = 6;
      }
    } else if (codeUnit >= 0xdc00 && codeUnit <= 0xdfff) {
      increment = 6;
    } else {
      increment = 3;
    }
    if (length > Number.MAX_SAFE_INTEGER - increment) {
      invalidInput();
    }
    length += increment;
  }
  return length;
};

const readOwnDataProperty = (value: object, key: string): unknown => {
  const descriptor = Object.getOwnPropertyDescriptor(value, key);
  if (descriptor === undefined || !("value" in descriptor)) {
    return invalidInput();
  }
  return descriptor.value;
};

const normalizeJson = (
  value: unknown,
  depth: number,
  seen: WeakSet<object>,
  budget: JsonBudget,
): unknown => {
  consumeBudget(budget, "nodes", 1, MAX_CODEC_BOUNDARY_NODES);
  if (value === null) {
    consumeBudget(budget, "bytes", 4, MAX_CODEC_FFI_INPUT_BYTES);
    return null;
  }
  switch (typeof value) {
    case "boolean":
      consumeBudget(budget, "bytes", value ? 4 : 5, MAX_CODEC_FFI_INPUT_BYTES);
      return value;
    case "string":
      consumeBudget(
        budget,
        "bytes",
        jsonStringByteLength(value),
        MAX_CODEC_FFI_INPUT_BYTES,
      );
      return value;
    case "number": {
      if (!Number.isFinite(value)) {
        invalidInput();
      }
      const encoded = JSON.stringify(value);
      if (typeof encoded !== "string") {
        invalidInput();
      }
      consumeBudget(budget, "bytes", encoded.length, MAX_CODEC_FFI_INPUT_BYTES);
      return value;
    }
    case "bigint":
    case "function":
    case "symbol":
    case "undefined":
      return invalidInput();
    case "object":
      break;
  }

  if (seen.has(value) || depth >= MAX_JSON_CONTAINER_DEPTH) {
    invalidInput();
  }
  const prototype = Object.getPrototypeOf(value);
  if (prototype !== Object.prototype && prototype !== null && !Array.isArray(value)) {
    invalidInput();
  }

  seen.add(value);
  try {
    if (Array.isArray(value)) {
      const rawLength = readOwnDataProperty(value, "length");
      if (typeof rawLength !== "number") {
        return invalidInput();
      }
      const length = rawLength;
      if (
        !Number.isSafeInteger(length) ||
        length < 0 ||
        length > MAX_CODEC_BOUNDARY_NODES - budget.nodes
      ) {
        invalidInput();
      }
      consumeBudget(
        budget,
        "bytes",
        length === 0 ? 2 : length + 1,
        MAX_CODEC_FFI_INPUT_BYTES,
      );
      const normalized: unknown[] = [];
      for (let index = 0; index < length; index += 1) {
        // Indexed access and iteration both invoke array accessors. Snapshot
        // only own data properties so validation cannot execute untrusted code.
        const entry = readOwnDataProperty(value, String(index));
        normalized.push(normalizeJson(entry, depth + 1, seen, budget));
      }
      return normalized;
    }

    const keys = Object.keys(value);
    if (keys.length > MAX_CODEC_BOUNDARY_NODES - budget.nodes) {
      invalidInput();
    }
    consumeBudget(
      budget,
      "bytes",
      keys.length === 0 ? 2 : keys.length + 1,
      MAX_CODEC_FFI_INPUT_BYTES,
    );
    const entries: Array<readonly [string, unknown]> = [];
    for (const key of keys) {
      const descriptor = Object.getOwnPropertyDescriptor(value, key);
      if (descriptor === undefined) {
        return invalidInput();
      }
      if (!("value" in descriptor)) {
        return invalidInput();
      }
      consumeBudget(
        budget,
        "bytes",
        jsonStringByteLength(key),
        MAX_CODEC_FFI_INPUT_BYTES,
      );
      consumeBudget(budget, "bytes", 1, MAX_CODEC_FFI_INPUT_BYTES);
      const propertyValue: unknown = descriptor.value;
      entries.push([key, normalizeJson(propertyValue, depth + 1, seen, budget)]);
    }
    return Object.fromEntries(entries);
  } finally {
    seen.delete(value);
  }
};

/** Validates, snapshots, and serializes untrusted JSON within a fixed budget. */
export const stringifyBoundaryJson = (value: unknown): string => {
  try {
    const normalized = normalizeJson(value, 0, new WeakSet<object>(), {
      bytes: 0,
      nodes: 0,
    });
    const encoded = JSON.stringify(normalized);
    if (typeof encoded !== "string") {
      invalidInput();
    }
    requireBoundaryAggregate([utf8ByteLength(encoded)]);
    return encoded;
  } catch (error: unknown) {
    if (error instanceof ReallyMeCodecError) {
      throw error;
    }
    return invalidInput();
  }
};
