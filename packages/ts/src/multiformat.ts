// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { create } from "@bufbuild/protobuf";
import type { Message } from "@bufbuild/protobuf";

import { ReallyMeCodecError } from "./errors.js";
import {
  CodecMulticodecLookupPrefixRequestSchema,
  CodecMulticodecPrefixForNameRequestSchema,
  CodecMulticodecTableRequestSchema,
  CodecMultikeyParseRequestSchema,
  CodecOperationRequestSchema,
  CodecKeyMaterialKind,
  CodecTag,
} from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import {
  clearGeneratedOperationResult,
  processGeneratedOperationRequest,
} from "./operationContract.js";
import {
  ensureBytesInput,
  ensureStringInput,
  ensureStringValue,
  readBooleanOutput,
  readBytesOutput,
  readStringOutput,
  snapshotBoundedBytesInput,
} from "./readOutput.js";
import { requireReallyMeCodecWasmProvider } from "./wasmProvider.js";

export type ReallyMeMulticodecTag =
  | "encryption"
  | "hash"
  | "key"
  | "multihash"
  | "multikey";

export type ReallyMeKeyMaterialKind =
  | "not-key"
  | "public-key"
  | "private-key"
  | "symmetric-key";

export type ReallyMeMulticodecMetadata = Readonly<{
  name: string;
  alg: string;
  tag: ReallyMeMulticodecTag;
  keyMaterial: ReallyMeKeyMaterialKind;
  prefix: Uint8Array;
  expectedKeyLength?: number;
}>;

export type ReallyMeMulticodecLookupResult = Readonly<{
  name: string;
  prefixLength: number;
  metadata: ReallyMeMulticodecMetadata;
}>;

export type ReallyMeMulticodecTable = Readonly<{
  entries: ReadonlyArray<ReallyMeMulticodecMetadata>;
}>;

export type ReallyMeParsedMultikey = Readonly<{
  codecName: string;
  algorithmName: string;
  publicKey: Uint8Array;
  expectedPublicKeyLength?: number;
}>;

const MAX_MULTICODEC_TABLE_ENTRIES = 1_024;

const requireNoProviderUnknownFields = (message: Message): void => {
  if (message.$unknown !== undefined && message.$unknown.length !== 0) {
    throw new ReallyMeCodecError("provider-failure");
  }
};

const sdkTag = (tag: CodecTag): ReallyMeMulticodecTag => {
  switch (tag) {
    case CodecTag.ENCRYPTION:
      return "encryption";
    case CodecTag.HASH:
      return "hash";
    case CodecTag.KEY:
      return "key";
    case CodecTag.MULTIHASH:
      return "multihash";
    case CodecTag.MULTIKEY:
      return "multikey";
    case CodecTag.UNSPECIFIED:
    default:
      throw new ReallyMeCodecError("provider-failure");
  }
};

const sdkKeyMaterial = (
  keyMaterial: CodecKeyMaterialKind,
): ReallyMeKeyMaterialKind => {
  switch (keyMaterial) {
    case CodecKeyMaterialKind.NOT_KEY:
      return "not-key";
    case CodecKeyMaterialKind.PUBLIC_KEY:
      return "public-key";
    case CodecKeyMaterialKind.PRIVATE_KEY:
      return "private-key";
    case CodecKeyMaterialKind.SYMMETRIC_KEY:
      return "symmetric-key";
    case CodecKeyMaterialKind.UNSPECIFIED:
    default:
      throw new ReallyMeCodecError("provider-failure");
  }
};

const readMulticodecMetadata = (
  value: Message & {
    name: string;
    algorithmName: string;
    tag: CodecTag;
    keyMaterialKind: CodecKeyMaterialKind;
    prefix: Uint8Array;
    fixedLength: number;
    variableLength: boolean;
  },
): ReallyMeMulticodecMetadata => {
  validateMulticodecMetadata(value);
  const metadata = {
    name: value.name,
    alg: value.algorithmName,
    tag: sdkTag(value.tag),
    keyMaterial: sdkKeyMaterial(value.keyMaterialKind),
    prefix: value.prefix.slice(),
  };
  if (value.variableLength || value.fixedLength === 0) {
    return metadata;
  }
  return { ...metadata, expectedKeyLength: value.fixedLength };
};

const validateMulticodecMetadata = (
  value: Message & {
    tag: CodecTag;
    keyMaterialKind: CodecKeyMaterialKind;
  },
): void => {
  requireNoProviderUnknownFields(value);
  sdkTag(value.tag);
  sdkKeyMaterial(value.keyMaterialKind);
};

export const base58btcEncode = (bytes: Uint8Array): string => {
  ensureBytesInput(bytes);
  return readStringOutput(requireReallyMeCodecWasmProvider().base58btcEncode(bytes));
};

export const base58btcDecode = (encoded: string): Uint8Array => {
  ensureStringValue(encoded);
  return readBytesOutput(requireReallyMeCodecWasmProvider().base58btcDecode(encoded));
};

export const multibaseBase64urlEncode = (bytes: Uint8Array): string => {
  ensureBytesInput(bytes);
  return readStringOutput(
    requireReallyMeCodecWasmProvider().multibaseBase64urlEncode(bytes),
  );
};

export const multibaseBase58btcEncode = (bytes: Uint8Array): string => {
  ensureBytesInput(bytes);
  return readStringOutput(
    requireReallyMeCodecWasmProvider().multibaseBase58btcEncode(bytes),
  );
};

export const multibaseDecode = (encoded: string): Uint8Array => {
  ensureStringInput(encoded);
  return readBytesOutput(requireReallyMeCodecWasmProvider().multibaseDecode(encoded));
};

