// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import {
  MAX_CODEC_FFI_INPUT_BYTES,
  MAX_CODEC_FFI_OUTPUT_BYTES,
  requireBoundaryUtf8String,
} from "./boundary.js";
import { ReallyMeCodecError } from "./errors.js";

const uint8ArraySubarray = Uint8Array.prototype.subarray;
const uint8ArraySet = Uint8Array.prototype.set;

const readProviderProperty = (
  object: object,
  name: string,
  allowMissing = false,
): unknown => {
  try {
    const descriptor = Object.getOwnPropertyDescriptor(object, name);
    if (descriptor === undefined) {
      if (allowMissing) {
        return undefined;
      }
      throw new ReallyMeCodecError("provider-failure");
    }
    if (!("value" in descriptor)) {
      throw new ReallyMeCodecError("provider-failure");
    }
    return descriptor.value;
  } catch (error: unknown) {
    if (error instanceof ReallyMeCodecError) {
      throw error;
    }
    throw new ReallyMeCodecError("provider-failure");
  }
};

const requireBoundedProviderString = (value: string, allowEmpty: boolean): string => {
  try {
    requireBoundaryUtf8String(value, allowEmpty, MAX_CODEC_FFI_OUTPUT_BYTES);
    return value;
  } catch (_error: unknown) {
    throw new ReallyMeCodecError("provider-failure");
  }
};

export const ensureBytesInput = (value: Uint8Array): void => {
  if (
    !(value instanceof Uint8Array) ||
    !ArrayBuffer.isView(value) ||
    value.constructor !== Uint8Array
  ) {
    throw new ReallyMeCodecError("invalid-input");
  }
};

/**
 * Copies one bounded caller-owned byte view and verifies that the same length
 * was copied. Length-tracking resizable buffers, detached buffers, and exotic
 * typed-array wrappers must not let validation observe one length while the
 * provider or protobuf serializer consumes another.
 */
export const snapshotBoundedBytesInput = (
  value: Uint8Array,
  maximumLength = MAX_CODEC_FFI_INPUT_BYTES,
): Uint8Array => {
  ensureBytesInput(value);
  const expectedLength = value.length;
  if (expectedLength > maximumLength) {
    throw new ReallyMeCodecError("invalid-input");
  }
  let snapshot: Uint8Array | undefined;
  try {
    const boundedView = uint8ArraySubarray.call(value, 0, expectedLength);
    if (boundedView.length !== expectedLength) {
      throw new ReallyMeCodecError("invalid-input");
    }
    snapshot = new Uint8Array(expectedLength);
    uint8ArraySet.call(snapshot, boundedView, 0);
    if (
      value.length !== expectedLength ||
      boundedView.length !== expectedLength ||
      snapshot.length !== expectedLength
    ) {
      snapshot.fill(0);
      throw new ReallyMeCodecError("invalid-input");
    }
    return snapshot;
  } catch (error: unknown) {
    snapshot?.fill(0);
    if (error instanceof ReallyMeCodecError) {
      throw error;
    }
    throw new ReallyMeCodecError("invalid-input");
  }
};

export const ensureStringInput = (value: string): void => {
  if (typeof value !== "string" || value.length === 0) {
    throw new ReallyMeCodecError("invalid-input");
  }
};

export const ensureStringValue = (value: string): void => {
  if (typeof value !== "string") {
    throw new ReallyMeCodecError("invalid-input");
  }
};

export const readBooleanOutput = (value: unknown): boolean => {
  if (typeof value !== "boolean") {
    throw new ReallyMeCodecError("provider-failure");
  }
  return value;
};

export const readBytesOutput = (value: unknown): Uint8Array => {
  if (!(value instanceof Uint8Array)) {
    throw new ReallyMeCodecError("provider-failure");
  }
  if (value.length > MAX_CODEC_FFI_OUTPUT_BYTES) {
    value.fill(0);
    throw new ReallyMeCodecError("provider-failure");
  }
  return value;
};

