// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

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
  MAX_DETERMINISTIC_CBOR_NODES,
} from "./deterministicCborBoundary.js";
import { ReallyMeCodecError } from "./errors.js";
import {
  consumeDeterministicCborBudget,
  consumeDeterministicCborText,
  deterministicCborMapKeySet,
  deterministicCborValidationState,
  readDeterministicCborDataProperty,
  recordDeterministicCborMapKey,
  requireDeterministicCborDepth,
  requireDeterministicCborObject,
  snapshotDeterministicCborArray,
  snapshotDeterministicCborBytes,
  wipeOwnedByteSnapshots,
} from "./snapshotDeterministicCbor.js";
import type { DeterministicCborValidationState } from "./snapshotDeterministicCbor.js";
import { invalidCborInput } from "./validateDagCbor.js";

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

export const normalizeDeterministicCborValue = (
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
