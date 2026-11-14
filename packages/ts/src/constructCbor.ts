// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import type {
  ReallyMeCborMapEntry,
  ReallyMeCborValue,
  ReallyMeDeterministicCborMapEntry,
  ReallyMeDeterministicCborMapKey,
  ReallyMeDeterministicCborValue,
} from "./cborValues.js";
import {
  DETERMINISTIC_CBOR_I64_MIN,
  DETERMINISTIC_CBOR_NEGATIVE_MAX,
  DETERMINISTIC_CBOR_U64_MAX,
} from "./deterministicCborBoundary.js";
import { ensureBytesInput, ensureStringValue, snapshotBoundedBytesInput } from "./readOutput.js";
import { invalidCborInput, validateCborInteger } from "./validateDagCbor.js";

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
    return { type: "bytes", value: snapshotBoundedBytesInput(value) };
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
    return { type: "bytes", value: snapshotBoundedBytesInput(value) };
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
