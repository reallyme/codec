// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

// Generated from reallyme/codec/v1/codec.proto. Do not edit.

import {
  CodecKeyMaterialKind,
  CodecPemLabel,
  CodecPemLineEnding,
  CodecSdkErrorCategory,
  CodecTag,
} from "./reallyme/codec/v1/codec_pb.js";

const projectMulticodecTagValues = {
  [CodecTag.ENCRYPTION]: "encryption",
  [CodecTag.HASH]: "hash",
  [CodecTag.KEY]: "key",
  [CodecTag.MULTIHASH]: "multihash",
  [CodecTag.MULTIKEY]: "multikey",
} as const;
export type ReallyMeMulticodecTag = (typeof projectMulticodecTagValues)[keyof typeof projectMulticodecTagValues];

export function projectMulticodecTag(value: CodecTag): ReallyMeMulticodecTag | undefined {
  switch (value) {
    case CodecTag.ENCRYPTION: return projectMulticodecTagValues[value];
    case CodecTag.HASH: return projectMulticodecTagValues[value];
    case CodecTag.KEY: return projectMulticodecTagValues[value];
    case CodecTag.MULTIHASH: return projectMulticodecTagValues[value];
    case CodecTag.MULTIKEY: return projectMulticodecTagValues[value];
    default: return undefined;
  }
}

const projectKeyMaterialKindValues = {
  [CodecKeyMaterialKind.NOT_KEY]: "not-key",
  [CodecKeyMaterialKind.PUBLIC_KEY]: "public-key",
  [CodecKeyMaterialKind.PRIVATE_KEY]: "private-key",
  [CodecKeyMaterialKind.SYMMETRIC_KEY]: "symmetric-key",
} as const;
export type ReallyMeKeyMaterialKind = (typeof projectKeyMaterialKindValues)[keyof typeof projectKeyMaterialKindValues];

export function projectKeyMaterialKind(value: CodecKeyMaterialKind): ReallyMeKeyMaterialKind | undefined {
  switch (value) {
    case CodecKeyMaterialKind.NOT_KEY: return projectKeyMaterialKindValues[value];
    case CodecKeyMaterialKind.PUBLIC_KEY: return projectKeyMaterialKindValues[value];
    case CodecKeyMaterialKind.PRIVATE_KEY: return projectKeyMaterialKindValues[value];
    case CodecKeyMaterialKind.SYMMETRIC_KEY: return projectKeyMaterialKindValues[value];
    default: return undefined;
  }
}

export const labelToProto = {
  "PRIVATE KEY": CodecPemLabel.PRIVATE_KEY,
  "EC PRIVATE KEY": CodecPemLabel.EC_PRIVATE_KEY,
  "PUBLIC KEY": CodecPemLabel.PUBLIC_KEY,
} as const;
export type ReallyMePemLabel = keyof typeof labelToProto;

export function isPemLabel(value: unknown): value is ReallyMePemLabel {
  return typeof value === "string" && Object.hasOwn(labelToProto, value);
}

export const lineEndingToProto = {
  "lf": CodecPemLineEnding.LF,
  "crlf": CodecPemLineEnding.CRLF,
} as const;
export type ReallyMePemLineEnding = keyof typeof lineEndingToProto;

export function isPemLineEnding(value: unknown): value is ReallyMePemLineEnding {
  return typeof value === "string" && Object.hasOwn(lineEndingToProto, value);
}

const sdkErrorCodes = {
  [CodecSdkErrorCategory.INVALID_INPUT]: "invalid-input",
  [CodecSdkErrorCategory.UNSUPPORTED_CODEC]: "unsupported-codec",
  [CodecSdkErrorCategory.UNSUPPORTED_IPLD_VALUE]: "unsupported-ipld-value",
  [CodecSdkErrorCategory.NON_CANONICAL]: "non-canonical",
  [CodecSdkErrorCategory.PROVIDER_FAILURE]: "provider-failure",
} as const;
export type ReallyMeCodecErrorCode = (typeof sdkErrorCodes)[keyof typeof sdkErrorCodes];

const codeValues: ReadonlySet<unknown> = new Set(Object.values(sdkErrorCodes));
export function isReallyMeCodecErrorCode(value: unknown): value is ReallyMeCodecErrorCode {
  return codeValues.has(value);
}
