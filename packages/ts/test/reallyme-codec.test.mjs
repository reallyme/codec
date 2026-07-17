// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import * as wasm from "../dist/wasm/reallyme_codec_wasm.js";
import {
  REALLYME_CODEC_WASM_EXPORTS,
  ReallyMeCodec,
  ReallyMeCodecError,
  base58btcDecode,
  base58btcEncode,
  base64Decode,
  base64Encode,
  base64urlDecode,
  base64urlDecodeBytes,
  base64urlEncode,
  bindingTypeMatchesCodec,
  bytesToLowerHex,
  canonicalizeJson,
  canonicalizeJsonText,
  dagCborCodecCode,
  dagCborComputeCid,
  dagCborDecode,
  dagCborEncode,
  dagCborMultihash,
  dagCborSha256ContentHash,
  dagCborVerifyCid,
  dagCborVerifyCidProto,
  dagCborVerifyCidProtoResult,
  decodePem,
  decodePemProto,
  decodePemProtoResult,
  encodePem,
  installReallyMeCodecWasmProvider,
  isValidCidString,
  lowerHexToBytes,
  multibaseBase58btcEncode,
  multibaseBase64urlEncode,
  multibaseDecode,
  multicodecLookupPrefix,
  multicodecLookupPrefixProto,
  multicodecLookupPrefixProtoResult,
  multicodecPrefixForName,
  multicodecPrefixForNameProto,
  multicodecPrefixForNameProtoResult,
  multicodecStripPrefix,
  multicodecTable,
  multicodecTableProto,
  multicodecTableProtoResult,
  multikeyEncode,
  multikeyParse,
  multikeyParseProto,
  multikeyParseProtoResult,
  processProto,
  processProtoJson,
  requireSupportedMulticodec,
  tryParseCid,
  validateKeyBinding,
} from "../dist/index.js";
import {
  CodecBackendErrorSchema,
  CodecBoundaryErrorSchema,
  CodecCanonicalizationErrorSchema,
  CodecDagCborVerifyCidResultSchema,
  CodecErrorSchema,
  CodecErrorReason,
  CodecKeyMaterialKind,
  CodecMulticodecLookupResultSchema,
  CodecMulticodecPrefixForNameRequestSchema,
  CodecMulticodecSpecSchema,
  CodecMulticodecTableResultSchema,
  CodecMultikeyParseResultSchema,
  CodecOperationRequestSchema,
  CodecPemDecodeResultSchema,
  CodecProtoResultEnvelopeSchema,
  CodecProtoResultStatus,
  CodecTag,
} from "../dist/proto.js";
import { protoPayloadOrThrow } from "../dist/protoProcess.js";
import {
  readIndependentBoundedBytesOutput,
  readStringProperty,
} from "../dist/readOutput.js";

const wasmBytes = readFileSync(
  new URL("../dist/wasm/reallyme_codec_wasm_bg.wasm", import.meta.url),
);
const codecVectorManifest = JSON.parse(
  readFileSync(
    new URL("../../../test-vectors/codec-vectors.json", import.meta.url),
    "utf8",
  ),
);
assert.equal(codecVectorManifest.schemaVersion, 2);
const codecVectors = codecVectorManifest.vectors;

wasm.initSync({ module: wasmBytes });
installReallyMeCodecWasmProvider(wasm);

const bytes = (...values) => Uint8Array.from(values);
const hex = (value) => Buffer.from(value).toString("hex");
const bytesFromHex = (value) => Uint8Array.from(Buffer.from(value, "hex"));
const utf8 = (value) => new TextEncoder().encode(value);

const assertCodecError = (operation, code) => {
  assert.throws(
    operation,
    (error) => error instanceof ReallyMeCodecError && error.code === code,
  );
};

const assertCodecRejected = (operation) => {
  assert.throws(operation, (error) => error instanceof ReallyMeCodecError);
};

const codecErrorResult = (error) => {
  const encoded = toBinary(CodecErrorSchema, error);
  return {
    status: "codec-error",
    bytes: encoded,
    isCodecError: true,
  };
};

test("WASM exports match the TypeScript provider contract", () => {
  for (const name of REALLYME_CODEC_WASM_EXPORTS) {
    assert.equal(typeof wasm[name], "function", name);
  }
});

test("WASM provider installation fails closed after the first install", () => {
  assertCodecError(() => installReallyMeCodecWasmProvider(wasm), "provider-failure");
});

test("protobuf envelope validation rejects shared and invalid provider storage", () => {
  const shared = new Uint8Array(8).fill(0x5a);
  assertCodecError(
    () => readIndependentBoundedBytesOutput(shared.subarray(4), shared.subarray(0, 4), 8),
    "provider-failure",
  );
  assert.deepEqual(shared, new Uint8Array(8).fill(0x5a));

  const oversized = new Uint8Array(9).fill(0xa5);
  assertCodecError(
    () => readIndependentBoundedBytesOutput(oversized, new Uint8Array([1]), 8),
    "provider-failure",
  );
  assert.deepEqual(oversized, new Uint8Array(9));
  assertCodecError(
    () => readIndependentBoundedBytesOutput(new Uint8Array(), new Uint8Array([1]), 8),
    "provider-failure",
  );
});

test("provider output validation rejects accessors without invoking them", () => {
  let getterInvoked = false;
  const output = Object.defineProperty({}, "value", {
    enumerable: true,
    get() {
      getterInvoked = true;
      return "provider-controlled";
    },
  });
  assertCodecError(() => readStringProperty(output, "value"), "provider-failure");
  assert.equal(getterInvoked, false);
});

