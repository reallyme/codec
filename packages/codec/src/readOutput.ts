// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { ReallyMeCodecError } from "./errors.js";

export const ensureBytesInput = (value: Uint8Array): void => {
  if (!(value instanceof Uint8Array)) {
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
  return value;
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
  return value;
};

export const readStringOutput = (value: unknown): string => {
  if (typeof value !== "string") {
    throw new ReallyMeCodecError("provider-failure");
  }
  return value;
};

export const readStringProperty = (object: object, name: string): string => {
  const value: unknown = Reflect.get(object, name);
  if (typeof value !== "string" || value.length === 0) {
    throw new ReallyMeCodecError("provider-failure");
  }
  return value;
};

export const readStringValueProperty = (object: object, name: string): string => {
  const value: unknown = Reflect.get(object, name);
  if (typeof value !== "string") {
    throw new ReallyMeCodecError("provider-failure");
  }
  return value;
};

export const readBooleanProperty = (object: object, name: string): boolean => {
  const value: unknown = Reflect.get(object, name);
  if (typeof value !== "boolean") {
    throw new ReallyMeCodecError("provider-failure");
  }
  return value;
};

export const readBytesProperty = (object: object, name: string): Uint8Array => {
  const value: unknown = Reflect.get(object, name);
  if (!(value instanceof Uint8Array)) {
    throw new ReallyMeCodecError("provider-failure");
  }
  return value;
};

export type ReallyMeCodecProtoStatus = "result" | "codec-error";

export type ReallyMeCodecProtoResult = Readonly<{
  status: ReallyMeCodecProtoStatus;
  bytes: Uint8Array;
  isCodecError: boolean;
}>;

export const readProtoResultOutput = (value: unknown): ReallyMeCodecProtoResult => {
  const object = readObjectOutput(value);
  const status = readStringProperty(object, "status");
  if (status !== "result" && status !== "codec-error") {
    throw new ReallyMeCodecError("provider-failure");
  }
  return {
    status,
    bytes: readBytesProperty(object, "bytes"),
    isCodecError: status === "codec-error",
  };
};

export const readOptionalLengthProperty = (
  object: object,
  name: string,
): number | undefined => {
  const value: unknown = Reflect.get(object, name);
  if (value === undefined) {
    return undefined;
  }
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) {
    throw new ReallyMeCodecError("provider-failure");
  }
  return value;
};

export const asciiBytesToString = (bytes: Uint8Array): string => {
  let value = "";
  for (const byte of bytes) {
    if (byte > 0x7f) {
      throw new ReallyMeCodecError("invalid-input");
    }
    value += String.fromCharCode(byte);
  }
  return value;
};
