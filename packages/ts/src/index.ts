// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

export { ReallyMeCodecError } from "./errors.js";
export type { ReallyMeCodecErrorCode } from "./errors.js";
export {
  base64Decode,
  base64Encode,
  base64urlDecode,
  base64urlDecodeBytes,
  base64urlEncode,
  bytesToLowerHex,
  lowerHexToBytes,
} from "./baseEncoding.js";
export {
  base58btcDecode,
  base58btcEncode,
  bindingTypeMatchesCodec,
  multibaseBase58btcEncode,
  multibaseBase64urlEncode,
  multibaseDecode,
  multicodecLookupPrefix,
  multicodecPrefixForName,
  multicodecStripPrefix,
  multicodecTable,
  multikeyEncode,
  multikeyParse,
  requireSupportedMulticodec,
  validateKeyBinding,
} from "./multiformat.js";
export type {
  ReallyMeKeyMaterialKind,
  ReallyMeMulticodecLookupResult,
  ReallyMeMulticodecMetadata,
  ReallyMeMulticodecTable,
  ReallyMeMulticodecTag,
  ReallyMeParsedMultikey,
} from "./multiformat.js";
export {
  dagCborCodecCode,
  dagCborCidMatches,
  dagCborComputeCid,
  dagCborDecode,
  dagCborEncode,
  dagCborMultihash,
  dagCborSha256ContentHash,
  dagCborVerifyCid,
  dagCborVerifyCidDetails,
  deterministicCborDecode,
  deterministicCborEncode,
  isValidCidString,
  ReallyMeDagCbor,
  ReallyMeDeterministicCbor,
  tryParseCid,
} from "./cbor.js";
export type {
  ReallyMeDeterministicCborInteger,
  ReallyMeDeterministicCborMapEntry,
  ReallyMeDeterministicCborMapKey,
  ReallyMeDeterministicCborValue,
  ReallyMeCborMapEntry,
  ReallyMeCborValue,
  ReallyMeDagCborCidVerification,
} from "./cbor.js";
export { canonicalizeJson, canonicalizeJsonText } from "./jcs.js";
export { decodePem, encodePem } from "./pem.js";
export { processOperation, processOperationJson } from "./operationContract.js";
export type {
  ReallyMePemDecodePolicy,
  ReallyMePemDocument,
  ReallyMePemEncodeOptions,
  ReallyMePemLabel,
  ReallyMePemLineEnding,
} from "./pem.js";
export {
  REALLYME_CODEC_WASM_EXPORTS,
  installReallyMeCodecWasmProvider,
} from "./wasmProvider.js";
export type { ReallyMeCodecWasmProvider } from "./wasmProvider.js";

export { ReallyMeCodec } from "./composeCodec.js";