test("ReallyMeCodec object exposes every codec family", () => {
  assert.deepEqual(
    Object.keys(ReallyMeCodec).sort(),
    [
      "base58btcDecode",
      "base58btcEncode",
      "base64Decode",
      "base64Encode",
      "base64urlDecode",
      "base64urlDecodeBytes",
      "base64urlEncode",
      "bindingTypeMatchesCodec",
      "bytesToLowerHex",
      "canonicalizeJson",
      "canonicalizeJsonText",
      "dagCborCodecCode",
      "dagCborComputeCid",
      "dagCborDecode",
      "dagCborEncode",
      "dagCborMultihash",
      "dagCborSha256ContentHash",
      "dagCborVerifyCid",
      "dagCborVerifyCidProto",
      "dagCborVerifyCidProtoResult",
      "decodePem",
      "decodePemProto",
      "decodePemProtoResult",
      "encodePem",
      "isValidCidString",
      "lowerHexToBytes",
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
      "processProto",
      "processProtoJson",
      "requireSupportedMulticodec",
      "tryParseCid",
      "validateKeyBinding",
    ].sort(),
  );
});

test("shared codec vector suite covers TypeScript public methods", () => {
  const baseInput = bytesFromHex(codecVectors.baseInputHex);
  assert.equal(base64Encode(baseInput), codecVectors.base64Padded);
  assert.deepEqual(base64Decode(codecVectors.base64Padded), baseInput);
  assert.equal(base64urlEncode(baseInput), codecVectors.base64urlUnpadded);
  assert.deepEqual(base64urlDecode(codecVectors.base64urlUnpadded), baseInput);
  assert.deepEqual(base64urlDecodeBytes(utf8(codecVectors.base64urlUnpadded)), baseInput);
  assert.equal(bytesToLowerHex(baseInput), codecVectors.lowerHex);
  assert.deepEqual(lowerHexToBytes(codecVectors.lowerHex), baseInput);
  assert.equal(base58btcEncode(baseInput), codecVectors.base58btcEncoded);
  assert.deepEqual(base58btcDecode(codecVectors.base58btcEncoded), baseInput);

  const publicKey = bytesFromHex(codecVectors.publicKeyHex);
  const prefixed = bytesFromHex(codecVectors.ed25519PrefixedPublicKeyHex);
  assert.equal(base58btcEncode(publicKey), codecVectors.publicKeyBase58btc);
  assert.equal(multibaseBase58btcEncode(publicKey), codecVectors.publicKeyMultibaseBase58btc);
  assert.equal(multibaseBase64urlEncode(publicKey), codecVectors.publicKeyMultibaseBase64url);
  assert.deepEqual(multibaseDecode(codecVectors.publicKeyMultibaseBase58btc), publicKey);
  assert.deepEqual(multibaseDecode(codecVectors.publicKeyMultibaseBase64url), publicKey);

  const metadata = multicodecPrefixForName(codecVectors.ed25519CodecName);
  const metadataProto = fromBinary(
    CodecMulticodecSpecSchema,
    multicodecPrefixForNameProto(codecVectors.ed25519CodecName),
  );
  const metadataProtoResult = multicodecPrefixForNameProtoResult(
    codecVectors.ed25519CodecName,
  );
  assert.equal(metadata.name, codecVectors.ed25519CodecName);
  assert.equal(metadata.alg, codecVectors.ed25519AlgorithmName);
  assert.equal(metadata.tag, codecVectors.ed25519Tag);
  assert.equal(metadata.keyMaterial, codecVectors.ed25519KeyMaterial);
  assert.equal(metadata.expectedKeyLength, codecVectors.ed25519ExpectedKeyLength);
  assert.equal(hex(metadata.prefix), codecVectors.ed25519PrefixHex);
  assert.equal(metadataProto.name, codecVectors.ed25519CodecName);
  assert.equal(metadataProto.algorithmName, codecVectors.ed25519AlgorithmName);
  assert.equal(metadataProtoResult.status, "result");
  assert.equal(
    fromBinary(CodecMulticodecSpecSchema, metadataProtoResult.bytes).name,
    codecVectors.ed25519CodecName,
  );

  const lookup = multicodecLookupPrefix(prefixed);
  const lookupProto = fromBinary(
    CodecMulticodecLookupResultSchema,
    multicodecLookupPrefixProto(prefixed),
  );
  assert.equal(lookup.name, codecVectors.ed25519CodecName);
  assert.equal(lookupProto.name, codecVectors.ed25519CodecName);
  assert.equal(multicodecLookupPrefixProtoResult(prefixed).status, "result");
  assert.deepEqual(multicodecStripPrefix(prefixed), publicKey);
  assert.ok(multicodecTable().some((entry) => entry.name === codecVectors.multicodecTableRequiredName));
  assert.ok(
    fromBinary(CodecMulticodecTableResultSchema, multicodecTableProto()).entries.some(
      (entry) => entry.name === codecVectors.multicodecTableRequiredName,
    ),
  );
  assert.equal(multicodecTableProtoResult().status, "result");

  assert.equal(
    multikeyEncode(codecVectors.ed25519CodecName, publicKey),
    codecVectors.ed25519Multikey,
  );
  const parsed = multikeyParse(codecVectors.ed25519Multikey);
  const parsedProto = fromBinary(
    CodecMultikeyParseResultSchema,
    multikeyParseProto(codecVectors.ed25519Multikey),
  );
  assert.equal(parsed.codecName, codecVectors.ed25519CodecName);
  assert.equal(parsed.algorithmName, codecVectors.ed25519AlgorithmName);
  assert.deepEqual(parsed.publicKey, publicKey);
  assert.equal(parsedProto.codecName, codecVectors.ed25519CodecName);
  assert.equal(multikeyParseProtoResult(codecVectors.ed25519Multikey).status, "result");
  assert.equal(
    bindingTypeMatchesCodec(
      codecVectors.multikeyBindingType,
      codecVectors.ed25519CodecName,
    ),
    true,
  );
  validateKeyBinding(
    codecVectors.multikeyBindingType,
    undefined,
    codecVectors.ed25519Multikey,
  );
  requireSupportedMulticodec(codecVectors.ed25519CodecName);
  assertCodecError(
    () => validateKeyBinding(
      codecVectors.mismatchedBindingType,
      codecVectors.mismatchedBindingAlgorithm,
      codecVectors.ed25519Multikey,
    ),
    "invalid-input",
  );

  const tagged = JSON.parse(codecVectors.dagCborTaggedJson);
  const encoded = dagCborEncode(tagged);
  assert.equal(hex(encoded), codecVectors.dagCborEncodedHex);
  assert.deepEqual(dagCborDecode(encoded), JSON.parse(codecVectors.dagCborCanonicalTaggedJson));
  assert.equal(dagCborComputeCid(encoded), codecVectors.dagCborCid);
  assert.equal(hex(dagCborSha256ContentHash(encoded)), codecVectors.dagCborSha256Hex);
  assert.equal(hex(dagCborMultihash(encoded)), codecVectors.dagCborMultihashHex);
  assert.equal(dagCborCodecCode(), codecVectors.dagCborCodecCode);
  assert.equal(isValidCidString(codecVectors.dagCborCid), true);
  assert.equal(tryParseCid(codecVectors.dagCborCid), codecVectors.dagCborCid);
  assert.equal(isValidCidString(codecVectors.invalidCid), false);
  assert.equal(tryParseCid(codecVectors.invalidCid), undefined);
  assert.equal(dagCborVerifyCid(codecVectors.dagCborCid, encoded).valid, true);
  assert.equal(
    fromBinary(
      CodecDagCborVerifyCidResultSchema,
      dagCborVerifyCidProto(codecVectors.dagCborCid, encoded),
    ).valid,
    true,
  );
  assert.equal(dagCborVerifyCidProtoResult(codecVectors.dagCborCid, encoded).status, "result");

  assert.equal(
    canonicalizeJson(JSON.parse(codecVectors.jcsObjectInputJson)),
    codecVectors.jcsObjectCanonicalJson,
  );
  assert.equal(
    canonicalizeJsonText(codecVectors.jcsObjectInputJson),
    codecVectors.jcsObjectCanonicalJson,
  );
  assert.equal(
    canonicalizeJsonText(codecVectors.jcsNumberInputJson),
    codecVectors.jcsNumberCanonicalJson,
  );

  const privateDer = bytesFromHex(codecVectors.pemPrivateDerHex);
  assert.deepEqual(
    encodePem(codecVectors.pemPrivateLabel, privateDer),
    utf8(codecVectors.pemPrivatePem),
  );
  const decodedPem = decodePem(utf8(codecVectors.pemPrivatePem));
  assert.equal(decodedPem.label, codecVectors.pemPrivateLabel);
  assert.deepEqual(decodedPem.der, privateDer);
  assert.deepEqual(
    encodePem(
      codecVectors.pemPublicLabel,
      utf8(codecVectors.pemWrappedDerText),
      { lineWidth: 4 },
    ),
    utf8(codecVectors.pemWrappedPem),
  );
  const decodedPemProto = fromBinary(
    CodecPemDecodeResultSchema,
    decodePemProto(utf8(codecVectors.pemPrivatePem)),
  );
  assert.equal(decodedPemProto.label, codecVectors.pemPrivateLabel);
  assert.equal(decodePemProtoResult(utf8(codecVectors.pemPrivatePem)).status, "result");

  const envelope = fromBinary(
    CodecProtoResultEnvelopeSchema,
    processProto(bytesFromHex(codecVectors.protoMulticodecTableRequestHex)),
  );
  assert.equal(envelope.status, CodecProtoResultStatus.RESULT);
  assert.ok(
    fromBinary(CodecMulticodecTableResultSchema, envelope.payload).entries.some(
      (entry) => entry.name === codecVectors.multicodecTableRequiredName,
    ),
  );
  const jsonEnvelope = fromBinary(
    CodecProtoResultEnvelopeSchema,
    processProtoJson(utf8(codecVectors.protoMulticodecTableRequestJson)),
  );
  assert.equal(jsonEnvelope.status, CodecProtoResultStatus.RESULT);
});

