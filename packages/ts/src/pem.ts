// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { create } from "@bufbuild/protobuf";
import type { Message } from "@bufbuild/protobuf";

import { MAX_CODEC_PEM_INPUT_BYTES } from "./boundary.js";
import { ReallyMeCodecError } from "./errors.js";
import {
  CodecOperationRequestSchema,
  CodecPemDecodeOptionsSchema,
  CodecPemDecodeRequestSchema,
  CodecPemEncodeOptionsSchema,
  CodecPemEncodeRequestSchema,
  CodecPemLabel,
  CodecPemLineEnding,
} from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import type {
  CodecOperationResult,
  CodecPemDecodeOptions,
  CodecPemDecodeResult,
  CodecPemEncodeOptions,
} from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import {
  clearGeneratedOperationResult,
  processGeneratedOperationRequest,
} from "./operationContract.js";
import {
  ensureBytesInput,
  snapshotBoundedBytesInput,
} from "./readOutput.js";

import { isPemLabel, isPemLineEnding, labelToProto, lineEndingToProto } from "./proto/generated/sdkContract.js";
import type { ReallyMePemLabel, ReallyMePemLineEnding } from "./proto/generated/sdkContract.js";
export type { ReallyMePemLabel, ReallyMePemLineEnding } from "./proto/generated/sdkContract.js";

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
  if (!isPemLabel(label)) throw new ReallyMeCodecError("invalid-input");
  return label;
};

const readPemLabel = (label: string): ReallyMePemLabel => {
  if (!isPemLabel(label)) throw new ReallyMeCodecError("provider-failure");
  return label;
};

const requireNoProviderUnknownFields = (message: Message): void => {
  if (message.$unknown !== undefined && message.$unknown.length !== 0) {
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
  if (lineWidth !== undefined && (lineWidth < 1 || lineWidth > 76)) {
    throw new ReallyMeCodecError("invalid-input");
  }
  const lineEndingValue = readSnapshotProperty(snapshot, "lineEnding");
  let lineEnding: ReallyMePemLineEnding | undefined;
  if (lineEndingValue !== undefined) {
    if (!isPemLineEnding(lineEndingValue)) {
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



const readPemDocument = (result: CodecPemDecodeResult): ReallyMePemDocument => {
  try {
    requireNoProviderUnknownFields(result);
    return {
      label: readPemLabel(result.label),
      der: result.der.slice(),
    };
  } catch (error: unknown) {
    if (error instanceof ReallyMeCodecError) {
      throw error;
    }
    throw new ReallyMeCodecError("provider-failure");
  } finally {
    result.der.fill(0);
  }
};

const pemDecodeOptions = (policy?: ReallyMePemDecodePolicy): CodecPemDecodeOptions | undefined => {
  const snapshot = snapshotDecodePolicy(policy);
  if (snapshot === undefined) {
    return undefined;
  }
  const allowedLabels: CodecPemLabel[] = [];
  if (snapshot.allowedLabels !== undefined) {
    for (const label of snapshot.allowedLabels) {
      allowedLabels.push(labelToProto[requirePemLabel(label)]);
    }
  }
  return create(CodecPemDecodeOptionsSchema, {
    allowedLabels,
    maxInputLen: requireProtoUint32(snapshot.maxInputLen),
    maxDerLen: requireProtoUint32(snapshot.maxDerLen),
  });
};

const pemEncodeOptions = (options?: ReallyMePemEncodeOptions): CodecPemEncodeOptions | undefined => {
  const snapshot = snapshotEncodeOptions(options);
  if (snapshot === undefined) {
    return undefined;
  }
  return create(CodecPemEncodeOptionsSchema, {
    maxDerLen: requireProtoUint32(snapshot.maxDerLen),
    lineWidth: requireProtoUint32(snapshot.lineWidth),
    lineEnding: snapshot.lineEnding === undefined
      ? CodecPemLineEnding.UNSPECIFIED
      : lineEndingToProto[snapshot.lineEnding],
  });
};

export const decodePem = (
  input: Uint8Array,
  policy?: ReallyMePemDecodePolicy,
): ReallyMePemDocument => {
  ensureBytesInput(input);
  const options = pemDecodeOptions(policy);
  const pemSnapshot = snapshotBoundedBytesInput(input, MAX_CODEC_PEM_INPUT_BYTES);
  let operationResult: CodecOperationResult;
  try {
    // Serialized from the generated descriptor into one wiped buffer, so PEM
    // text that may hold a private key leaves no serializer copy behind.
    operationResult = processGeneratedOperationRequest(
      create(CodecOperationRequestSchema, {
        operation: {
          case: "pemDecode",
          value: create(CodecPemDecodeRequestSchema, { pem: pemSnapshot, options }),
        },
      }),
    );
  } finally {
    pemSnapshot.fill(0);
  }
  if (operationResult.result.case !== "pemDecode") {
    clearGeneratedOperationResult(operationResult);
    throw new ReallyMeCodecError("provider-failure");
  }
  return readPemDocument(operationResult.result.value);
};

export const encodePem = (
  label: ReallyMePemLabel,
  der: Uint8Array,
  options?: ReallyMePemEncodeOptions,
): Uint8Array => {
  requirePemLabel(label);
  ensureBytesInput(der);
  const encodeOptions = pemEncodeOptions(options);
  const derSnapshot = snapshotBoundedBytesInput(der);
  let operationResult: CodecOperationResult;
  try {
    // Serialized from the generated descriptor into one wiped buffer, so DER
    // that may hold a private key leaves no serializer copy behind.
    operationResult = processGeneratedOperationRequest(
      create(CodecOperationRequestSchema, {
        operation: {
          case: "pemEncode",
          value: create(CodecPemEncodeRequestSchema, {
            label: labelToProto[label],
            der: derSnapshot,
            options: encodeOptions,
          }),
        },
      }),
    );
  } finally {
    derSnapshot.fill(0);
  }
  if (operationResult.result.case !== "pemEncode") {
    clearGeneratedOperationResult(operationResult);
    throw new ReallyMeCodecError("provider-failure");
  }
  const result = operationResult.result.value;
  try {
    requireNoProviderUnknownFields(result);
    return result.pem.slice();
  } finally {
    result.pem.fill(0);
  }
};
