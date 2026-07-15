// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { ReallyMeCodecError } from "./errors.js";
import type { ReallyMeCodecErrorCode } from "./errors.js";

type BytesToStringFn = (bytes: Uint8Array) => unknown;
type StringToBytesFn = (text: string) => unknown;
type StringToStringFn = (text: string) => unknown;
type StringToObjectFn = (text: string) => unknown;
type BytesToObjectFn = (bytes: Uint8Array) => unknown;
type BytesToBytesFn = (bytes: Uint8Array) => unknown;
type StringBytesToStringFn = (text: string, bytes: Uint8Array) => unknown;
type StringToBooleanFn = (text: string) => unknown;
type StringStringToBooleanFn = (left: string, right: string) => unknown;
type ValidateKeyBindingFn = (
  bindingType: string,
  algorithm: string | undefined,
  multikey: string,
) => unknown;
type PemDecodeFn = (input: string, optionsJson: string) => unknown;
type PemEncodeFn = (label: string, der: Uint8Array, optionsJson: string) => unknown;
type Function0 = () => unknown;
type StringToObjectOutputFn = (text: string) => unknown;
type BytesToObjectOutputFn = (bytes: Uint8Array) => unknown;
type StringBytesToObjectOutputFn = (text: string, bytes: Uint8Array) => unknown;
type PemDecodeObjectOutputFn = (input: string, optionsJson: string) => unknown;
type WasmArgument = Uint8Array | string | undefined;
type WasmCallable = (...args: ReadonlyArray<WasmArgument>) => unknown;

export const REALLYME_CODEC_WASM_EXPORTS = [
  "base64Decode",
  "base64Encode",
  "base64urlDecode",
  "base64urlEncode",
  "bytesToLowerHex",
  "lowerHexToBytes",
  "base58btcDecode",
  "base58btcEncode",
  "multibaseBase58btcEncode",
  "multibaseBase64urlEncode",
  "multibaseDecode",
  "multicodecLookupPrefix",
  "multicodecLookupPrefixProto",
  "multicodecLookupPrefixProtoResult",
  "multicodecPrefixForName",
  "multicodecPrefixForNameProto",
  "multicodecPrefixForNameProtoResult",
  "multicodecStripPrefix",
  "multicodecTable",
  "multicodecTableProto",
  "multicodecTableProtoResult",
  "multikeyEncode",
  "multikeyParse",
  "multikeyParseProto",
  "multikeyParseProtoResult",
  "bindingTypeMatchesCodec",
  "validateKeyBinding",
  "requireSupportedMulticodec",
  "dagCborCodecCode",
  "dagCborComputeCid",
  "dagCborDecode",
  "dagCborEncode",
  "dagCborMultihash",
  "dagCborSha256ContentHash",
  "dagCborVerifyCid",
  "dagCborVerifyCidProto",
  "dagCborVerifyCidProtoResult",
  "isValidCidString",
  "tryParseCid",
  "canonicalizeJson",
  "pemDecode",
  "pemDecodeProto",
  "pemDecodeProtoResult",
  "pemEncode",
] as const;

export type ReallyMeCodecWasmProvider = Readonly<{
  base64Decode: StringToBytesFn;
  base64Encode: BytesToStringFn;
  base64urlDecode: StringToBytesFn;
  base64urlEncode: BytesToStringFn;
  bytesToLowerHex: BytesToStringFn;
  lowerHexToBytes: StringToBytesFn;
  base58btcDecode: StringToBytesFn;
  base58btcEncode: BytesToStringFn;
  multibaseBase58btcEncode: BytesToStringFn;
  multibaseBase64urlEncode: BytesToStringFn;
  multibaseDecode: StringToBytesFn;
  multicodecLookupPrefix: BytesToObjectFn;
  multicodecLookupPrefixProto: BytesToBytesFn;
  multicodecLookupPrefixProtoResult: BytesToObjectOutputFn;
  multicodecPrefixForName: StringToObjectFn;
  multicodecPrefixForNameProto: StringToBytesFn;
  multicodecPrefixForNameProtoResult: StringToObjectOutputFn;
  multicodecStripPrefix: BytesToBytesFn;
  multicodecTable: Function0;
  multicodecTableProto: Function0;
  multicodecTableProtoResult: Function0;
  multikeyEncode: StringBytesToStringFn;
  multikeyParse: StringToObjectFn;
  multikeyParseProto: StringToBytesFn;
  multikeyParseProtoResult: StringToObjectOutputFn;
  bindingTypeMatchesCodec: StringStringToBooleanFn;
  validateKeyBinding: ValidateKeyBindingFn;
  requireSupportedMulticodec: StringToStringFn;
  dagCborCodecCode: Function0;
  dagCborComputeCid: BytesToStringFn;
  dagCborDecode: BytesToStringFn;
  dagCborEncode: StringToBytesFn;
  dagCborMultihash: BytesToBytesFn;
  dagCborSha256ContentHash: BytesToBytesFn;
  dagCborVerifyCid: StringBytesToStringFn;
  dagCborVerifyCidProto: StringBytesToStringFn;
  dagCborVerifyCidProtoResult: StringBytesToObjectOutputFn;
  isValidCidString: StringToBooleanFn;
  tryParseCid: StringToObjectFn;
  canonicalizeJson: StringToStringFn;
  pemDecode: PemDecodeFn;
  pemDecodeProto: PemDecodeFn;
  pemDecodeProtoResult: PemDecodeObjectOutputFn;
  pemEncode: PemEncodeFn;
}>;