test("shared codec vector suite rejects non-canonical inputs in TypeScript", () => {
  assertCodecRejected(() => base64Decode(codecVectors.base64MissingPadding));
  assertCodecRejected(() => base64Decode(codecVectors.base64NonCanonicalTrailingBits));
  assertCodecRejected(() => base64urlDecode(codecVectors.base64urlPadded));
  assertCodecRejected(() => base64urlDecode(codecVectors.base64urlNonCanonicalTrailingBits));
  assertCodecRejected(() => multibaseDecode(codecVectors.unsupportedMultibase));
  assertCodecRejected(() => multikeyParse(codecVectors.nonCanonicalBase64urlMultikey));
  assertCodecRejected(() =>
    dagCborDecode(bytesFromHex(codecVectors.dagCborNonCanonicalIntegerHex))
  );
  assertCodecRejected(() =>
    dagCborDecode(bytesFromHex(codecVectors.dagCborDuplicateKeyHex))
  );
  assertCodecRejected(() =>
    dagCborDecode(bytesFromHex(codecVectors.dagCborOutOfOrderKeyHex))
  );
  assertCodecRejected(() => canonicalizeJsonText(codecVectors.jcsDuplicateMemberJson));
  assertCodecRejected(() => canonicalizeJsonText(codecVectors.jcsNonInteroperableIntegerJson));
  assertCodecRejected(() => canonicalizeJsonText(codecVectors.jcsLoneSurrogateJson));
});

