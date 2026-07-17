// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { create } from "@bufbuild/protobuf";

import { stringifyBoundaryJson } from "./boundary.js";
import { ReallyMeCodecError } from "./errors.js";
import {
  CodecOperationRequestSchema,
  CodecPemDecodeOptionsSchema,
  CodecPemDecodeRequestSchema,
  CodecPemLabel,
} from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import {
  processGeneratedProtoRequest,
  protoPayloadOrThrow,
} from "./protoProcess.js";
import {
  ensureBytesInput,
  readBytesOutput,
  readBytesProperty,
  readObjectOutput,
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

const MAX_PEM_ALLOWED_LABELS = 1_024;
const pemDecodePolicyFields: ReadonlySet<string> = new Set([
  "allowedLabels",
  "maxInputLen",
  "maxDerLen",
]);
const pemEncodeOptionFields: ReadonlySet<string> = new Set([
  "maxDerLen",
  "lineWidth",
  "lineEnding",
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

const readPemLabel = (label: string): ReallyMePemLabel => {
  if (!validLabels.has(label)) {
    throw new ReallyMeCodecError("provider-failure");
  }
  switch (label) {
    case "PRIVATE KEY":
    case "EC PRIVATE KEY":
    case "PUBLIC KEY":
      return label;
    default:
      throw new ReallyMeCodecError("provider-failure");
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

const readSnapshotProperty = (value: object, name: string): unknown => {
  const descriptor = Object.getOwnPropertyDescriptor(value, name);
  if (descriptor === undefined) {
    return undefined;
  }
  if (!("value" in descriptor)) {
    throw new ReallyMeCodecError("invalid-input");
  }
  return descriptor.value;
};

const snapshotOptionsRecord = (
  value: unknown,
  allowedFields: ReadonlySet<string>,
): object => {
  try {
    if (typeof value !== "object" || value === null || Array.isArray(value)) {
      throw new ReallyMeCodecError("invalid-input");
    }
    const prototype = Object.getPrototypeOf(value);
    if (prototype !== Object.prototype && prototype !== null) {
      throw new ReallyMeCodecError("invalid-input");
    }
    const entries: Array<readonly [string, unknown]> = [];
    for (const key of Reflect.ownKeys(value)) {
      if (typeof key !== "string" || !allowedFields.has(key)) {
        throw new ReallyMeCodecError("invalid-input");
      }
      const descriptor = Object.getOwnPropertyDescriptor(value, key);
      if (descriptor === undefined || !("value" in descriptor)) {
        throw new ReallyMeCodecError("invalid-input");
      }
      entries.push([key, descriptor.value]);
    }
    return Object.fromEntries(entries);
  } catch (error: unknown) {
    if (error instanceof ReallyMeCodecError) {
      throw error;
    }
    throw new ReallyMeCodecError("invalid-input");
  }
};

const readPositiveIntegerOption = (
  value: object,
  name: string,
): number | undefined => {
  const property = readSnapshotProperty(value, name);
  if (property === undefined) {
    return undefined;
  }
  if (typeof property !== "number") {
    throw new ReallyMeCodecError("invalid-input");
  }
  requirePositiveInteger(property);
  return property;
};

const snapshotDecodePolicy = (
  policy: ReallyMePemDecodePolicy | undefined,
): ReallyMePemDecodePolicy | undefined => {
  if (policy === undefined) {
    return undefined;
  }
  const snapshot = snapshotOptionsRecord(policy, pemDecodePolicyFields);
  const labelsValue = readSnapshotProperty(snapshot, "allowedLabels");
  let allowedLabels: ReallyMePemLabel[] | undefined;
  if (labelsValue !== undefined) {
    if (
      !Array.isArray(labelsValue) ||
      Object.getPrototypeOf(labelsValue) !== Array.prototype ||
      labelsValue.length > MAX_PEM_ALLOWED_LABELS
    ) {
      throw new ReallyMeCodecError("invalid-input");
    }
    allowedLabels = [];
    for (let index = 0; index < labelsValue.length; index += 1) {
      const label = readSnapshotProperty(labelsValue, String(index));
      if (typeof label !== "string") {
        throw new ReallyMeCodecError("invalid-input");
      }
      allowedLabels.push(requirePemLabel(label));
    }
  }
  const maxInputLen = readPositiveIntegerOption(snapshot, "maxInputLen");
  const maxDerLen = readPositiveIntegerOption(snapshot, "maxDerLen");
  return {
    ...(allowedLabels === undefined ? {} : { allowedLabels }),
    ...(maxInputLen === undefined ? {} : { maxInputLen }),
    ...(maxDerLen === undefined ? {} : { maxDerLen }),
  };
};

const snapshotEncodeOptions = (
  options: ReallyMePemEncodeOptions | undefined,
): ReallyMePemEncodeOptions | undefined => {
  if (options === undefined) {
    return undefined;
  }
  const snapshot = snapshotOptionsRecord(options, pemEncodeOptionFields);
  const maxDerLen = readPositiveIntegerOption(snapshot, "maxDerLen");
  const lineWidth = readPositiveIntegerOption(snapshot, "lineWidth");
  const lineEndingValue = readSnapshotProperty(snapshot, "lineEnding");
  let lineEnding: ReallyMePemLineEnding | undefined;
  if (lineEndingValue !== undefined) {
    if (lineEndingValue !== "lf" && lineEndingValue !== "crlf") {
      throw new ReallyMeCodecError("invalid-input");
    }
    lineEnding = lineEndingValue;
  }
  return {
    ...(maxDerLen === undefined ? {} : { maxDerLen }),
    ...(lineWidth === undefined ? {} : { lineWidth }),
    ...(lineEnding === undefined ? {} : { lineEnding }),
  };
};

const requireProtoUint32 = (value: number | undefined): number => {
  requirePositiveInteger(value);
  if (value === undefined) {
    return 0;
  }
  if (value > 0xffff_ffff) {
    throw new ReallyMeCodecError("invalid-input");
  }
  return value;
};

const protoPemLabel = (label: ReallyMePemLabel): CodecPemLabel => {
  switch (label) {
    case "PRIVATE KEY":
      return CodecPemLabel.PRIVATE_KEY;
    case "EC PRIVATE KEY":
      return CodecPemLabel.EC_PRIVATE_KEY;
    case "PUBLIC KEY":
      return CodecPemLabel.PUBLIC_KEY;
  }
};

const encodeOptions = (
  options: ReallyMePemDecodePolicy | ReallyMePemEncodeOptions | undefined,
): string => {
  if (options === undefined) {
    return "";
  }
  return stringifyBoundaryJson(options);
};

const readPemDocument = (value: unknown): ReallyMePemDocument => {
  const object = readObjectOutput(value);
  return {
    label: readPemLabel(readStringProperty(object, "label")),
    der: readBytesProperty(object, "der"),
  };
};

export const decodePem = (
  input: Uint8Array,
  policy?: ReallyMePemDecodePolicy,
): ReallyMePemDocument => {
  ensureBytesInput(input);
  const snapshot = snapshotDecodePolicy(policy);
  return readPemDocument(
    requireReallyMeCodecWasmProvider().pemDecode(input, encodeOptions(snapshot)),
  );
};

export const decodePemProto = (
  input: Uint8Array,
  policy?: ReallyMePemDecodePolicy,
): Uint8Array => {
  return protoPayloadOrThrow(decodePemProtoResult(input, policy));
};

export const decodePemProtoResult = (
  input: Uint8Array,
  policy?: ReallyMePemDecodePolicy,
): ReallyMeCodecProtoResult => {
  ensureBytesInput(input);
  const snapshot = snapshotDecodePolicy(policy);
  const allowedLabels: CodecPemLabel[] = [];
  if (snapshot?.allowedLabels !== undefined) {
    for (const label of snapshot.allowedLabels) {
      allowedLabels.push(protoPemLabel(requirePemLabel(label)));
    }
  }
  const request = create(CodecPemDecodeRequestSchema, { pem: input });
  try {
    if (snapshot !== undefined) {
      request.options = create(CodecPemDecodeOptionsSchema, {
        allowedLabels,
        maxInputLen: requireProtoUint32(snapshot.maxInputLen),
        maxDerLen: requireProtoUint32(snapshot.maxDerLen),
      });
    }
    return processGeneratedProtoRequest(create(CodecOperationRequestSchema, {
      operation: {
        case: "pemDecode",
        value: request,
      },
    }));
  } finally {
    request.pem = new Uint8Array(0);
  }
};

export const encodePem = (
  label: ReallyMePemLabel,
  der: Uint8Array,
  options?: ReallyMePemEncodeOptions,
): Uint8Array => {
  requirePemLabel(label);
  ensureBytesInput(der);
  const snapshot = snapshotEncodeOptions(options);
  return readBytesOutput(
    requireReallyMeCodecWasmProvider().pemEncode(label, der, encodeOptions(snapshot)),
  );
};