let installedProvider: ReallyMeCodecWasmProvider | undefined;

const wasmErrorCode = (error: unknown): ReallyMeCodecErrorCode => {
  switch (error) {
    case "invalid-input":
      return "invalid-input";
    case "non-canonical":
      return "non-canonical";
    case "unsupported-codec":
      return "unsupported-codec";
    case "provider-failure":
    default:
      return "provider-failure";
  }
};

const requireObject = (module: unknown): object => {
  if (typeof module !== "object" || module === null) {
    throw new ReallyMeCodecError("provider-failure");
  }
  return module;
};

const requireFunction = (module: object, name: string): WasmCallable => {
  const candidate: unknown = Reflect.get(module, name);
  if (typeof candidate !== "function") {
    throw new ReallyMeCodecError("provider-failure");
  }
  return (...args: ReadonlyArray<WasmArgument>): unknown => {
    try {
      return candidate(...args);
    } catch (error: unknown) {
      throw new ReallyMeCodecError(wasmErrorCode(error));
    }
  };
};

const function0 = (module: object, name: string): Function0 => {
  const callable = requireFunction(module, name);
  return (): unknown => callable();
};

const stringFunction1 = (module: object, name: string): StringToBytesFn => {
  const callable = requireFunction(module, name);
  return (text: string): unknown => callable(text);
};

const bytesFunction1 = (module: object, name: string): BytesToStringFn => {
  const callable = requireFunction(module, name);
  return (bytes: Uint8Array): unknown => callable(bytes);
};

const stringBytesFunction2 = (module: object, name: string): StringBytesToStringFn => {
  const callable = requireFunction(module, name);
  return (text: string, bytes: Uint8Array): unknown => callable(text, bytes);
};

const stringStringBooleanFunction = (
  module: object,
  name: string,
): StringStringToBooleanFn => {
  const callable = requireFunction(module, name);
  return (left: string, right: string): unknown => callable(left, right);
};

const stringBooleanFunction = (module: object, name: string): StringToBooleanFn => {
  const callable = requireFunction(module, name);
  return (text: string): unknown => callable(text);
};

const validateKeyBindingFunction = (
  module: object,
  name: string,
): ValidateKeyBindingFn => {
  const callable = requireFunction(module, name);
  return (
    bindingType: string,
    algorithm: string | undefined,
    multikey: string,
  ): unknown => callable(bindingType, algorithm, multikey);
};

const pemDecodeFunction = (module: object, name: string): PemDecodeFn => {
  const callable = requireFunction(module, name);
  return (input: string, optionsJson: string): unknown => callable(input, optionsJson);
};

const pemEncodeFunction = (module: object, name: string): PemEncodeFn => {
  const callable = requireFunction(module, name);
  return (label: string, der: Uint8Array, optionsJson: string): unknown =>
    callable(label, der, optionsJson);
};