test("base64 and base64url match the Rust codec policy", () => {
  const input = bytes(0, 1, 2, 251, 255);
  assert.equal(base64Encode(input), "AAEC+/8=");
  assert.deepEqual(base64Decode("AAEC+/8="), input);
  assert.deepEqual(base64Decode(""), bytes());
  assert.equal(base64urlEncode(input), "AAEC-_8");
  assert.deepEqual(base64urlDecode("AAEC-_8"), input);
  assert.deepEqual(base64urlDecode(""), bytes());
  assert.deepEqual(base64urlDecodeBytes(bytes(65, 65, 69, 67, 45, 95, 56)), input);
  assertCodecError(() => base64Decode("Zh=="), "invalid-input");
  assertCodecError(() => base64urlDecode("AAEC-_8="), "invalid-input");
});

test("base58btc handles empty decode and rejects oversized encode inputs", () => {
  assert.deepEqual(base58btcDecode(""), bytes());
  const oversized = new Uint8Array(8 * 1024 + 1);
  assertCodecError(() => base58btcEncode(oversized), "invalid-input");
  assertCodecError(() => multibaseBase58btcEncode(oversized), "invalid-input");
});

test("lowercase hex is canonical and rejects uppercase input", () => {
  const input = bytes(0, 10, 255);
  assert.equal(bytesToLowerHex(input), "000aff");
  assert.deepEqual(lowerHexToBytes("000aff"), input);
  assert.deepEqual(lowerHexToBytes(""), bytes());
  assertCodecError(() => lowerHexToBytes("000AFF"), "non-canonical");
  assertCodecError(() => lowerHexToBytes("zz"), "invalid-input");
  assertCodecError(() => lowerHexToBytes("abc"), "invalid-input");
});

