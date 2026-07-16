// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { fromBinary } from "@bufbuild/protobuf";
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
  requireSupportedMulticodec,
  tryParseCid,
  validateKeyBinding,
} from "../dist/index.js";
import {
  CodecDagCborVerifyCidResultSchema,
  CodecErrorSchema,
  CodecErrorReason,
  CodecKeyMaterialKind,
  CodecMulticodecLookupResultSchema,
  CodecMulticodecSpecSchema,
  CodecMulticodecTableResultSchema,
  CodecMultikeyParseResultSchema,
  CodecPemDecodeResultSchema,
  CodecTag,
} from "../dist/proto.js";

const wasmBytes = readFileSync(
  new URL("../dist/wasm/reallyme_codec_wasm_bg.wasm", import.meta.url),
);

wasm.initSync({ module: wasmBytes });
installReallyMeCodecWasmProvider(wasm);

const bytes = (...values) => Uint8Array.from(values);
const hex = (value) => Buffer.from(value).toString("hex");

const assertCodecError = (operation, code) => {
  assert.throws(
    operation,
    (error) => error instanceof ReallyMeCodecError && error.code === code,
  );
};

test("WASM exports match the TypeScript provider contract", () => {
  for (const name of REALLYME_CODEC_WASM_EXPORTS) {
    assert.equal(typeof wasm[name], "function", name);
  }
});

test("WASM provider installation fails closed after the first install", () => {
  assertCodecError(() => installReallyMeCodecWasmProvider(wasm), "provider-failure");
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
      "requireSupportedMulticodec",
      "tryParseCid",
      "validateKeyBinding",
    ].sort(),
  );
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
  assertCodecError(() => dagCborDecode(oversizedCbor), "non-canonical");
  assertCodecError(() => dagCborComputeCid(oversizedCbor), "invalid-input");
  assertCodecError(() => dagCborVerifyCid(cid, oversizedCbor), "invalid-input");
  assertCodecError(() => dagCborVerifyCidProto(cid, oversizedCbor), "invalid-input");
  assert.equal(dagCborVerifyCidProtoResult(cid, oversizedCbor).status, "codec-error");
  assertCodecError(() => dagCborSha256ContentHash(oversizedCbor), "invalid-input");
  assertCodecError(() => dagCborMultihash(oversizedCbor), "invalid-input");
});

test("DAG-CBOR encode rejects hostile object graphs with typed errors", () => {
  const cycle = { type: "array" };
  cycle.value = [cycle];
  assertCodecError(() => dagCborEncode(cycle), "invalid-input");

  let tooDeep = { type: "null" };
  for (let index = 0; index < 129; index += 1) {
    tooDeep = { type: "array", value: [tooDeep] };
  }
  assertCodecError(() => dagCborEncode(tooDeep), "invalid-input");
});

test("JCS canonicalization is stable for supported JSON values", () => {
  assert.equal(canonicalizeJson({ b: 2, a: 1 }), "{\"a\":1,\"b\":2}");
  assert.equal(canonicalizeJsonText("{\"b\":2,\"a\":1}"), "{\"a\":1,\"b\":2}");
  assertCodecError(() => canonicalizeJsonText("{"), "invalid-input");
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
});

test("PEM armor uses Rust policy for labels, wrapping, and decoding", () => {
  const der = Uint8Array.from(Buffer.from("not real der"));
  const pem = encodePem("PUBLIC KEY", der, { lineWidth: 4 });
  assert.equal(
    pem,
    "-----BEGIN PUBLIC KEY-----\nbm90\nIHJl\nYWwg\nZGVy\n-----END PUBLIC KEY-----\n",
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
