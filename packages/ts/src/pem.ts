// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { create, toBinary } from "@bufbuild/protobuf";
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
import type { CodecPemDecodeResult } from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import { frameOperationRequest } from "./frameSensitiveRequest.js";
import {
  clearGeneratedOperationResult,
  processOperationRequestBytes,
} from "./operationContract.js";
import {
  ensureBytesInput,
  snapshotBoundedBytesInput,
} from "./readOutput.js";

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

// Field numbers come from the generated schema so framing cannot drift from
// the protobuf contract.
const PEM_DECODE_OPERATION_FIELD = CodecOperationRequestSchema.field.pemDecode.number;
const PEM_ENCODE_OPERATION_FIELD = CodecOperationRequestSchema.field.pemEncode.number;
const PEM_DECODE_PEM_FIELD = CodecPemDecodeRequestSchema.field.pem.number;
const PEM_DECODE_OPTIONS_FIELD = CodecPemDecodeRequestSchema.field.options.number;
const PEM_ENCODE_LABEL_FIELD = CodecPemEncodeRequestSchema.field.label.number;
const PEM_ENCODE_DER_FIELD = CodecPemEncodeRequestSchema.field.der.number;
const PEM_ENCODE_OPTIONS_FIELD = CodecPemEncodeRequestSchema.field.options.number;

// Options carry only labels and limits, never key material, so the generated
// serializer may encode them; the PEM or DER bytes are framed separately.
const serializeOptions = (encode: () => Uint8Array): Uint8Array => {
  try {
    return encode();
  } catch (error: unknown) {
    if (error instanceof ReallyMeCodecError) {
      throw error;
    }
    throw new ReallyMeCodecError("provider-failure");
  }
};

const pemDecodeOptionsBytes = (policy?: ReallyMePemDecodePolicy): Uint8Array | undefined => {
  const snapshot = snapshotDecodePolicy(policy);
  if (snapshot === undefined) {
    return undefined;
  }
  const allowedLabels: CodecPemLabel[] = [];
  if (snapshot.allowedLabels !== undefined) {
    for (const label of snapshot.allowedLabels) {
      allowedLabels.push(protoPemLabel(requirePemLabel(label)));
    }
  }
  const decodeOptions = create(CodecPemDecodeOptionsSchema, {
    allowedLabels,
    maxInputLen: requireProtoUint32(snapshot.maxInputLen),
    maxDerLen: requireProtoUint32(snapshot.maxDerLen),
  });
  return serializeOptions(() => toBinary(CodecPemDecodeOptionsSchema, decodeOptions));
};

const pemEncodeOptionsBytes = (options?: ReallyMePemEncodeOptions): Uint8Array | undefined => {
  const snapshot = snapshotEncodeOptions(options);
  if (snapshot === undefined) {
    return undefined;
  }
  const encodeOptions = create(CodecPemEncodeOptionsSchema, {
    maxDerLen: requireProtoUint32(snapshot.maxDerLen),
    lineWidth: requireProtoUint32(snapshot.lineWidth),
    lineEnding: snapshot.lineEnding === "crlf"
      ? CodecPemLineEnding.CRLF
      : snapshot.lineEnding === "lf"
        ? CodecPemLineEnding.LF
        : CodecPemLineEnding.UNSPECIFIED,
  });
  return serializeOptions(() => toBinary(CodecPemEncodeOptionsSchema, encodeOptions));
};

export const decodePem = (
  input: Uint8Array,
  policy?: ReallyMePemDecodePolicy,
): ReallyMePemDocument => {
  ensureBytesInput(input);
  const optionsBytes = pemDecodeOptionsBytes(policy);
  const pemSnapshot = snapshotBoundedBytesInput(input, MAX_CODEC_PEM_INPUT_BYTES);
  let requestBytes: Uint8Array;
  try {
    requestBytes = frameOperationRequest(PEM_DECODE_OPERATION_FIELD, [
      { field: PEM_DECODE_PEM_FIELD, bytes: pemSnapshot },
      ...(optionsBytes === undefined
        ? []
        : [{ field: PEM_DECODE_OPTIONS_FIELD, message: optionsBytes }]),
    ]);
  } finally {
    pemSnapshot.fill(0);
  }
  const operationResult = processOperationRequestBytes(requestBytes);
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
  const optionsBytes = pemEncodeOptionsBytes(options);
  const derSnapshot = snapshotBoundedBytesInput(der);
  let requestBytes: Uint8Array;
  try {
    requestBytes = frameOperationRequest(PEM_ENCODE_OPERATION_FIELD, [
      { field: PEM_ENCODE_LABEL_FIELD, varint: protoPemLabel(label) },
      { field: PEM_ENCODE_DER_FIELD, bytes: derSnapshot },
      ...(optionsBytes === undefined
        ? []
        : [{ field: PEM_ENCODE_OPTIONS_FIELD, message: optionsBytes }]),
    ]);
  } finally {
    derSnapshot.fill(0);
  }
  const operationResult = processOperationRequestBytes(requestBytes);
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