test("multibase, multicodec, and multikey round-trip through Rust WASM", () => {
  const publicKey = new Uint8Array(32);
  publicKey[31] = 7;

  const base58 = multibaseBase58btcEncode(publicKey);
  const base64url = multibaseBase64urlEncode(publicKey);
  assert.equal(base58[0], "z");
  assert.equal(base64url[0], "u");
  assert.deepEqual(multibaseDecode(base58), publicKey);
  assert.deepEqual(multibaseDecode(base64url), publicKey);
  assert.equal(multibaseBase64urlEncode(bytes()), "u");
  assert.deepEqual(multibaseDecode("u"), bytes());

  const metadata = multicodecPrefixForName("ed25519-pub");
  const metadataProto = fromBinary(
    CodecMulticodecSpecSchema,
    multicodecPrefixForNameProto("ed25519-pub"),
  );
  const metadataProtoResult = multicodecPrefixForNameProtoResult("ed25519-pub");
  assert.equal(metadataProtoResult.status, "result");
  assert.equal(metadataProtoResult.isCodecError, false);
  assert.equal(
    fromBinary(CodecMulticodecSpecSchema, metadataProtoResult.bytes).name,
    "ed25519-pub",
  );
  assert.equal(metadata.name, "ed25519-pub");
  assert.equal(metadataProto.name, "ed25519-pub");
  assert.equal(metadata.alg, "Ed25519");
  assert.equal(metadataProto.algorithmName, "Ed25519");
  assert.equal(metadata.tag, "key");
  assert.equal(metadataProto.tag, CodecTag.KEY);
  assert.equal(metadata.keyMaterial, "public-key");
  assert.equal(metadataProto.keyMaterialKind, CodecKeyMaterialKind.PUBLIC_KEY);
  assert.equal(metadata.expectedKeyLength, 32);
  assert.equal(metadataProto.fixedLength, 32);
  assert.deepEqual(metadataProto.prefix, metadata.prefix);

  const prefixed = new Uint8Array(metadata.prefix.length + publicKey.length);
  prefixed.set(metadata.prefix);
  prefixed.set(publicKey, metadata.prefix.length);
  const lookup = multicodecLookupPrefix(prefixed);
  const lookupProto = fromBinary(
    CodecMulticodecLookupResultSchema,
    multicodecLookupPrefixProto(prefixed),
  );
  assert.equal(multicodecLookupPrefixProtoResult(prefixed).status, "result");
  assert.equal(lookup.name, "ed25519-pub");
  assert.equal(lookupProto.name, "ed25519-pub");
  assert.equal(lookupProto.prefixLength, metadata.prefix.length);
  assert.deepEqual(multicodecStripPrefix(prefixed), publicKey);
  assert.ok(multicodecTable().some((entry) => entry.name === "mlkem-1024-pub"));
  assert.ok(
    fromBinary(CodecMulticodecTableResultSchema, multicodecTableProto()).entries.some(
      (entry) => entry.name === "mlkem-1024-pub",
    ),
  );
  assert.equal(multicodecTableProtoResult().status, "result");

  const multikey = multikeyEncode("ed25519-pub", publicKey);
  const parsed = multikeyParse(multikey);
  const parsedProto = fromBinary(CodecMultikeyParseResultSchema, multikeyParseProto(multikey));
  const parsedProtoResult = multikeyParseProtoResult(multikey);
  assert.equal(parsedProtoResult.status, "result");
  assert.equal(
    fromBinary(CodecMultikeyParseResultSchema, parsedProtoResult.bytes).codecName,
    "ed25519-pub",
  );
  assert.equal(parsed.codecName, "ed25519-pub");
  assert.equal(parsedProto.codecName, "ed25519-pub");
  assert.equal(parsed.algorithmName, "Ed25519");
  assert.equal(parsedProto.algorithmName, "Ed25519");
  assert.deepEqual(parsed.publicKey, publicKey);
  assert.deepEqual(parsedProto.publicKey, publicKey);
  assert.equal(parsed.expectedPublicKeyLength, 32);
  assert.equal(parsedProto.expectedPublicKeyLength, 32);

  const nonCanonicalMultikey = `u${base64urlEncode(prefixed)}`;
  assertCodecError(() => multikeyParse(nonCanonicalMultikey), "invalid-input");
  assertCodecError(() => multikeyParseProto(nonCanonicalMultikey), "invalid-input");
  const nonCanonicalMultikeyErrorResult = multikeyParseProtoResult(nonCanonicalMultikey);
  const nonCanonicalMultikeyError = fromBinary(
    CodecErrorSchema,
    nonCanonicalMultikeyErrorResult.bytes,
  );
  assert.equal(nonCanonicalMultikeyErrorResult.status, "codec-error");
  assert.equal(nonCanonicalMultikeyError.error.case, "multiformat");
  assert.equal(
    nonCanonicalMultikeyError.error.value.reason,
    CodecErrorReason.MULTIFORMAT_INVALID_MULTIKEY,
  );

  assert.equal(bindingTypeMatchesCodec("Multikey", parsed.codecName), true);
  validateKeyBinding("Multikey", undefined, multikey);
  assertCodecError(() => validateKeyBinding("P256Key2024", "P-256", multikey), "invalid-input");
  assertCodecError(() => requireSupportedMulticodec("not-a-codec"), "unsupported-codec");
  assertCodecError(() => multikeyEncode("not-a-codec", publicKey), "unsupported-codec");

  assertCodecError(() => multikeyParseProto("not-a-key"), "invalid-input");
  const multikeyErrorResult = multikeyParseProtoResult("not-a-key");
  const multikeyError = fromBinary(CodecErrorSchema, multikeyErrorResult.bytes);
  assert.equal(multikeyErrorResult.status, "codec-error");
  assert.equal(multikeyErrorResult.isCodecError, true);
  assert.equal(multikeyError.error.case, "multiformat");
  assert.equal(
    multikeyError.error.value.reason,
    CodecErrorReason.MULTIFORMAT_INVALID_MULTIKEY,
  );
  const unknownPrefixMultikey = multibaseBase58btcEncode(bytes(0, 0, 7));
  assertCodecError(() => multikeyParseProto(unknownPrefixMultikey), "invalid-input");
  const unknownPrefixMultikeyErrorResult = multikeyParseProtoResult(unknownPrefixMultikey);
  const unknownPrefixMultikeyError = fromBinary(
    CodecErrorSchema,
    unknownPrefixMultikeyErrorResult.bytes,
  );
  assert.equal(unknownPrefixMultikeyErrorResult.status, "codec-error");
  assert.equal(unknownPrefixMultikeyError.error.case, "multiformat");
  assert.equal(
    unknownPrefixMultikeyError.error.value.reason,
    CodecErrorReason.MULTIFORMAT_UNKNOWN_MULTICODEC,
  );
  assertCodecError(() => multicodecPrefixForNameProto("not-a-codec"), "invalid-input");
  const multicodecErrorResult = multicodecPrefixForNameProtoResult("not-a-codec");
  const multicodecError = fromBinary(CodecErrorSchema, multicodecErrorResult.bytes);
  assert.equal(multicodecErrorResult.status, "codec-error");
  assert.equal(
    fromBinary(CodecErrorSchema, multicodecErrorResult.bytes).error.case,
    "multiformat",
  );
  assert.equal(multicodecError.error.case, "multiformat");
  assert.equal(
    multicodecError.error.value.reason,
    CodecErrorReason.MULTIFORMAT_UNKNOWN_MULTICODEC,
  );
});