export const multicodecPrefixForName = (
  codecName: string,
): ReallyMeMulticodecMetadata => {
  ensureStringInput(codecName);
  const result = processGeneratedOperationRequest(
    multicodecPrefixForNameRequest(codecName),
  );
  if (result.result.case !== "multicodecPrefixForName") {
    clearGeneratedOperationResult(result);
    throw new ReallyMeCodecError("provider-failure");
  }
  try {
    return readMulticodecMetadata(result.result.value);
  } finally {
    result.result.value.prefix.fill(0);
  }
};

const multicodecPrefixForNameRequest = (
  codecName: string,
) => create(CodecOperationRequestSchema, {
    operation: {
      case: "multicodecPrefixForName",
      value: create(CodecMulticodecPrefixForNameRequestSchema, {
        name: codecName,
      }),
    },
  });

export const multicodecLookupPrefix = (
  bytes: Uint8Array,
): ReallyMeMulticodecLookupResult => {
  ensureBytesInput(bytes);
  const bytesSnapshot = snapshotBoundedBytesInput(bytes);
  try {
    const operationResult = processGeneratedOperationRequest(
      multicodecLookupPrefixRequest(bytesSnapshot),
    );
    if (operationResult.result.case !== "multicodecLookupPrefix") {
      clearGeneratedOperationResult(operationResult);
      throw new ReallyMeCodecError("provider-failure");
    }
    const result = operationResult.result.value;
    try {
      requireNoProviderUnknownFields(result);
      if (result.metadata === undefined) {
        throw new ReallyMeCodecError("provider-failure");
      }
      return {
        name: result.name,
        prefixLength: result.prefixLength,
        metadata: readMulticodecMetadata(result.metadata),
      };
    } finally {
      result.metadata?.prefix.fill(0);
    }
  } finally {
    bytesSnapshot.fill(0);
  }
};

const multicodecLookupPrefixRequest = (
  bytes: Uint8Array,
) => create(CodecOperationRequestSchema, {
    operation: {
      case: "multicodecLookupPrefix",
      value: create(CodecMulticodecLookupPrefixRequestSchema, {
        value: bytes,
      }),
    },
  });

export const multicodecStripPrefix = (bytes: Uint8Array): Uint8Array => {
  ensureBytesInput(bytes);
  return readBytesOutput(requireReallyMeCodecWasmProvider().multicodecStripPrefix(bytes));
};

export const multicodecTable = (): ReallyMeMulticodecTable => {
  const operationResult = processGeneratedOperationRequest(multicodecTableRequest());
  if (operationResult.result.case !== "multicodecTable") {
    clearGeneratedOperationResult(operationResult);
    throw new ReallyMeCodecError("provider-failure");
  }
  const result = operationResult.result.value;
  try {
    requireNoProviderUnknownFields(result);
    if (result.entries.length > MAX_MULTICODEC_TABLE_ENTRIES) {
      throw new ReallyMeCodecError("provider-failure");
    }
    // Validate every entry before making caller-owned byte copies. If one
    // entry is malformed, no earlier prefix copy can become unreachable
    // without an explicit wipe while the generated response is rejected.
    for (const entry of result.entries) {
      validateMulticodecMetadata(entry);
    }
    return { entries: result.entries.map((entry) => readMulticodecMetadata(entry)) };
  } finally {
    for (const entry of result.entries) {
      entry.prefix.fill(0);
    }
  }
};

const multicodecTableRequest = () => create(CodecOperationRequestSchema, {
    operation: {
      case: "multicodecTable",
      value: create(CodecMulticodecTableRequestSchema),
    },
  });

export const multikeyEncode = (codecName: string, publicKey: Uint8Array): string => {
  ensureStringInput(codecName);
  ensureBytesInput(publicKey);
  return readStringOutput(
    requireReallyMeCodecWasmProvider().multikeyEncode(codecName, publicKey),
  );
};

export const multikeyParse = (multikey: string): ReallyMeParsedMultikey => {
  ensureStringInput(multikey);
  const operationResult = processGeneratedOperationRequest(multikeyParseRequest(multikey));
  if (operationResult.result.case !== "multikeyParse") {
    clearGeneratedOperationResult(operationResult);
    throw new ReallyMeCodecError("provider-failure");
  }
  const result = operationResult.result.value;
  try {
    requireNoProviderUnknownFields(result);
    const parsed = {
      codecName: result.codecName,
      algorithmName: result.algorithmName,
      publicKey: result.publicKey.slice(),
    };
    if (result.variablePublicKeyLength || result.expectedPublicKeyLength === 0) {
      return parsed;
    }
    return { ...parsed, expectedPublicKeyLength: result.expectedPublicKeyLength };
  } finally {
    result.publicKey.fill(0);
  }
};

const multikeyParseRequest = (
  multikey: string,
) => create(CodecOperationRequestSchema, {
    operation: {
      case: "multikeyParse",
      value: create(CodecMultikeyParseRequestSchema, { multikey }),
    },
  });

export const bindingTypeMatchesCodec = (
  bindingType: string,
  codecName: string,
): boolean => {
  ensureStringInput(bindingType);
  ensureStringInput(codecName);
  return readBooleanOutput(
    requireReallyMeCodecWasmProvider().bindingTypeMatchesCodec(bindingType, codecName),
  );
};

export const validateKeyBinding = (
  bindingType: string,
  algorithm: string | undefined,
  multikey: string,
): void => {
  ensureStringInput(bindingType);
  if (algorithm !== undefined) {
    ensureStringInput(algorithm);
  }
  ensureStringInput(multikey);
  requireReallyMeCodecWasmProvider().validateKeyBinding(bindingType, algorithm, multikey);
};

export const requireSupportedMulticodec = (codecName: string): void => {
  ensureStringInput(codecName);
  requireReallyMeCodecWasmProvider().requireSupportedMulticodec(codecName);
};
