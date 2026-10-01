// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { ReallyMeCodecError } from "./errors.js";
import type { ReallyMeCodecErrorCode } from "./errors.js";
import * as bundledWasm from "../dist/wasm/reallyme_codec_wasm.js";

type BytesToStringFn = (bytes: Uint8Array) => unknown;
type StringToBytesFn = (text: string) => unknown;
type StringToStringFn = (text: string) => unknown;
type StringToUnknownFn = (text: string) => unknown;
type BytesToBytesFn = (bytes: Uint8Array) => unknown;
type StringBytesToStringFn = (text: string, bytes: Uint8Array) => unknown;
type StringToBooleanFn = (text: string) => unknown;
type StringStringToBooleanFn = (left: string, right: string) => unknown;
type ValidateKeyBindingFn = (
  bindingType: string,
  algorithm: string | undefined,
  multikey: string,
) => unknown;
type Function0 = () => unknown;
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
  "multicodecStripPrefix",
  "multikeyEncode",
  "bindingTypeMatchesCodec",
  "validateKeyBinding",
  "requireSupportedMulticodec",
  "dagCborCodecCode",
  "dagCborComputeCid",
  "dagCborMultihash",
  "dagCborSha256ContentHash",
  "isValidCidString",
  "tryParseCid",
  "canonicalizeJson",
  "processOperation",
  "processOperationJson",
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
  multicodecStripPrefix: BytesToBytesFn;
  multikeyEncode: StringBytesToStringFn;
  bindingTypeMatchesCodec: StringStringToBooleanFn;
  validateKeyBinding: ValidateKeyBindingFn;
  requireSupportedMulticodec: StringToStringFn;
  dagCborCodecCode: Function0;
  dagCborComputeCid: BytesToStringFn;
  dagCborMultihash: BytesToBytesFn;
  dagCborSha256ContentHash: BytesToBytesFn;
  isValidCidString: StringToBooleanFn;
  tryParseCid: StringToUnknownFn;
  canonicalizeJson: StringToStringFn;
  processOperation: BytesToBytesFn;
  processOperationJson: BytesToBytesFn;
}>;

let installedProvider: ReallyMeCodecWasmProvider | undefined;
let providerPoisoned = false;
const DAG_CBOR_CODEC_CODE = 0x71;

const wasmErrorCode = (error: unknown): ReallyMeCodecErrorCode | undefined => {
  switch (error) {
    case "invalid-input":
      return "invalid-input";
    case "non-canonical":
      return "non-canonical";
    case "unsupported-codec":
      return "unsupported-codec";
    case "unsupported-ipld-value":
      return "unsupported-ipld-value";
    case "provider-failure":
      return "provider-failure";
    default:
      return undefined;
  }
};

const isWasmRuntimeError = (error: unknown): boolean => {
  const wasm: unknown = Reflect.get(globalThis, "WebAssembly");
  if (typeof wasm !== "object" || wasm === null) {
    return false;
  }
  const runtimeError: unknown = Reflect.get(wasm, "RuntimeError");
  return typeof runtimeError === "function" && error instanceof runtimeError;
};

const requireObject = (module: unknown): object => {
  if (typeof module !== "object" || module === null) {
    throw new ReallyMeCodecError("provider-failure");
  }
  return module;
};

const requireFunction = (module: object, name: string): WasmCallable => {
  let candidate: unknown;
  try {
    const descriptor = Object.getOwnPropertyDescriptor(module, name);
    if (descriptor === undefined || !("value" in descriptor)) {
      throw new ReallyMeCodecError("provider-failure");
    }
    candidate = descriptor.value;
  } catch (error: unknown) {
    if (error instanceof ReallyMeCodecError) {
      throw error;
    }
    throw new ReallyMeCodecError("provider-failure");
  }
  if (typeof candidate !== "function") {
    throw new ReallyMeCodecError("provider-failure");
  }
  return (...args: ReadonlyArray<WasmArgument>): unknown => {
    if (providerPoisoned) {
      throw new ReallyMeCodecError("provider-failure");
    }
    try {
      return candidate(...args);
    } catch (error: unknown) {
      const code = wasmErrorCode(error);
      if (code === undefined) {
        // A WASM trap may leave the instance partially unwound. Host-side
        // errors do not prove that the instance is unusable.
        if (isWasmRuntimeError(error)) {
          providerPoisoned = true;
        }
        throw new ReallyMeCodecError("provider-failure");
      }
      throw new ReallyMeCodecError(code);
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

export const installReallyMeCodecWasmProvider = (module: unknown): void => {
  if (installedProvider !== undefined) {
    throw new ReallyMeCodecError("provider-failure");
  }
  // The generated ES module namespace is immutable. Identity pins the public
  // installer to the Rust build shipped in this package instead of accepting
  // a structurally compatible object supplied by another script.
  if (module !== bundledWasm) {
    throw new ReallyMeCodecError("provider-failure");
  }
  const providerModule = requireObject(module);
  // This zero-argument export accesses the initialized WASM module but does
  // not handle caller data. Reject pre-init installation before making the
  // install-once provider visible; a later initialized attempt can succeed.
  const probe = Object.getOwnPropertyDescriptor(providerModule, "dagCborCodecCode")?.value;
  if (typeof probe !== "function") {
    throw new ReallyMeCodecError("provider-failure");
  }
  let codecCode: unknown;
  try {
    codecCode = probe();
  } catch {
    throw new ReallyMeCodecError("provider-failure");
  }
  if (codecCode !== DAG_CBOR_CODEC_CODE) {
    throw new ReallyMeCodecError("provider-failure");
  }
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
    multicodecStripPrefix: bytesFunction1(providerModule, "multicodecStripPrefix"),
    multikeyEncode: stringBytesFunction2(providerModule, "multikeyEncode"),
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
    dagCborMultihash: bytesFunction1(providerModule, "dagCborMultihash"),
    dagCborSha256ContentHash: bytesFunction1(
      providerModule,
      "dagCborSha256ContentHash",
    ),
    isValidCidString: stringBooleanFunction(providerModule, "isValidCidString"),
    tryParseCid: stringFunction1(providerModule, "tryParseCid"),
    canonicalizeJson: stringFunction1(providerModule, "canonicalizeJson"),
    processOperation: bytesFunction1(providerModule, "processOperation"),
    processOperationJson: bytesFunction1(providerModule, "processOperationJson"),
  };
};

export const requireReallyMeCodecWasmProvider = (): ReallyMeCodecWasmProvider => {
  if (installedProvider === undefined || providerPoisoned) {
    throw new ReallyMeCodecError("provider-failure");
  }
  return installedProvider;
};