test("DAG-CBOR encode/decode and CID helpers use the Rust codec", () => {
  const value = {
    type: "map",
    value: [
      { key: "b", value: { type: "int", value: 2 } },
      { key: "a", value: { type: "string", value: "one" } },
      { key: "bytes", value: { type: "bytes", value: "AAEC" } },
    ],
  };
  const encoded = dagCborEncode(value);
  assert.deepEqual(dagCborDecode(encoded), {
    type: "map",
    value: [
      { key: "a", value: { type: "string", value: "one" } },
      { key: "b", value: { type: "int", value: 2 } },
      { key: "bytes", value: { type: "bytes", value: "AAEC" } },
    ],
  });

  const cid = dagCborComputeCid(encoded);
  assert.equal(isValidCidString(cid), true);
  assert.equal(tryParseCid(cid), cid);
  assert.equal(isValidCidString(""), false);
  assert.equal(tryParseCid(""), undefined);
  assert.equal(dagCborCodecCode(), 0x71);
  assert.equal(dagCborVerifyCid(cid, encoded).valid, true);
  assert.equal(
    fromBinary(CodecDagCborVerifyCidResultSchema, dagCborVerifyCidProto(cid, encoded)).valid,
    true,
  );
  assert.equal(dagCborVerifyCidProtoResult(cid, encoded).status, "result");
  const invalidUpperPayloadCid = `${cid[0]}${cid.slice(1).toUpperCase()}`;
  const invalidVerification = dagCborVerifyCid(invalidUpperPayloadCid, encoded);
  assert.equal(invalidVerification.valid, false);
  assert.equal(invalidVerification.expectedCid, cid);
  assert.equal(invalidVerification.actualCid, "");
  const invalidProtoVerification = fromBinary(
    CodecDagCborVerifyCidResultSchema,
    dagCborVerifyCidProto(invalidUpperPayloadCid, encoded),
  );
  assert.equal(invalidProtoVerification.valid, false);
  assert.equal(invalidProtoVerification.expectedCid, cid);
  assert.equal(invalidProtoVerification.actualCid, "");
  const emptyCidVerification = dagCborVerifyCid("", encoded);
  assert.equal(emptyCidVerification.valid, false);
  assert.equal(emptyCidVerification.expectedCid, cid);
  assert.equal(emptyCidVerification.actualCid, "");
  assert.equal(
    fromBinary(CodecDagCborVerifyCidResultSchema, dagCborVerifyCidProto("", encoded)).valid,
    false,
  );
  assert.equal(dagCborVerifyCidProtoResult("", encoded).status, "result");

  const largeInteger = { type: "int", value: 9_007_199_254_740_993n };
  assert.deepEqual(dagCborDecode(dagCborEncode(largeInteger)), largeInteger);
  assert.equal(hex(dagCborSha256ContentHash(encoded)).length, 64);
  assert.ok(dagCborMultihash(encoded).length > 32);
  assertCodecError(() => dagCborDecode(bytes(0xa2, 0x61, 0x62, 0x01, 0x61, 0x61, 0x02)), "non-canonical");

  const oversizedCbor = new Uint8Array(1024 * 1024 + 1);
  assertCodecError(() => dagCborDecode(oversizedCbor), "invalid-input");
  assertCodecError(() => dagCborComputeCid(oversizedCbor), "invalid-input");
  assertCodecError(() => dagCborVerifyCid(cid, oversizedCbor), "invalid-input");
  assertCodecError(() => dagCborVerifyCidProto(cid, oversizedCbor), "invalid-input");
  assert.equal(dagCborVerifyCidProtoResult(cid, oversizedCbor).status, "codec-error");
  assertCodecError(() => dagCborSha256ContentHash(oversizedCbor), "invalid-input");
  assertCodecError(() => dagCborMultihash(oversizedCbor), "invalid-input");
});

test("DAG-CBOR encode rejects hostile object graphs with typed errors", () => {
  assertCodecError(
    () =>
      dagCborEncode({
        type: "map",
        value: [
          { key: "a", value: { type: "int", value: 1 } },
          { key: "a", value: { type: "int", value: 2 } },
        ],
      }),
    "invalid-input",
  );

  const cycle = { type: "array" };
  cycle.value = [cycle];
  assertCodecError(() => dagCborEncode(cycle), "invalid-input");

  let tooDeep = { type: "null" };
  for (let index = 0; index < 129; index += 1) {
    tooDeep = { type: "array", value: [tooDeep] };
  }
  assertCodecError(() => dagCborEncode(tooDeep), "invalid-input");

  const tooWide = {
    type: "array",
    value: Array.from({ length: 65_537 }, () => ({ type: "null" })),
  };
  assertCodecError(() => dagCborEncode(tooWide), "invalid-input");

  let getterInvoked = false;
  const accessor = Object.defineProperty({}, "type", {
    enumerable: true,
    get() {
      getterInvoked = true;
      return "null";
    },
  });
  assertCodecError(() => dagCborEncode(accessor), "invalid-input");
  assert.equal(getterInvoked, false);
});

test("JCS canonicalization is stable for supported JSON values", () => {
  assert.equal(canonicalizeJson({ b: 2, a: 1 }), "{\"a\":1,\"b\":2}");
  assert.equal(canonicalizeJsonText("{\"b\":2,\"a\":1}"), "{\"a\":1,\"b\":2}");
  assertCodecError(() => canonicalizeJsonText("{"), "invalid-input");
  assertCodecError(() => canonicalizeJsonText("{\"a\":1,\"a\":2}"), "invalid-input");
  assertCodecError(() => canonicalizeJsonText("18446744073709551615"), "invalid-input");
});

test("JCS traversal guards do not reject byte-bounded documents accepted by Rust", () => {
  const values = Array.from({ length: 70_000 }, () => 0);
  const canonical = canonicalizeJson(values);
  assert.equal(canonical.length, 140_001);
});

test("array metadata is snapshotted once without invoking proxy getters", () => {
  let lengthGets = 0;
  let lengthDescriptors = 0;
  const proxied = new Proxy([0, 1, 2], {
    get(target, property, receiver) {
      if (property === "length") {
        lengthGets += 1;
      }
      return Reflect.get(target, property, receiver);
    },
    getOwnPropertyDescriptor(target, property) {
      if (property === "length") {
        lengthDescriptors += 1;
      }
      return Reflect.getOwnPropertyDescriptor(target, property);
    },
  });
  assert.equal(canonicalizeJson(proxied), "[0,1,2]");
  assert.equal(lengthGets, 0);
  assert.equal(lengthDescriptors, 1);
});

