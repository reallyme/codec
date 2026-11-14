// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { strictUtf8ByteLength } from "./boundary.js";
import type {
  ReallyMeDeterministicCborInteger,
  ReallyMeDeterministicCborMapEntry,
  ReallyMeDeterministicCborMapKey,
  ReallyMeDeterministicCborValue,
} from "./cborValues.js";
import {
  DETERMINISTIC_CBOR_I64_MIN,
  DETERMINISTIC_CBOR_NEGATIVE_MAX,
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
import { snapshotProviderBytes } from "./readOutput.js";
import { deterministicCborMapKeySet, recordDeterministicCborMapKey } from "./snapshotDeterministicCbor.js";
import type { DeterministicCborValidationState } from "./snapshotDeterministicCbor.js";
import type { Message } from "@bufbuild/protobuf";

export const providerFailure = (): never => {
  throw new ReallyMeCodecError("provider-failure");
};

export const requireNoProviderUnknownFields = (message: Message): void => {
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

export const deterministicCborValueFromProto = (
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
      const copy = snapshotProviderBytes(
        bytes,
        MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
      );
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
