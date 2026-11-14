// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { MAX_CODEC_FFI_INPUT_BYTES, strictUtf8ByteLength, utf8ByteLength } from "./boundary.js";
import { DAG_CBOR_I64_MAX, DETERMINISTIC_CBOR_PROTO_NODE_OVERHEAD_BUDGET } from "./boundedOperationResponse.js";
import {
  DETERMINISTIC_CBOR_I64_MIN,
  DETERMINISTIC_CBOR_U64_MAX,
  MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
  MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES,
  MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES,
  MAX_DETERMINISTIC_CBOR_NESTING_DEPTH,
  MAX_DETERMINISTIC_CBOR_NODES,
} from "./deterministicCborBoundary.js";
import { ReallyMeCodecError } from "./errors.js";
import type {
  CodecDeterministicCborInteger,
  CodecDeterministicCborMapKey,
  CodecDeterministicCborValue,
} from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import type { Message } from "@bufbuild/protobuf";

type GeneratedDeterministicCborBudget = {
  nodes: number;
  textBytes: number;
  byteStringBytes: number;
};

type GeneratedDeterministicCborMapKeyIdentity =
  | Readonly<{ type: "integer"; value: bigint }>
  | Readonly<{ type: "text"; value: string }>;

export const requireNoGeneratedUnknownFields = (message: Message): void => {
  if (message.$unknown !== undefined && message.$unknown.length !== 0) {
    throw new ReallyMeCodecError("invalid-input");
  }
};

export const hasUnknownFields = (message: Message): boolean =>
  message.$unknown !== undefined && message.$unknown.length !== 0;

export const requireProtoBytes = (value: unknown): number => {
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

export const protoUtf8Length = (
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

export const validateGeneratedDeterministicCborValue = (
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

export const validateGeneratedDagCborValue = (
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