test("throwing protobuf APIs preserve caller-versus-provider attribution", () => {
  const backendResult = codecErrorResult(create(CodecErrorSchema, {
    error: {
      case: "backend",
      value: create(CodecBackendErrorSchema, {
        reason: CodecErrorReason.BACKEND_INTERNAL,
      }),
    },
  }));
  const backendBytes = backendResult.bytes;
  assertCodecError(() => protoPayloadOrThrow(backendResult), "provider-failure");
  assert.deepEqual(backendBytes, new Uint8Array(backendBytes.length));

  const internalResult = codecErrorResult(create(CodecErrorSchema, {
    error: {
      case: "canonicalization",
      value: create(CodecCanonicalizationErrorSchema, {
        reason: CodecErrorReason.CANONICAL_INTERNAL,
      }),
    },
  }));
  assertCodecError(() => protoPayloadOrThrow(internalResult), "provider-failure");

  const malformedBoundaryResult = codecErrorResult(create(CodecErrorSchema, {
    error: {
      case: "boundary",
      value: create(CodecBoundaryErrorSchema, {
        reason: CodecErrorReason.BOUNDARY_MALFORMED_PROTOBUF,
      }),
    },
  }));
  assertCodecError(
    () => protoPayloadOrThrow(malformedBoundaryResult),
    "provider-failure",
  );

  const resourceBoundaryResult = codecErrorResult(create(CodecErrorSchema, {
    error: {
      case: "boundary",
      value: create(CodecBoundaryErrorSchema, {
        reason: CodecErrorReason.BOUNDARY_RESOURCE_LIMIT_EXCEEDED,
      }),
    },
  }));
  assertCodecError(() => protoPayloadOrThrow(resourceBoundaryResult), "invalid-input");

  const mismatchedResult = codecErrorResult(create(CodecErrorSchema, {
    error: {
      case: "backend",
      value: create(CodecBackendErrorSchema, {
        reason: CodecErrorReason.MULTIFORMAT_INVALID_MULTIKEY,
      }),
    },
  }));
  assertCodecError(() => protoPayloadOrThrow(mismatchedResult), "provider-failure");

  const unknownReasonResult = codecErrorResult(create(CodecErrorSchema, {
    error: {
      case: "canonicalization",
      value: create(CodecCanonicalizationErrorSchema, { reason: 450 }),
    },
  }));
  assertCodecError(() => protoPayloadOrThrow(unknownReasonResult), "provider-failure");

  const malformedResult = {
    status: "codec-error",
    bytes: bytes(0xff),
    isCodecError: true,
  };
  assertCodecError(() => protoPayloadOrThrow(malformedResult), "provider-failure");
});

test("JCS canonicalization rejects non-JSON JavaScript values with typed errors", () => {
  assertCodecError(() => canonicalizeJson(1n), "invalid-input");
  assertCodecError(() => canonicalizeJson(Number.NaN), "invalid-input");
  assertCodecError(() => canonicalizeJson(Number.POSITIVE_INFINITY), "invalid-input");
  assertCodecError(() => canonicalizeJson(() => "hidden"), "invalid-input");
  assertCodecError(() => canonicalizeJson({ omitted: undefined }), "invalid-input");
  assertCodecError(() => canonicalizeJson(new Date("2026-01-01T00:00:00Z")), "invalid-input");

  const cycle = { value: null };
  cycle.value = cycle;
  assertCodecError(() => canonicalizeJson(cycle), "invalid-input");

  let getterInvoked = false;
  const accessor = Object.defineProperty({}, "secret", {
    enumerable: true,
    get() {
      getterInvoked = true;
      return "secret";
    },
  });
  assertCodecError(() => canonicalizeJson(accessor), "invalid-input");
  assert.equal(getterInvoked, false);

  let arrayGetterInvoked = false;
  const accessorArray = Object.defineProperty([], "0", {
    enumerable: true,
    get() {
      arrayGetterInvoked = true;
      return "secret";
    },
  });
  Object.defineProperty(accessorArray, "length", { value: 1 });
  assertCodecError(() => canonicalizeJson(accessorArray), "invalid-input");
  assert.equal(arrayGetterInvoked, false);

  const oversizedText = "a".repeat(1_048_577);
  assertCodecError(() => canonicalizeJson({ value: oversizedText }), "invalid-input");
  assertCodecError(() => canonicalizeJsonText(oversizedText), "invalid-input");
  assertCodecError(
    () => base64urlDecodeBytes(new Uint8Array(1_048_577)),
    "invalid-input",
  );

  assertCodecError(
    () => multicodecPrefixForNameProto(oversizedText),
    "invalid-input",
  );
  const oversizedProtoResult = multicodecPrefixForNameProtoResult(oversizedText);
  assert.equal(oversizedProtoResult.status, "codec-error");
  const oversizedProtoError = fromBinary(
    CodecErrorSchema,
    oversizedProtoResult.bytes,
  );
  assert.equal(
    oversizedProtoError.error.value.reason,
    CodecErrorReason.BOUNDARY_RESOURCE_LIMIT_EXCEEDED,
  );
});