export const readIndependentBoundedBytesOutput = (
  value: unknown,
  input: Uint8Array,
  maximumLength: number,
): Uint8Array => {
  try {
    if (!(value instanceof Uint8Array)) {
      throw new ReallyMeCodecError("provider-failure");
    }
    // Even non-overlapping views into one ArrayBuffer share transfer and detach
    // semantics, so they do not provide independent result ownership.
    if (value.buffer === input.buffer) {
      throw new ReallyMeCodecError("provider-failure");
    }
    if (value.length === 0 || value.length > maximumLength) {
      value.fill(0);
      throw new ReallyMeCodecError("provider-failure");
    }
    return value;
  } catch (error: unknown) {
    if (error instanceof ReallyMeCodecError) {
      throw error;
    }
    // Detached views and proxy-wrapped typed arrays can throw while reading
    // buffer metadata or clearing storage. They are malformed provider output,
    // never platform exceptions that should escape the typed SDK contract.
    throw new ReallyMeCodecError("provider-failure");
  }
};

export const readNumberOutput = (value: unknown): number => {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) {
    throw new ReallyMeCodecError("provider-failure");
  }
  return value;
};

export const readObjectOutput = (value: unknown): object => {
  if (typeof value !== "object" || value === null) {
    throw new ReallyMeCodecError("provider-failure");
  }
  try {
    const prototype = Object.getPrototypeOf(value);
    if (prototype !== Object.prototype && prototype !== null) {
      throw new ReallyMeCodecError("provider-failure");
    }
    return value;
  } catch (error: unknown) {
    if (error instanceof ReallyMeCodecError) {
      throw error;
    }
    throw new ReallyMeCodecError("provider-failure");
  }
};

export const readStringOutput = (value: unknown): string => {
  if (typeof value !== "string") {
    throw new ReallyMeCodecError("provider-failure");
  }
  return requireBoundedProviderString(value, true);
};

export const readStringProperty = (object: object, name: string): string => {
  const value = readProviderProperty(object, name);
  if (typeof value !== "string" || value.length === 0) {
    throw new ReallyMeCodecError("provider-failure");
  }
  return requireBoundedProviderString(value, false);
};

export const readStringValueProperty = (object: object, name: string): string => {
  const value = readProviderProperty(object, name);
  if (typeof value !== "string") {
    throw new ReallyMeCodecError("provider-failure");
  }
  return requireBoundedProviderString(value, true);
};

export const readBooleanProperty = (object: object, name: string): boolean => {
  const value = readProviderProperty(object, name);
  if (typeof value !== "boolean") {
    throw new ReallyMeCodecError("provider-failure");
  }
  return value;
};

export const readBytesProperty = (object: object, name: string): Uint8Array => {
  const value = readProviderProperty(object, name);
  if (!(value instanceof Uint8Array)) {
    throw new ReallyMeCodecError("provider-failure");
  }
  if (value.length > MAX_CODEC_FFI_OUTPUT_BYTES) {
    value.fill(0);
    throw new ReallyMeCodecError("provider-failure");
  }
  return value;
};

export const readOptionalLengthProperty = (
  object: object,
  name: string,
): number | undefined => {
  const value = readProviderProperty(object, name, true);
  if (value === undefined) {
    return undefined;
  }
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) {
    throw new ReallyMeCodecError("provider-failure");
  }
  return value;
};

export const asciiBytesToString = (bytes: Uint8Array): string => {
  if (bytes.length > MAX_CODEC_FFI_INPUT_BYTES) {
    throw new ReallyMeCodecError("invalid-input");
  }
  let value = "";
  // Indexed access avoids executing an attacker-supplied iterator installed
  // on an otherwise valid Uint8Array instance.
  for (let index = 0; index < bytes.length; index += 1) {
    const byte = bytes[index];
    if (byte === undefined) {
      throw new ReallyMeCodecError("invalid-input");
    }
    if (byte > 0x7f) {
      throw new ReallyMeCodecError("invalid-input");
    }
    value += String.fromCharCode(byte);
  }
  return value;
};
