// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { strictUtf8ByteLength } from "./boundary.js";
import type { ReallyMeDeterministicCborMapKey } from "./cborValues.js";
import {
  MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES,
  MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES,
  MAX_DETERMINISTIC_CBOR_NESTING_DEPTH,
} from "./deterministicCborBoundary.js";
import { ReallyMeCodecError } from "./errors.js";
import { invalidCborInput, isRecord } from "./validateDagCbor.js";

export type DeterministicCborValidationState = {
  nodes: number;
  textBytes: number;
  byteStringBytes: number;
  ownedBytes: Uint8Array[];
};

type DeterministicCborMapKeySet = {
  integerKeys: Set<bigint>;
  textKeys: Set<string>;
};

export const deterministicCborMapKeySet = (): DeterministicCborMapKeySet => ({
  integerKeys: new Set<bigint>(),
  textKeys: new Set<string>(),
});

export const recordDeterministicCborMapKey = (
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

export const deterministicCborValidationState =
  (): DeterministicCborValidationState => ({
    nodes: 0,
    textBytes: 0,
    byteStringBytes: 0,
    ownedBytes: [],
  });

export const wipeOwnedByteSnapshots = (
  state: Pick<DeterministicCborValidationState, "ownedBytes">,
): void => {
  for (const value of state.ownedBytes) {
    value.fill(0);
  }
};

export const consumeDeterministicCborBudget = (
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

export const consumeDeterministicCborText = (
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

export const requireDeterministicCborObject = (
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

export const readDeterministicCborDataProperty = (
  properties: ReadonlyMap<string, unknown>,
  key: string,
): unknown => {
  if (!properties.has(key)) {
    return invalidCborInput();
  }
  return properties.get(key);
};

export const snapshotDeterministicCborArray = (
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
export const snapshotDeterministicCborBytes = (
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

export const requireDeterministicCborDepth = (depth: number): number => {
  const nextDepth = depth + 1;
  if (nextDepth > MAX_DETERMINISTIC_CBOR_NESTING_DEPTH) {
    return invalidCborInput();
  }
  return nextDepth;
};