test("PEM armor uses Rust policy for labels, wrapping, and decoding", () => {
  const der = Uint8Array.from(Buffer.from("not real der"));
  const pem = encodePem("PUBLIC KEY", der, { lineWidth: 4 });
  assert.deepEqual(
    pem,
    utf8("-----BEGIN PUBLIC KEY-----\nbm90\nIHJl\nYWwg\nZGVy\n-----END PUBLIC KEY-----\n"),
  );
  const decoded = decodePem(pem, { allowedLabels: ["PUBLIC KEY"] });
  const decodedProto = fromBinary(
    CodecPemDecodeResultSchema,
    decodePemProto(pem, { allowedLabels: ["PUBLIC KEY"] }),
  );
  const decodedProtoResult = decodePemProtoResult(pem, { allowedLabels: ["PUBLIC KEY"] });
  assert.equal(decodedProtoResult.status, "result");
  assert.deepEqual(fromBinary(CodecPemDecodeResultSchema, decodedProtoResult.bytes).der, der);
  assert.equal(decoded.label, "PUBLIC KEY");
  assert.equal(decodedProto.label, "PUBLIC KEY");
  assert.deepEqual(decoded.der, der);
  assert.deepEqual(decodedProto.der, der);
  assertCodecError(() => decodePem(pem, { allowedLabels: ["PRIVATE KEY"] }), "invalid-input");
  assertCodecError(() => encodePem("PUBLIC KEY", bytes()), "invalid-input");
  assertCodecError(() => encodePem("PUBLIC KEY", der, { maxDerLen: 4 }), "invalid-input");
  assertCodecError(() => encodePem("PUBLIC KEY", der, { lineWidth: 77 }), "invalid-input");

  let policyGetterInvoked = false;
  const accessorPolicy = Object.defineProperty({}, "allowedLabels", {
    enumerable: true,
    get() {
      policyGetterInvoked = true;
      return ["PUBLIC KEY"];
    },
  });
  assertCodecError(() => decodePem(pem, accessorPolicy), "invalid-input");
  assertCodecError(() => decodePemProtoResult(pem, accessorPolicy), "invalid-input");
  assert.equal(policyGetterInvoked, false);

  let optionsGetterInvoked = false;
  const accessorOptions = Object.defineProperty({}, "lineWidth", {
    enumerable: true,
    get() {
      optionsGetterInvoked = true;
      return 64;
    },
  });
  assertCodecError(() => encodePem("PUBLIC KEY", der, accessorOptions), "invalid-input");
  assert.equal(optionsGetterInvoked, false);
  assertCodecError(
    () => decodePem(pem, { allowedLabels: ["PUBLIC KEY"], unexpected: true }),
    "invalid-input",
  );
  assertCodecError(
    () => encodePem("PUBLIC KEY", der, { [Symbol("unexpected")]: 64 }),
    "invalid-input",
  );

  assertCodecError(
    () => decodePemProto(pem, { allowedLabels: ["PRIVATE KEY"] }),
    "invalid-input",
  );
  const pemErrorResult = decodePemProtoResult(pem, { allowedLabels: ["PRIVATE KEY"] });
  const pemError = fromBinary(CodecErrorSchema, pemErrorResult.bytes);
  assert.equal(pemErrorResult.status, "codec-error");
  assert.equal(fromBinary(CodecErrorSchema, pemErrorResult.bytes).error.case, "pem");
  assert.equal(pemError.error.case, "pem");
  assert.equal(pemError.error.value.reason, CodecErrorReason.PEM_UNSUPPORTED_LABEL);
});

test("codec proto exports carry codec-owned error reasons", () => {
  assert.equal(CodecErrorReason.BASE_INVALID_HEX, 120);
});

test("single protobuf and generated ProtoJSON entrypoints return equivalent envelopes", () => {
  const request = create(CodecOperationRequestSchema, {
    operation: {
      case: "multicodecPrefixForName",
      value: create(CodecMulticodecPrefixForNameRequestSchema, {
        name: "ed25519-pub",
      }),
    },
  });
  const binaryEnvelope = fromBinary(
    CodecProtoResultEnvelopeSchema,
    processProto(toBinary(CodecOperationRequestSchema, request)),
  );
  const jsonEnvelope = fromBinary(
    CodecProtoResultEnvelopeSchema,
    processProtoJson(
      new TextEncoder().encode(
        '{"multicodecPrefixForName":{"name":"ed25519-pub"}}',
      ),
    ),
  );

  assert.equal(binaryEnvelope.status, CodecProtoResultStatus.RESULT);
  assert.equal(jsonEnvelope.status, CodecProtoResultStatus.RESULT);
  assert.deepEqual(jsonEnvelope.payload, binaryEnvelope.payload);
});

test("malformed protobuf and ProtoJSON fail inside typed boundary envelopes", () => {
  const malformedProtobuf = fromBinary(
    CodecProtoResultEnvelopeSchema,
    processProto(bytes(0xff)),
  );
  assert.equal(malformedProtobuf.status, CodecProtoResultStatus.CODEC_ERROR);
  const protobufError = fromBinary(CodecErrorSchema, malformedProtobuf.payload);
  assert.equal(protobufError.error.case, "boundary");
  assert.equal(
    protobufError.error.value.reason,
    CodecErrorReason.BOUNDARY_MALFORMED_PROTOBUF,
  );

  const malformedJson = fromBinary(
    CodecProtoResultEnvelopeSchema,
    processProtoJson(new TextEncoder().encode('{"unknownOperation":{}}')),
  );
  assert.equal(malformedJson.status, CodecProtoResultStatus.CODEC_ERROR);
  const jsonError = fromBinary(CodecErrorSchema, malformedJson.payload);
  assert.equal(jsonError.error.case, "boundary");
  assert.equal(
    jsonError.error.value.reason,
    CodecErrorReason.BOUNDARY_MALFORMED_JSON,
  );
});

test("WASM boundaries reject oversized non-proto inputs before codec allocation", () => {
  const oversizedRaw = new Uint8Array(1024 * 1024 + 1);
  assertCodecError(() => base64Encode(oversizedRaw), "invalid-input");
  assertCodecError(() => decodePem(oversizedRaw), "invalid-input");
  assertCodecError(
    () => canonicalizeJsonText(" ".repeat(1024 * 1024 + 1)),
    "invalid-input",
  );

});

test("WASM proto boundaries preserve typed resource-limit envelopes", () => {
  const cases = [
    processProto(new Uint8Array(1024 * 1024 + 1)),
    processProtoJson(new Uint8Array(1_572_864 + 1)),
  ];

  for (const envelopeBytes of cases) {
    const envelope = fromBinary(CodecProtoResultEnvelopeSchema, envelopeBytes);
    assert.equal(envelope.status, CodecProtoResultStatus.CODEC_ERROR);
    const error = fromBinary(CodecErrorSchema, envelope.payload);
    assert.equal(error.error.case, "boundary");
    assert.equal(
      error.error.value.reason,
      CodecErrorReason.BOUNDARY_RESOURCE_LIMIT_EXCEEDED,
    );
    envelopeBytes.fill(0);
  }
});
