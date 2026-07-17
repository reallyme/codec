// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { create } from "@bufbuild/protobuf";

import { ReallyMeCodecError } from "./errors.js";
import {
  CodecMulticodecLookupPrefixRequestSchema,
  CodecMulticodecPrefixForNameRequestSchema,
  CodecMulticodecTableRequestSchema,
  CodecMultikeyParseRequestSchema,
  CodecOperationRequestSchema,
} from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import {
  processGeneratedProtoRequest,
  protoPayloadOrThrow,
} from "./protoProcess.js";
import {
  ensureBytesInput,
  ensureStringInput,
  ensureStringValue,
  readBooleanOutput,
  readBytesOutput,
  readBytesProperty,
  readObjectOutput,
  readOptionalLengthProperty,
  readStringOutput,
  readStringProperty,
} from "./readOutput.js";
import type { ReallyMeCodecProtoResult } from "./readOutput.js";
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

export type ReallyMeParsedMultikey = Readonly<{
  codecName: string;
  algorithmName: string;
  publicKey: Uint8Array;
  expectedPublicKeyLength?: number;
}>;

const validTags: ReadonlySet<string> = new Set([
  "encryption",
  "hash",
  "key",
  "multihash",
  "multikey",
]);

const validKeyMaterialKinds: ReadonlySet<string> = new Set([
  "not-key",
  "public-key",
  "private-key",
  "symmetric-key",
]);

const MAX_MULTICODEC_TABLE_ENTRIES = 1_024;

const readTag = (object: object): ReallyMeMulticodecTag => {
  const tag = readStringProperty(object, "tag");
  if (!validTags.has(tag)) {
    throw new ReallyMeCodecError("provider-failure");
  }
  switch (tag) {
    case "encryption":
    case "hash":
    case "key":
    case "multihash":
    case "multikey":
      return tag;
    default:
      throw new ReallyMeCodecError("provider-failure");
  }
};

const readKeyMaterial = (object: object): ReallyMeKeyMaterialKind => {
  const keyMaterial = readStringProperty(object, "keyMaterial");
  if (!validKeyMaterialKinds.has(keyMaterial)) {
    throw new ReallyMeCodecError("provider-failure");
  }
  switch (keyMaterial) {
    case "not-key":
    case "public-key":
    case "private-key":
    case "symmetric-key":
      return keyMaterial;
    default:
      throw new ReallyMeCodecError("provider-failure");
  }
};

export const readMulticodecMetadata = (
  value: unknown,
): ReallyMeMulticodecMetadata => {
  const object = readObjectOutput(value);
  const expectedKeyLength = readOptionalLengthProperty(object, "expectedKeyLength");
  const metadata = {
    name: readStringProperty(object, "name"),
    alg: readStringProperty(object, "alg"),
    tag: readTag(object),
    keyMaterial: readKeyMaterial(object),
    prefix: readBytesProperty(object, "prefix"),
  };
  if (expectedKeyLength === undefined) {
    return metadata;
  }
  return { ...metadata, expectedKeyLength };
};

const readParsedMultikey = (value: unknown): ReallyMeParsedMultikey => {
  const object = readObjectOutput(value);
  const expectedPublicKeyLength = readOptionalLengthProperty(
    object,
    "expectedPublicKeyLength",
  );
  const parsed = {
    codecName: readStringProperty(object, "codecName"),
    algorithmName: readStringProperty(object, "algorithmName"),
    publicKey: readBytesProperty(object, "publicKey"),
  };
  if (expectedPublicKeyLength === undefined) {
    return parsed;
  }
  return { ...parsed, expectedPublicKeyLength };
};