export const installReallyMeCodecWasmProvider = (module: unknown): void => {
  if (installedProvider !== undefined) {
    throw new ReallyMeCodecError("provider-failure");
  }
  const providerModule = requireObject(module);
  installedProvider = {
    base64Decode: stringFunction1(providerModule, "base64Decode"),
    base64Encode: bytesFunction1(providerModule, "base64Encode"),
    base64urlDecode: stringFunction1(providerModule, "base64urlDecode"),
    base64urlEncode: bytesFunction1(providerModule, "base64urlEncode"),
    bytesToLowerHex: bytesFunction1(providerModule, "bytesToLowerHex"),
    lowerHexToBytes: stringFunction1(providerModule, "lowerHexToBytes"),
    base58btcDecode: stringFunction1(providerModule, "base58btcDecode"),
    base58btcEncode: bytesFunction1(providerModule, "base58btcEncode"),
    multibaseBase58btcEncode: bytesFunction1(
      providerModule,
      "multibaseBase58btcEncode",
    ),
    multibaseBase64urlEncode: bytesFunction1(
      providerModule,
      "multibaseBase64urlEncode",
    ),
    multibaseDecode: stringFunction1(providerModule, "multibaseDecode"),
    multicodecLookupPrefix: bytesFunction1(providerModule, "multicodecLookupPrefix"),
    multicodecLookupPrefixProto: bytesFunction1(
      providerModule,
      "multicodecLookupPrefixProto",
    ),
    multicodecLookupPrefixProtoResult: bytesFunction1(
      providerModule,
      "multicodecLookupPrefixProtoResult",
    ),
    multicodecPrefixForName: stringFunction1(providerModule, "multicodecPrefixForName"),
    multicodecPrefixForNameProto: stringFunction1(
      providerModule,
      "multicodecPrefixForNameProto",
    ),
    multicodecPrefixForNameProtoResult: stringFunction1(
      providerModule,
      "multicodecPrefixForNameProtoResult",
    ),
    multicodecStripPrefix: bytesFunction1(providerModule, "multicodecStripPrefix"),
    multicodecTable: function0(providerModule, "multicodecTable"),
    multicodecTableProto: function0(providerModule, "multicodecTableProto"),
    multicodecTableProtoResult: function0(providerModule, "multicodecTableProtoResult"),
    multikeyEncode: stringBytesFunction2(providerModule, "multikeyEncode"),
    multikeyParse: stringFunction1(providerModule, "multikeyParse"),
    multikeyParseProto: stringFunction1(providerModule, "multikeyParseProto"),
    multikeyParseProtoResult: stringFunction1(
      providerModule,
      "multikeyParseProtoResult",
    ),
    bindingTypeMatchesCodec: stringStringBooleanFunction(
      providerModule,
      "bindingTypeMatchesCodec",
    ),
    validateKeyBinding: validateKeyBindingFunction(providerModule, "validateKeyBinding"),
    requireSupportedMulticodec: stringFunction1(
      providerModule,
      "requireSupportedMulticodec",
    ),
    dagCborCodecCode: function0(providerModule, "dagCborCodecCode"),
    dagCborComputeCid: bytesFunction1(providerModule, "dagCborComputeCid"),
    dagCborDecode: bytesFunction1(providerModule, "dagCborDecode"),
    dagCborEncode: stringFunction1(providerModule, "dagCborEncode"),
    dagCborMultihash: bytesFunction1(providerModule, "dagCborMultihash"),
    dagCborSha256ContentHash: bytesFunction1(
      providerModule,
      "dagCborSha256ContentHash",
    ),
    dagCborVerifyCid: stringBytesFunction2(providerModule, "dagCborVerifyCid"),
    dagCborVerifyCidProto: stringBytesFunction2(
      providerModule,
      "dagCborVerifyCidProto",
    ),
    dagCborVerifyCidProtoResult: stringBytesFunction2(
      providerModule,
      "dagCborVerifyCidProtoResult",
    ),
    isValidCidString: stringBooleanFunction(providerModule, "isValidCidString"),
    tryParseCid: stringFunction1(providerModule, "tryParseCid"),
    canonicalizeJson: stringFunction1(providerModule, "canonicalizeJson"),
    pemDecode: pemDecodeFunction(providerModule, "pemDecode"),
    pemDecodeProto: pemDecodeFunction(providerModule, "pemDecodeProto"),
    pemDecodeProtoResult: pemDecodeFunction(providerModule, "pemDecodeProtoResult"),
    pemEncode: pemEncodeFunction(providerModule, "pemEncode"),
  };
};

export const requireReallyMeCodecWasmProvider = (): ReallyMeCodecWasmProvider => {
  if (installedProvider === undefined) {
    throw new ReallyMeCodecError("provider-failure");
  }
  return installedProvider;
};
