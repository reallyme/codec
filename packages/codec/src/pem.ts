// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { ReallyMeCodecError } from "./errors.js";
import {
  ensureBytesInput,
  ensureStringInput,
  readBytesOutput,
  readBytesProperty,
  readObjectOutput,
  readProtoResultOutput,
  readStringOutput,
  readStringProperty,
} from "./readOutput.js";
import type { ReallyMeCodecProtoResult } from "./readOutput.js";
import { requireReallyMeCodecWasmProvider } from "./wasmProvider.js";

export type ReallyMePemLabel = "PRIVATE KEY" | "EC PRIVATE KEY" | "PUBLIC KEY";
export type ReallyMePemLineEnding = "lf" | "crlf";

export type ReallyMePemDecodePolicy = Readonly<{
  allowedLabels?: ReadonlyArray<ReallyMePemLabel>;
  maxInputLen?: number;
  maxDerLen?: number;
}>;

export type ReallyMePemEncodeOptions = Readonly<{
  maxDerLen?: number;
  lineWidth?: number;
  lineEnding?: ReallyMePemLineEnding;
}>;

export type ReallyMePemDocument = Readonly<{
  label: ReallyMePemLabel;
  der: Uint8Array;
}>;

const validLabels: ReadonlySet<string> = new Set([
  "PRIVATE KEY",
  "EC PRIVATE KEY",
  "PUBLIC KEY",
]);

const requirePemLabel = (label: string): ReallyMePemLabel => {
  if (!validLabels.has(label)) {
    throw new ReallyMeCodecError("invalid-input");
  }
  switch (label) {
    case "PRIVATE KEY":
    case "EC PRIVATE KEY":
    case "PUBLIC KEY":
      return label;
    default:
      throw new ReallyMeCodecError("invalid-input");
  }
};

const requirePositiveInteger = (value: number | undefined): void => {
  if (
    value !== undefined &&
    (!Number.isSafeInteger(value) || value <= 0)
  ) {
    throw new ReallyMeCodecError("invalid-input");
  }
};

const encodeOptions = (
  options: ReallyMePemDecodePolicy | ReallyMePemEncodeOptions | undefined,
): string => {
  if (options === undefined) {
    return "";
  }
  return JSON.stringify(options);
};

const readPemDocument = (value: unknown): ReallyMePemDocument => {
  const object = readObjectOutput(value);
  return {
    label: requirePemLabel(readStringProperty(object, "label")),
    der: readBytesProperty(object, "der"),
  };
};

export const decodePem = (
  input: string,
  policy?: ReallyMePemDecodePolicy,
): ReallyMePemDocument => {
  ensureStringInput(input);
  if (policy?.allowedLabels !== undefined) {
    for (const label of policy.allowedLabels) {
      requirePemLabel(label);
    }
  }
  requirePositiveInteger(policy?.maxInputLen);
  requirePositiveInteger(policy?.maxDerLen);
  return readPemDocument(
    requireReallyMeCodecWasmProvider().pemDecode(input, encodeOptions(policy)),
  );
};

export const decodePemProto = (
  input: string,
  policy?: ReallyMePemDecodePolicy,
): Uint8Array => {
  ensureStringInput(input);
  if (policy?.allowedLabels !== undefined) {
    for (const label of policy.allowedLabels) {
      requirePemLabel(label);
    }
  }
  requirePositiveInteger(policy?.maxInputLen);
  requirePositiveInteger(policy?.maxDerLen);
  return readBytesOutput(
    requireReallyMeCodecWasmProvider().pemDecodeProto(input, encodeOptions(policy)),
  );
};

export const decodePemProtoResult = (
  input: string,
  policy?: ReallyMePemDecodePolicy,
): ReallyMeCodecProtoResult => {
  ensureStringInput(input);
  if (policy?.allowedLabels !== undefined) {
    for (const label of policy.allowedLabels) {
      requirePemLabel(label);
    }
  }
  requirePositiveInteger(policy?.maxInputLen);
  requirePositiveInteger(policy?.maxDerLen);
  return readProtoResultOutput(
    requireReallyMeCodecWasmProvider().pemDecodeProtoResult(input, encodeOptions(policy)),
  );
};

export const encodePem = (
  label: ReallyMePemLabel,
  der: Uint8Array,
  options?: ReallyMePemEncodeOptions,
): string => {
  requirePemLabel(label);
  ensureBytesInput(der);
  requirePositiveInteger(options?.maxDerLen);
  requirePositiveInteger(options?.lineWidth);
  if (
    options?.lineEnding !== undefined &&
    options.lineEnding !== "lf" &&
    options.lineEnding !== "crlf"
  ) {
    throw new ReallyMeCodecError("invalid-input");
  }
  return readStringOutput(
    requireReallyMeCodecWasmProvider().pemEncode(label, der, encodeOptions(options)),
  );
};
