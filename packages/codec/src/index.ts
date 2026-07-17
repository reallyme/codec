// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

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
  multicodecLookupPrefixProto,
  multicodecLookupPrefixProtoResult,
  multicodecLookupPrefix,
  multicodecPrefixForNameProto,
  multicodecPrefixForNameProtoResult,
  multicodecPrefixForName,
  multicodecStripPrefix,
  multicodecTable,
  multicodecTableProto,
  multicodecTableProtoResult,
  multikeyEncode,
  multikeyParse,
  multikeyParseProto,
  multikeyParseProtoResult,
  requireSupportedMulticodec,
  validateKeyBinding,
} from "./multiformat.js";
export type {
  ReallyMeKeyMaterialKind,
  ReallyMeMulticodecMetadata,
  ReallyMeMulticodecTag,
  ReallyMeParsedMultikey,
} from "./multiformat.js";
export {
  dagCborCodecCode,
  dagCborComputeCid,
  dagCborDecode,
  dagCborEncode,
  dagCborMultihash,
  dagCborSha256ContentHash,
  dagCborVerifyCid,
  dagCborVerifyCidProto,
  dagCborVerifyCidProtoResult,
  isValidCidString,
  tryParseCid,
} from "./cbor.js";
export type {
  ReallyMeCborMapEntry,
  ReallyMeCborValue,
  ReallyMeDagCborCidVerification,
} from "./cbor.js";
export { canonicalizeJson, canonicalizeJsonText } from "./jcs.js";
export { decodePem, decodePemProto, decodePemProtoResult, encodePem } from "./pem.js";
export { processProto, processProtoJson } from "./protoProcess.js";
export type { ReallyMeCodecProtoResult, ReallyMeCodecProtoStatus } from "./readOutput.js";
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
  requireReallyMeCodecWasmProvider,
} from "./wasmProvider.js";
export type { ReallyMeCodecWasmProvider } from "./wasmProvider.js";

import {
  base64Decode,
  base64Encode,
  base64urlDecode,
  base64urlDecodeBytes,
  base64urlEncode,
  bytesToLowerHex,
  lowerHexToBytes,
} from "./baseEncoding.js";
import {
  dagCborCodecCode,
  dagCborComputeCid,
  dagCborDecode,
  dagCborEncode,
  dagCborMultihash,
  dagCborSha256ContentHash,
  dagCborVerifyCid,
  dagCborVerifyCidProto,
  dagCborVerifyCidProtoResult,
  isValidCidString,
  tryParseCid,
} from "./cbor.js";
import { canonicalizeJson, canonicalizeJsonText } from "./jcs.js";
import {
  base58btcDecode,
  base58btcEncode,
  bindingTypeMatchesCodec,
  multibaseBase58btcEncode,
  multibaseBase64urlEncode,
  multibaseDecode,
  multicodecLookupPrefixProto,
  multicodecLookupPrefixProtoResult,
  multicodecLookupPrefix,
  multicodecPrefixForNameProto,
  multicodecPrefixForNameProtoResult,
  multicodecPrefixForName,
  multicodecStripPrefix,
  multicodecTable,
  multicodecTableProto,
  multicodecTableProtoResult,
  multikeyEncode,
  multikeyParse,
  multikeyParseProto,
  multikeyParseProtoResult,
  requireSupportedMulticodec,
  validateKeyBinding,
} from "./multiformat.js";
import { decodePem, decodePemProto, decodePemProtoResult, encodePem } from "./pem.js";
import { processProto, processProtoJson } from "./protoProcess.js";

export const ReallyMeCodec = {
  base64Decode,
  base64Encode,
  base64urlDecode,
  base64urlDecodeBytes,
  base64urlEncode,
  bytesToLowerHex,
  lowerHexToBytes,
  base58btcDecode,
  base58btcEncode,
  bindingTypeMatchesCodec,
  multibaseBase58btcEncode,
  multibaseBase64urlEncode,
  multibaseDecode,
  multicodecLookupPrefixProto,
  multicodecLookupPrefixProtoResult,
  multicodecLookupPrefix,
  multicodecPrefixForNameProto,
  multicodecPrefixForNameProtoResult,
  multicodecPrefixForName,
  multicodecStripPrefix,
  multicodecTable,
  multicodecTableProto,
  multicodecTableProtoResult,
  multikeyEncode,
  multikeyParse,
  multikeyParseProto,
  multikeyParseProtoResult,
  requireSupportedMulticodec,
  validateKeyBinding,
  dagCborCodecCode,
  dagCborComputeCid,
  dagCborDecode,
  dagCborEncode,
  dagCborMultihash,
  dagCborSha256ContentHash,
  dagCborVerifyCid,
  dagCborVerifyCidProto,
  dagCborVerifyCidProtoResult,
  isValidCidString,
  tryParseCid,
  canonicalizeJson,
  canonicalizeJsonText,
  decodePem,
  decodePemProto,
  decodePemProtoResult,
  encodePem,
  processProto,
  processProtoJson,
} as const;