const readMulticodecTable = (value: unknown): ReadonlyArray<ReallyMeMulticodecMetadata> => {
  try {
    if (
      !Array.isArray(value) ||
      Object.getPrototypeOf(value) !== Array.prototype ||
      value.length > MAX_MULTICODEC_TABLE_ENTRIES
    ) {
      throw new ReallyMeCodecError("provider-failure");
    }
    const table: ReallyMeMulticodecMetadata[] = [];
    for (let index = 0; index < value.length; index += 1) {
      const descriptor = Object.getOwnPropertyDescriptor(value, String(index));
      if (descriptor === undefined || !("value" in descriptor)) {
        throw new ReallyMeCodecError("provider-failure");
      }
      table.push(readMulticodecMetadata(descriptor.value));
    }
    return table;
  } catch (error: unknown) {
    if (error instanceof ReallyMeCodecError) {
      throw error;
    }
    throw new ReallyMeCodecError("provider-failure");
  }
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
  return readMulticodecMetadata(
    requireReallyMeCodecWasmProvider().multicodecPrefixForName(codecName),
  );
};

export const multicodecPrefixForNameProto = (codecName: string): Uint8Array => {
  return protoPayloadOrThrow(multicodecPrefixForNameProtoResult(codecName));
};

export const multicodecPrefixForNameProtoResult = (
  codecName: string,
): ReallyMeCodecProtoResult => {
  ensureStringInput(codecName);
  return processGeneratedProtoRequest(create(CodecOperationRequestSchema, {
    operation: {
      case: "multicodecPrefixForName",
      value: create(CodecMulticodecPrefixForNameRequestSchema, {
        name: codecName,
      }),
    },
  }));
};

export const multicodecLookupPrefix = (
  bytes: Uint8Array,
): ReallyMeMulticodecMetadata => {
  ensureBytesInput(bytes);
  return readMulticodecMetadata(
    requireReallyMeCodecWasmProvider().multicodecLookupPrefix(bytes),
  );
};

export const multicodecLookupPrefixProto = (bytes: Uint8Array): Uint8Array => {
  return protoPayloadOrThrow(multicodecLookupPrefixProtoResult(bytes));
};

export const multicodecLookupPrefixProtoResult = (
  bytes: Uint8Array,
): ReallyMeCodecProtoResult => {
  ensureBytesInput(bytes);
  return processGeneratedProtoRequest(create(CodecOperationRequestSchema, {
    operation: {
      case: "multicodecLookupPrefix",
      value: create(CodecMulticodecLookupPrefixRequestSchema, {
        value: bytes,
      }),
    },
  }));
};

export const multicodecStripPrefix = (bytes: Uint8Array): Uint8Array => {
  ensureBytesInput(bytes);
  return readBytesOutput(requireReallyMeCodecWasmProvider().multicodecStripPrefix(bytes));
};

export const multicodecTable = (): ReadonlyArray<ReallyMeMulticodecMetadata> =>
  readMulticodecTable(requireReallyMeCodecWasmProvider().multicodecTable());

export const multicodecTableProto = (): Uint8Array =>
  protoPayloadOrThrow(multicodecTableProtoResult());

export const multicodecTableProtoResult = (): ReallyMeCodecProtoResult =>
  processGeneratedProtoRequest(create(CodecOperationRequestSchema, {
    operation: {
      case: "multicodecTable",
      value: create(CodecMulticodecTableRequestSchema),
    },
  }));

export const multikeyEncode = (codecName: string, publicKey: Uint8Array): string => {
  ensureStringInput(codecName);
  ensureBytesInput(publicKey);
  return readStringOutput(
    requireReallyMeCodecWasmProvider().multikeyEncode(codecName, publicKey),
  );
};

export const multikeyParse = (multikey: string): ReallyMeParsedMultikey => {
  ensureStringInput(multikey);
  return readParsedMultikey(requireReallyMeCodecWasmProvider().multikeyParse(multikey));
};

export const multikeyParseProto = (multikey: string): Uint8Array => {
  return protoPayloadOrThrow(multikeyParseProtoResult(multikey));
};

export const multikeyParseProtoResult = (
  multikey: string,
): ReallyMeCodecProtoResult => {
  ensureStringInput(multikey);
  return processGeneratedProtoRequest(create(CodecOperationRequestSchema, {
    operation: {
      case: "multikeyParse",
      value: create(CodecMultikeyParseRequestSchema, { multikey }),
    },
  }));
};

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
