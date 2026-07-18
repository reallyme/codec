// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import fc from "fast-check";
import * as wasm from "../dist/wasm/reallyme_codec_wasm.js";
import {
  REALLYME_CODEC_WASM_EXPORTS,
  ReallyMeCodec,
  ReallyMeCodecError,
  ReallyMeDagCbor,
  ReallyMeDeterministicCbor,
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
  deterministicCborDecode,
  deterministicCborEncode,
  decodePem,
  encodePem,
  installReallyMeCodecWasmProvider,
  isValidCidString,
  lowerHexToBytes,
  multibaseBase58btcEncode,
  multibaseBase64urlEncode,
  multibaseDecode,
  multicodecLookupPrefix,
  multicodecPrefixForName,
  multicodecStripPrefix,
  multicodecTable,
  multikeyEncode,
  multikeyParse,
  processOperation,
  processOperationJson,
  requireSupportedMulticodec,
  tryParseCid,
  validateKeyBinding,
} from "../dist/index.js";
import {
  CodecDagCborDecodeRequestSchema,
  CodecDagCborDecodeResultSchema,
  CodecDagCborEncodeRequestSchema,
  CodecDagCborEncodeResultSchema,
  CodecDeterministicCborEncodeRequestSchema,
  CodecDeterministicCborValueSchema,
  CodecErrorReason,
  CodecMulticodecPrefixForNameRequestSchema,
  CodecOperationRequestSchema,
  CodecOperationResponseSchema,
} from "../dist/proto.js";
import {
  MAX_CODEC_FFI_INPUT_BYTES,
  MAX_CODEC_PROTO_JSON_BYTES,
  MAX_CODEC_PROTO_MESSAGE_BYTES,
} from "../dist/boundary.js";
import {
  readIndependentBoundedBytesOutput,
  readStringProperty,
} from "../dist/readOutput.js";

const wasmBytes = readFileSync(
  new URL("../dist/wasm/reallyme_codec_wasm_bg.wasm", import.meta.url),
);
const codecVectorManifest = JSON.parse(
  readFileSync(
    new URL("../../../vectors/codec-vectors.json", import.meta.url),
    "utf8",
  ),
);
assert.equal(codecVectorManifest.schemaVersion, 2);
const codecVectors = codecVectorManifest.vectors;
const deterministicCborVectors = codecVectorManifest.deterministicCbor;

wasm.initSync({ module: wasmBytes });
installReallyMeCodecWasmProvider(wasm);

const hasOnlyUnicodeScalars = (value) => {
  for (let index = 0; index < value.length; index += 1) {
    const codeUnit = value.charCodeAt(index);
    if (codeUnit >= 0xd800 && codeUnit <= 0xdbff) {
      if (index + 1 >= value.length) {
        return false;
      }
      const next = value.charCodeAt(index + 1);
      if (next < 0xdc00 || next > 0xdfff) {
        return false;
      }
      index += 1;
    } else if (codeUnit >= 0xdc00 && codeUnit <= 0xdfff) {
      return false;
    }
  }
  return true;
};

const jsonStringArbitrary = fc
  .string({ maxLength: 16 })
  .filter(hasOnlyUnicodeScalars);

const jsonValueArbitrary = fc.letrec((tie) => ({
  value: fc.oneof(
    fc.constant(null),
    fc.boolean(),
    fc.double({
      min: -1_000_000,
      max: 1_000_000,
      noDefaultInfinity: true,
      noNaN: true,
    }),
    jsonStringArbitrary,
    fc.array(tie("value"), { maxLength: 4 }),
    fc.dictionary(jsonStringArbitrary, tie("value"), { maxKeys: 4 }),
  ),
})).value;

const bytes = (...values) => Uint8Array.from(values);
const hex = (value) => Buffer.from(value).toString("hex");
const bytesFromHex = (value) => Uint8Array.from(Buffer.from(value, "hex"));
const utf8 = (value) => new TextEncoder().encode(value);
const dagCborVectorValue = () => ({
  type: "map",
  value: [
    { key: "b", value: { type: "int", value: 2 } },
    { key: "a", value: { type: "string", value: "one" } },
    { key: "bytes", value: { type: "bytes", value: bytes(0, 1, 2) } },
  ],
});

const assertCodecError = (operation, code) => {
  assert.throws(
    operation,
    (error) => error instanceof ReallyMeCodecError && error.code === code,
  );
};

const assertWasmError = (operation, code) => {
  assert.throws(operation, (error) => error === code);
};

const assertCodecRejected = (operation) => {
  assert.throws(operation, (error) => error instanceof ReallyMeCodecError);
};

const deterministicFixtureInteger = (value) => {
  if (Object.hasOwn(value, "unsigned")) {
    return { type: "unsigned", value: BigInt(value.unsigned) };
  }
  if (Object.hasOwn(value, "negative")) {
    return { type: "negative", value: BigInt(value.negative) };
  }
  throw new TypeError("invalid deterministic-CBOR integer fixture");
};

const deterministicFixtureMapKey = (value) => {
  if (Object.hasOwn(value, "integer")) {
    return { type: "integer", value: deterministicFixtureInteger(value.integer) };
  }
  if (Object.hasOwn(value, "text")) {
    return { type: "text", value: value.text };
  }
  throw new TypeError("invalid deterministic-CBOR map-key fixture");
};

const deterministicFixtureValue = (value) => {
  if (Object.hasOwn(value, "unsigned") || Object.hasOwn(value, "negative")) {
    return { type: "integer", value: deterministicFixtureInteger(value) };
  }
  if (Object.hasOwn(value, "text")) {
    return { type: "text", value: value.text };
  }
  if (Object.hasOwn(value, "bytes")) {
    return {
      type: "bytes",
      value: Uint8Array.from(Buffer.from(value.bytes, "base64")),
    };
  }
  if (Object.hasOwn(value, "bool")) {
    return { type: "bool", value: value.bool };
  }
  if (Object.hasOwn(value, "null")) {
    return { type: "null" };
  }
  if (Object.hasOwn(value, "array")) {
    return { type: "array", value: value.array.map(deterministicFixtureValue) };
  }
  if (Object.hasOwn(value, "map")) {
    return {
      type: "map",
      value: value.map.map((entry) => ({
        key: deterministicFixtureMapKey(entry.key),
        value: deterministicFixtureValue(entry.value),
      })),
    };
  }
  throw new TypeError("invalid deterministic-CBOR value fixture");
};

test("WASM exports match the TypeScript provider contract", () => {
  for (const name of REALLYME_CODEC_WASM_EXPORTS) {
    assert.equal(typeof wasm[name], "function", name);
  }
});

test("superseded direct WASM structured result exports are absent", () => {
  for (const name of [
    "multicodecPrefixForName",
    "multicodecLookupPrefix",
    "multicodecTable",
    "multikeyParse",
    "dagCborVerifyCid",
    "pemDecode",
  ]) {
    assert.equal(Object.hasOwn(wasm, name), false, name);
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

test("byte boundaries reject proxy-wrapped typed arrays before length reads", () => {
  const proxied = new Proxy(Uint8Array.of(0xff), {});
  assertCodecError(() => base64Encode(proxied), "invalid-input");
  assertCodecError(() => processOperation(proxied), "invalid-input");
  assertCodecError(() => decodePem(proxied), "invalid-input");
  assertCodecError(() => deterministicCborDecode(proxied), "invalid-input");

  const forged = new DataView(new ArrayBuffer(1));
  Object.defineProperty(forged, Symbol.toStringTag, {
    configurable: true,
    value: "Uint8Array",
  });
  assertCodecError(() => base58btcEncode(forged), "invalid-input");
  assertCodecError(() => multicodecStripPrefix(forged), "invalid-input");
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
      "ReallyMeDagCbor",
      "ReallyMeDeterministicCbor",
      "dagCborCodecCode",
      "dagCborComputeCid",
      "dagCborDecode",
      "dagCborEncode",
      "dagCborMultihash",
      "dagCborSha256ContentHash",
      "dagCborVerifyCid",
      "deterministicCborDecode",
      "deterministicCborEncode",
      "decodePem",
      "encodePem",
      "isValidCidString",
      "lowerHexToBytes",
      "multibaseBase58btcEncode",
      "multibaseBase64urlEncode",
      "multibaseDecode",
      "multicodecLookupPrefix",
      "multicodecPrefixForName",
      "multicodecStripPrefix",
      "multicodecTable",
      "multikeyEncode",
      "multikeyParse",
      "processOperation",
      "processOperationJson",
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
  assert.equal(metadata.name, codecVectors.ed25519CodecName);
  assert.equal(metadata.alg, codecVectors.ed25519AlgorithmName);
  assert.equal(metadata.tag, codecVectors.ed25519Tag);
  assert.equal(metadata.keyMaterial, codecVectors.ed25519KeyMaterial);
  assert.equal(metadata.expectedKeyLength, codecVectors.ed25519ExpectedKeyLength);
  assert.equal(hex(metadata.prefix), codecVectors.ed25519PrefixHex);

  const lookup = multicodecLookupPrefix(prefixed);
  assert.equal(lookup.name, codecVectors.ed25519CodecName);
  assert.deepEqual(multicodecStripPrefix(prefixed), publicKey);
  assert.ok(multicodecTable().entries.some((entry) => entry.name === codecVectors.multicodecTableRequiredName));

  assert.equal(
    multikeyEncode(codecVectors.ed25519CodecName, publicKey),
    codecVectors.ed25519Multikey,
  );
  const parsed = multikeyParse(codecVectors.ed25519Multikey);
  assert.equal(parsed.codecName, codecVectors.ed25519CodecName);
  assert.equal(parsed.algorithmName, codecVectors.ed25519AlgorithmName);
  assert.deepEqual(parsed.publicKey, publicKey);
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

  const encoded = dagCborEncode(dagCborVectorValue());
  assert.equal(hex(encoded), codecVectors.dagCborEncodedHex);
  assert.equal(hex(dagCborEncode(dagCborDecode(encoded))), codecVectors.dagCborEncodedHex);
  assert.equal(dagCborComputeCid(encoded), codecVectors.dagCborCid);
  assert.equal(hex(dagCborSha256ContentHash(encoded)), codecVectors.dagCborSha256Hex);
  assert.equal(hex(dagCborMultihash(encoded)), codecVectors.dagCborMultihashHex);
  assert.equal(dagCborCodecCode(), codecVectors.dagCborCodecCode);
  assert.equal(isValidCidString(codecVectors.dagCborCid), true);
  assert.equal(tryParseCid(codecVectors.dagCborCid), codecVectors.dagCborCid);
  assert.equal(isValidCidString(codecVectors.invalidCid), false);
  assert.equal(tryParseCid(codecVectors.invalidCid), undefined);
  assert.equal(dagCborVerifyCid(codecVectors.dagCborCid, encoded).valid, true);

  const deterministic = {
    type: "map",
    value: [
      {
        key: { type: "text", value: "z" },
        value: { type: "bytes", value: bytes(1, 2, 3) },
      },
      {
        key: { type: "integer", value: { type: "negative", value: -1n } },
        value: { type: "integer", value: { type: "unsigned", value: 18446744073709551615n } },
      },
      {
        key: { type: "integer", value: { type: "unsigned", value: 9007199254740992n } },
        value: { type: "text", value: "wide" },
      },
    ],
  };
  const deterministicEncoded = deterministicCborEncode(deterministic);
  assert.equal(hex(deterministicEncoded), "a3201bffffffffffffffff617a430102031b00200000000000006477696465");
  assert.deepEqual(
    deterministicCborDecode(deterministicEncoded),
    {
      type: "map",
      value: [
        {
          key: { type: "integer", value: { type: "negative", value: -1n } },
          value: { type: "integer", value: { type: "unsigned", value: 18446744073709551615n } },
        },
        {
          key: { type: "text", value: "z" },
          value: { type: "bytes", value: bytes(1, 2, 3) },
        },
        {
          key: { type: "integer", value: { type: "unsigned", value: 9007199254740992n } },
          value: { type: "text", value: "wide" },
        },
      ],
    },
  );

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
  const response = fromBinary(
    CodecOperationResponseSchema,
    processOperation(bytesFromHex(codecVectors.protoMulticodecTableRequestHex)),
  );
  assert.equal(response.outcome.case, "result");
  assert.equal(response.outcome.value.result.case, "multicodecTable");
  assert.ok(
    response.outcome.value.result.value.entries.some(
      (entry) => entry.name === codecVectors.multicodecTableRequiredName,
    ),
  );
  const jsonResponse = fromBinary(
    CodecOperationResponseSchema,
    processOperationJson(utf8(codecVectors.protoMulticodecTableRequestJson)),
  );
  assert.deepEqual(jsonResponse, response);
});

test("shared codec vector suite rejects non-canonical inputs in TypeScript", () => {
  assertCodecRejected(() => base64Decode(codecVectors.base64MissingPadding));
  assertCodecRejected(() => base64Decode(codecVectors.base64NonCanonicalTrailingBits));
  assertCodecRejected(() => base64Decode(codecVectors.base64Whitespace));
  assertCodecRejected(() => base64urlDecode(codecVectors.base64urlPadded));
  assertCodecRejected(() => base64urlDecode(codecVectors.base64urlNonCanonicalTrailingBits));
  assertCodecRejected(() => base64urlDecode(codecVectors.base64urlInvalidLength));
  assertCodecRejected(() => base64urlDecode(codecVectors.base64urlWhitespace));
  assertCodecRejected(() => multibaseDecode(codecVectors.unsupportedMultibase));
  assertCodecRejected(() => multibaseDecode(codecVectors.multibaseMultibytePrefix));
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
  assertCodecRejected(() => deterministicCborDecode(bytes(0x18, 0x00)));
  assertCodecRejected(() => canonicalizeJsonText(codecVectors.jcsDuplicateMemberJson));
  assertCodecRejected(() => canonicalizeJsonText(codecVectors.jcsNonInteroperableIntegerJson));
  assertCodecRejected(() => canonicalizeJsonText(codecVectors.jcsLoneSurrogateJson));
  assert.equal(
    canonicalizeJsonText(codecVectors.jcsUtf16KeyOrderInputJson),
    codecVectors.jcsUtf16KeyOrderCanonicalJson,
  );
});

test("shared deterministic-CBOR literals and idkit fixture match byte for byte", () => {
  assert.equal(
    deterministicCborVectors.profile,
    "rfc8949-core-deterministic-reallyme-0.2.0",
  );
  assert.equal(deterministicCborVectors.fixtureClasses.positive, "golden");
  assert.equal(deterministicCborVectors.fixtureClasses.negative, "rejection-fixture");
  assert.equal(
    deterministicCborVectors.fixtureClasses.resourceRejections,
    "construction-recipe",
  );
  assert.equal(deterministicCborVectors.fixtureClasses.interoperability, "interop-fixture");
  for (const vector of deterministicCborVectors.positive) {
    const canonical = bytesFromHex(vector.hex);
    assert.equal(
      hex(deterministicCborEncode(deterministicFixtureValue(vector.value))),
      vector.hex,
    );
    assert.equal(hex(deterministicCborEncode(deterministicCborDecode(canonical))), vector.hex);
  }
  for (const vector of deterministicCborVectors.negative) {
    assertCodecError(
      () => deterministicCborDecode(bytesFromHex(vector.hex)),
      "invalid-input",
    );
  }
  for (const vector of deterministicCborVectors.equivalentInputOrders) {
    for (const entries of vector.inputs) {
      const value = {
        type: "map",
        value: entries.map((entry) => ({
          key: deterministicFixtureMapKey(entry.key),
          value: deterministicFixtureValue(entry.value),
        })),
      };
      assert.equal(hex(deterministicCborEncode(value)), vector.hex);
    }
  }

  const interoperabilityNames = new Set(
    deterministicCborVectors.interoperability.map((vector) => vector.name),
  );
  assert.equal(interoperabilityNames.has("idkit-ios-synthetic-passport-claims-v1"), true);
  assert.equal(
    interoperabilityNames.has("idkit-ios-synthetic-passport-claims-null-place-of-birth-v1"),
    true,
  );
  assert.equal(interoperabilityNames.has("idkit-ios-synthetic-fingerprint-map-v1"), true);
  assert.equal(interoperabilityNames.has("idkit-ios-synthetic-mixed-integer-claim-tags-v1"), true);
  for (const vector of deterministicCborVectors.interoperability) {
    assert.equal(vector.fixtureKind, "synthetic");
    assert.equal(vector.sourceRepo, "reallyme/idkit-ios");
    assert.equal(vector.sourceCommit, "content-hash-pinned");
    assert.equal(typeof vector.source, "string");
    assert.equal(vector.source.length > 0, true);
    assert.equal(typeof vector.explanation, "string");
    assert.equal(vector.explanation.length > 0, true);
    assert.equal(Array.isArray(vector.sourceFiles), true);
    assert.equal(vector.sourceFiles.length > 0, true);
    for (const sourceFile of vector.sourceFiles) {
      assert.equal(typeof sourceFile.path, "string");
      assert.equal(sourceFile.path.length > 0, true);
      assert.match(sourceFile.sha256, /^[0-9a-f]{64}$/u);
    }
    const vectorBytes = bytesFromHex(vector.hex);
    assert.equal(vectorBytes.length, vector.byteLength);
    assert.equal(createHash("sha256").update(vectorBytes).digest("hex"), vector.sha256);
    const value = deterministicCborDecode(vectorBytes);
    assert.equal(value.type, "map");
    assert.equal(value.value.length, vector.entryCount);
    assert.equal(hex(deterministicCborEncode(value)), vector.hex);
  }
});

test("CBOR helper builders preserve canonical bytes", () => {
  const deterministic = ReallyMeDeterministicCbor.mapText([
    ["b", ReallyMeDeterministicCbor.unsigned(2n)],
    ["a", ReallyMeDeterministicCbor.unsigned(1n)],
  ]);
  assert.equal(hex(deterministicCborEncode(deterministic)), "a2616101616202");

  const deterministicIntegerMap = ReallyMeDeterministicCbor.mapInt([
    [2n, ReallyMeDeterministicCbor.text("b")],
    [1n, ReallyMeDeterministicCbor.text("a")],
  ]);
  assert.equal(
    hex(deterministicCborEncode(deterministicIntegerMap)),
    "a2016161026162",
  );

  const dag = ReallyMeDagCbor.mapText([
    ["b", ReallyMeDagCbor.unsigned(2)],
    ["a", ReallyMeDagCbor.text("one")],
    ["bytes", ReallyMeDagCbor.bytes(new Uint8Array([0, 1, 2]))],
  ]);
  const encoded = dagCborEncode(dag);
  assert.equal(hex(encoded), codecVectors.dagCborEncodedHex);
  assert.equal(dagCborComputeCid(encoded), codecVectors.dagCborCid);
  assert.equal(dagCborVerifyCid(codecVectors.dagCborCid, encoded).valid, true);

  const dagConvenience = ReallyMeDagCbor.mapText([
    ["b", ReallyMeDagCbor.unsigned(2)],
    ["a", ReallyMeDagCbor.bytes(new Uint8Array([0, 1, 2]))],
  ]);
  assert.equal(
    hex(dagCborEncode(dagConvenience)),
    "a2616143000102616202",
  );
  assert.equal(hex(dagCborEncode(ReallyMeDagCbor.negative(-1))), "20");

  assertCodecError(
    () => ReallyMeDeterministicCbor.mapText([
      ["a", ReallyMeDeterministicCbor.null()],
      ["a", ReallyMeDeterministicCbor.bool(true)],
    ]),
    "invalid-input",
  );
});

test("shared deterministic-CBOR resource recipes fail at the TypeScript boundary", () => {
  const recipe = (name) => {
    const value = deterministicCborVectors.resourceRejections.find(
      (candidate) => candidate.name === name,
    );
    assert.notEqual(value, undefined);
    return value.construction;
  };

  const inputBytes = recipe("input-byte-limit-plus-one");
  assertCodecError(
    () =>
      deterministicCborDecode(
        new Uint8Array(inputBytes.count).fill(
          Number.parseInt(inputBytes.fillByteHex, 16),
        ),
      ),
    "invalid-input",
  );

  const byteString = recipe("output-byte-limit-plus-header");
  assertCodecError(
    () =>
      deterministicCborEncode({
        type: "bytes",
        value: new Uint8Array(byteString.count),
      }),
    "invalid-input",
  );

  const container = recipe("container-entry-limit-plus-one");
  assertCodecError(
    () =>
      deterministicCborEncode({
        type: "array",
        value: Array.from({ length: container.count }, () => ({ type: "null" })),
      }),
    "invalid-input",
  );

  const nesting = recipe("nesting-depth-limit-plus-one");
  let nested = { type: "null" };
  for (let depth = 0; depth < nesting.depth; depth += 1) {
    nested = { type: "array", value: [nested] };
  }
  assertCodecError(() => deterministicCborEncode(nested), "invalid-input");

  const balanced = recipe("node-limit-exceeded-balanced-tree");
  const balancedTree = (levels) => {
    if (levels === 0) {
      return { type: "null" };
    }
    return {
      type: "array",
      value: Array.from(
        { length: balanced.branching },
        () => balancedTree(levels - 1),
      ),
    };
  };
  assertCodecError(
    () => deterministicCborEncode(balancedTree(balanced.levels)),
    "invalid-input",
  );

  const duplicateTextKey = {
    type: "text",
    value: "duplicate",
  };
  assertCodecError(
    () =>
      deterministicCborEncode({
        type: "map",
        value: [
          { key: duplicateTextKey, value: { type: "null" } },
          { key: duplicateTextKey, value: { type: "bool", value: true } },
        ],
      }),
    "invalid-input",
  );

  const duplicateIntegerKey = {
    type: "integer",
    value: { type: "unsigned", value: 7n },
  };
  assertCodecError(
    () =>
      deterministicCborEncode({
        type: "map",
        value: [
          { key: duplicateIntegerKey, value: { type: "null" } },
          { key: duplicateIntegerKey, value: { type: "bool", value: false } },
        ],
      }),
    "invalid-input",
  );
});

test("deterministic CBOR reaches the exact semantic byte boundary through WASM", () => {
  // A byte string with a five-byte CBOR length header reaches the one-megabyte
  // encoded limit exactly. This guards against transport framing silently
  // making a semantically valid maximum-sized value unreachable.
  const payload = new Uint8Array(1_048_576 - 5);
  payload[0] = 0xa5;
  payload[payload.length - 1] = 0x5a;
  const encoded = deterministicCborEncode({ type: "bytes", value: payload });
  try {
    assert.equal(encoded.length, 1_048_576);
    const decoded = deterministicCborDecode(encoded);
    assert.equal(decoded.type, "bytes");
    try {
      assert.equal(decoded.value.length, payload.length);
      assert.equal(decoded.value[0], 0xa5);
      assert.equal(decoded.value[decoded.value.length - 1], 0x5a);
    } finally {
      decoded.value.fill(0);
    }
  } finally {
    encoded.fill(0);
    payload.fill(0);
  }
});

test("deterministic CBOR typed API snapshots, bounds, and uses generated protobuf", () => {
  const byteString = bytes(0xaa, 0xbb);
  const value = {
    type: "array",
    value: [
      { type: "integer", value: { type: "unsigned", value: 0n } },
      { type: "integer", value: { type: "unsigned", value: 9007199254740991n } },
      { type: "integer", value: { type: "unsigned", value: 9007199254740992n } },
      { type: "integer", value: { type: "negative", value: -9223372036854775808n } },
      { type: "bytes", value: byteString },
    ],
  };

  const encoded = deterministicCborEncode(value);
  byteString.fill(0);
  assert.equal(hex(encoded), "85001b001fffffffffffff1b00200000000000003b7fffffffffffffff42aabb");

  const decoded = deterministicCborDecode(encoded);
  assert.deepEqual(decoded, {
    type: "array",
    value: [
      { type: "integer", value: { type: "unsigned", value: 0n } },
      { type: "integer", value: { type: "unsigned", value: 9007199254740991n } },
      { type: "integer", value: { type: "unsigned", value: 9007199254740992n } },
      { type: "integer", value: { type: "negative", value: -9223372036854775808n } },
      { type: "bytes", value: bytes(0xaa, 0xbb) },
    ],
  });

  const protoResponse = fromBinary(CodecOperationResponseSchema, processOperation(toBinary(
    CodecOperationRequestSchema,
    create(CodecOperationRequestSchema, {
      operation: {
        case: "deterministicCborEncode",
        value: create(CodecDeterministicCborEncodeRequestSchema, {
          value: create(CodecDeterministicCborValueSchema, {
            value: {
              case: "bytesValue",
              value: { value: bytes(0x01, 0x02) },
            },
          }),
        }),
      },
    }),
  )));
  assert.equal(protoResponse.outcome.case, "result");
  assert.equal(protoResponse.outcome.value.result.case, "deterministicCborEncode");
  assert.equal(
    hex(protoResponse.outcome.value.result.value.encoded),
    "420102",
  );
  const protoDecodeResponse = fromBinary(CodecOperationResponseSchema, processOperation(
    toBinary(CodecOperationRequestSchema, create(CodecOperationRequestSchema, {
      operation: {
        case: "deterministicCborDecode",
        value: { encoded: bytes(0x42, 0x01, 0x02) },
      },
    })),
  ));
  assert.equal(protoDecodeResponse.outcome.case, "result");
  assert.equal(protoDecodeResponse.outcome.value.result.case, "deterministicCborDecode");
  assert.equal(
    hex(protoDecodeResponse.outcome.value.result.value.value.value.value.value),
    "0102",
  );
  assert.equal(
    hex(fromBinary(CodecDeterministicCborEncodeRequestSchema, toBinary(
      CodecDeterministicCborEncodeRequestSchema,
      create(CodecDeterministicCborEncodeRequestSchema, {
        value: create(CodecDeterministicCborValueSchema, {
          value: {
            case: "bytesValue",
            value: { value: bytes(0x03) },
          },
        }),
      }),
    )).value.value.value.value),
    "03",
  );

  assertCodecError(
    () => deterministicCborEncode({ type: "integer", value: { type: "unsigned", value: -1n } }),
    "invalid-input",
  );
  assertCodecError(
    () => deterministicCborEncode({ type: "integer", value: { type: "negative", value: 0n } }),
    "invalid-input",
  );
  assertCodecError(
    () =>
      deterministicCborEncode({
        type: "integer",
        value: { type: "unsigned", value: 18446744073709551616n },
      }),
    "invalid-input",
  );
});

test("deterministic CBOR rejects hostile JavaScript shapes", () => {
  let getterInvoked = false;
  const hostile = {
    type: "bytes",
  };
  Object.defineProperty(hostile, "value", {
    enumerable: true,
    get() {
      getterInvoked = true;
      return bytes(1);
    },
  });
  assertCodecError(() => deterministicCborEncode(hostile), "invalid-input");
  assert.equal(getterInvoked, false);

  const withSymbol = {
    type: "text",
    value: "x",
    [Symbol("leak")]: "ignored",
  };
  assertCodecError(() => deterministicCborEncode(withSymbol), "invalid-input");

  const cyclic = { type: "array", value: [] };
  cyclic.value.push(cyclic);
  assertCodecError(() => deterministicCborEncode(cyclic), "invalid-input");

  const sparse = [];
  sparse.length = 1;
  assertCodecError(
    () => deterministicCborEncode({ type: "array", value: sparse }),
    "invalid-input",
  );

  const detachedDecodeBytes = bytes(0x40);
  structuredClone(detachedDecodeBytes.buffer, { transfer: [detachedDecodeBytes.buffer] });
  assertCodecError(() => deterministicCborDecode(detachedDecodeBytes), "invalid-input");

  const detachedEncodeBytes = bytes(0x01);
  structuredClone(detachedEncodeBytes.buffer, { transfer: [detachedEncodeBytes.buffer] });
  assertCodecError(
    () => deterministicCborEncode({ type: "bytes", value: detachedEncodeBytes }),
    "invalid-input",
  );

  assertCodecError(
    () => deterministicCborEncode({ type: "text", value: "\ud800" }),
    "invalid-input",
  );

  const withUnexpectedArrayProperty = [{ type: "null" }];
  Object.defineProperty(withUnexpectedArrayProperty, "metadata", {
    configurable: true,
    enumerable: false,
    value: "not part of the value model",
  });
  assertCodecError(
    () =>
      deterministicCborEncode({
        type: "array",
        value: withUnexpectedArrayProperty,
      }),
    "invalid-input",
  );

  let valueDescriptorReads = 0;
  const changingProxy = new Proxy(
    { type: "text", value: "safe" },
    {
      getOwnPropertyDescriptor(target, property) {
        if (property === "value") {
          valueDescriptorReads += 1;
          return {
            configurable: true,
            enumerable: true,
            value: valueDescriptorReads === 1 ? "safe" : "changed",
            writable: true,
          };
        }
        return Reflect.getOwnPropertyDescriptor(target, property);
      },
    },
  );
  assert.equal(hex(deterministicCborEncode(changingProxy)), "6473616665");
  assert.equal(valueDescriptorReads, 1);
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
  assert.equal(metadata.name, "ed25519-pub");
  assert.equal(metadata.alg, "Ed25519");
  assert.equal(metadata.tag, "key");
  assert.equal(metadata.keyMaterial, "public-key");
  assert.equal(metadata.expectedKeyLength, 32);

  const prefixed = new Uint8Array(metadata.prefix.length + publicKey.length);
  prefixed.set(metadata.prefix);
  prefixed.set(publicKey, metadata.prefix.length);
  const lookup = multicodecLookupPrefix(prefixed);
  assert.equal(lookup.name, "ed25519-pub");
  assert.equal(lookup.prefixLength, metadata.prefix.length);
  assert.deepEqual(multicodecStripPrefix(prefixed), publicKey);
  assert.ok(multicodecTable().entries.some((entry) => entry.name === "mlkem-1024-pub"));

  const multikey = multikeyEncode("ed25519-pub", publicKey);
  const parsed = multikeyParse(multikey);
  assert.equal(parsed.codecName, "ed25519-pub");
  assert.equal(parsed.algorithmName, "Ed25519");
  assert.deepEqual(parsed.publicKey, publicKey);
  assert.equal(parsed.expectedPublicKeyLength, 32);

  const nonCanonicalMultikey = `u${base64urlEncode(prefixed)}`;
  assertCodecError(() => multikeyParse(nonCanonicalMultikey), "invalid-input");

  assert.equal(bindingTypeMatchesCodec("Multikey", parsed.codecName), true);
  validateKeyBinding("Multikey", undefined, multikey);
  assertCodecError(() => validateKeyBinding("P256Key2024", "P-256", multikey), "invalid-input");
  assertCodecError(() => requireSupportedMulticodec("not-a-codec"), "unsupported-codec");
  assertCodecError(() => multikeyEncode("not-a-codec", publicKey), "unsupported-codec");

  assertCodecError(() => multikeyParse("not-a-key"), "invalid-input");
  const unknownPrefixMultikey = multibaseBase58btcEncode(bytes(0, 0, 7));
  assertCodecError(() => multikeyParse(unknownPrefixMultikey), "invalid-input");
  assertCodecError(() => multicodecPrefixForName("not-a-codec"), "invalid-input");
});

test("DAG-CBOR encode/decode and CID helpers use the Rust codec", () => {
  const value = {
    type: "map",
    value: [
      { key: "b", value: { type: "int", value: 2 } },
      { key: "a", value: { type: "string", value: "one" } },
      { key: "bytes", value: { type: "bytes", value: bytes(0, 1, 2) } },
    ],
  };
  const encoded = dagCborEncode(value);
  const decoded = dagCborDecode(encoded);
  assert.deepEqual(decoded, {
    type: "map",
    value: [
      { key: "a", value: { type: "string", value: "one" } },
      { key: "b", value: { type: "int", value: 2n } },
      { key: "bytes", value: { type: "bytes", value: bytes(0, 1, 2) } },
    ],
  });
  const decodedBytes = decoded.value.find((entry) => entry.key === "bytes")?.value;
  assert.equal(decodedBytes?.type, "bytes");
  assert.equal(decodedBytes.value instanceof Uint8Array, true);

  const cid = dagCborComputeCid(encoded);
  assert.equal(isValidCidString(cid), true);
  assert.equal(tryParseCid(cid), cid);
  assert.equal(isValidCidString(""), false);
  assert.equal(tryParseCid(""), undefined);
  assert.equal(dagCborCodecCode(), 0x71);
  assert.equal(dagCborVerifyCid(cid, encoded).valid, true);
  const invalidUpperPayloadCid = `${cid[0]}${cid.slice(1).toUpperCase()}`;
  const invalidVerification = dagCborVerifyCid(invalidUpperPayloadCid, encoded);
  assert.equal(invalidVerification.valid, false);
  assert.equal(invalidVerification.expectedCid, cid);
  assert.equal(invalidVerification.actualCid, "");
  const emptyCidVerification = dagCborVerifyCid("", encoded);
  assert.equal(emptyCidVerification.valid, false);
  assert.equal(emptyCidVerification.expectedCid, cid);
  assert.equal(emptyCidVerification.actualCid, "");

  const largeInteger = { type: "int", value: 9_007_199_254_740_993n };
  assert.deepEqual(dagCborDecode(dagCborEncode(largeInteger)), largeInteger);
  assert.equal(hex(dagCborSha256ContentHash(encoded)).length, 64);
  assert.ok(dagCborMultihash(encoded).length > 32);
  assertCodecRejected(() => dagCborDecode(bytes(0xa2, 0x61, 0x62, 0x01, 0x61, 0x61, 0x02)));

  const oversizedCbor = new Uint8Array(1024 * 1024 + 1);
  assertCodecError(() => dagCborDecode(oversizedCbor), "invalid-input");
  assertCodecError(() => dagCborComputeCid(oversizedCbor), "invalid-input");
  assertCodecError(() => dagCborVerifyCid(cid, oversizedCbor), "invalid-input");
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
  assertCodecError(() => canonicalizeJsonText("1e19"), "invalid-input");
});

test("JCS object boundary matches text boundary for bounded JSON values", () => {
  fc.assert(
    fc.property(jsonValueArbitrary, (value) => {
      const encoded = JSON.stringify(value);
      assert.equal(typeof encoded, "string");
      assert.equal(canonicalizeJson(value), canonicalizeJsonText(encoded));
    }),
    { numRuns: 64, seed: 0x526d_0200 },
  );
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
});

test("PEM armor uses Rust policy for labels, wrapping, and decoding", () => {
  const der = Uint8Array.from(Buffer.from("not real der"));
  const pem = encodePem("PUBLIC KEY", der, { lineWidth: 4 });
  assert.deepEqual(
    pem,
    utf8("-----BEGIN PUBLIC KEY-----\nbm90\nIHJl\nYWwg\nZGVy\n-----END PUBLIC KEY-----\n"),
  );
  const decoded = decodePem(pem, { allowedLabels: ["PUBLIC KEY"] });
  assert.equal(decoded.label, "PUBLIC KEY");
  assert.deepEqual(decoded.der, der);
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
    () => decodePem(pem, { allowedLabels: ["PRIVATE KEY"] }),
    "invalid-input",
  );
});

test("decodePem rejects oversized proto policy before provider work", () => {
  const privateKeyPem = utf8(codecVectors.pemPrivatePem);
  assertCodecError(
    () => decodePem(privateKeyPem, { maxInputLen: 2 ** 40 }),
    "invalid-input",
  );
});

test("encodePem rejects oversized proto options before provider work", () => {
  const privateDer = bytesFromHex(codecVectors.pemPrivateDerHex);
  assertCodecError(
    () => encodePem(codecVectors.pemPrivateLabel, privateDer, { maxDerLen: 2 ** 40 }),
    "invalid-input",
  );
});

test("codec proto exports carry codec-owned error reasons", () => {
  assert.equal(CodecErrorReason.BASE_INVALID_HEX, 120);
});

test("codec proto facade exports DAG-CBOR operation schemas", () => {
  assert.equal(
    CodecDagCborEncodeRequestSchema.typeName,
    "reallyme.codec.v1.CodecDagCborEncodeRequest",
  );
  assert.equal(
    CodecDagCborEncodeResultSchema.typeName,
    "reallyme.codec.v1.CodecDagCborEncodeResult",
  );
  assert.equal(
    CodecDagCborDecodeRequestSchema.typeName,
    "reallyme.codec.v1.CodecDagCborDecodeRequest",
  );
  assert.equal(
    CodecDagCborDecodeResultSchema.typeName,
    "reallyme.codec.v1.CodecDagCborDecodeResult",
  );
});

test("binary protobuf and generated ProtoJSON return equivalent responses", () => {
  const request = create(CodecOperationRequestSchema, {
    operation: {
      case: "multicodecPrefixForName",
      value: create(CodecMulticodecPrefixForNameRequestSchema, {
        name: "ed25519-pub",
      }),
    },
  });
  const binaryResponse = fromBinary(
    CodecOperationResponseSchema,
    processOperation(toBinary(CodecOperationRequestSchema, request)),
  );
  const jsonResponse = fromBinary(
    CodecOperationResponseSchema,
    processOperationJson(
      new TextEncoder().encode(
        '{"multicodecPrefixForName":{"name":"ed25519-pub"}}',
      ),
    ),
  );

  assert.equal(binaryResponse.outcome.case, "result");
  assert.equal(jsonResponse.outcome.case, "result");
  assert.deepEqual(jsonResponse, binaryResponse);
});

test("malformed protobuf and ProtoJSON fail inside typed responses", () => {
  const malformedProtobuf = fromBinary(
    CodecOperationResponseSchema,
    processOperation(bytes(0xff)),
  );
  assert.equal(malformedProtobuf.outcome.case, "error");
  const protobufError = malformedProtobuf.outcome.value;
  assert.equal(protobufError.error.case, "boundary");
  assert.equal(
    protobufError.error.value.reason,
    CodecErrorReason.BOUNDARY_MALFORMED_PROTOBUF,
  );

  const malformedJson = fromBinary(
    CodecOperationResponseSchema,
    processOperationJson(new TextEncoder().encode('{"unknownOperation":{}}')),
  );
  assert.equal(malformedJson.outcome.case, "error");
  const jsonError = malformedJson.outcome.value;
  assert.equal(jsonError.error.case, "boundary");
  assert.equal(
    jsonError.error.value.reason,
    CodecErrorReason.BOUNDARY_MALFORMED_JSON,
  );
});

test("WASM boundaries reject oversized non-proto inputs before codec allocation", () => {
  const oversizedRaw = new Uint8Array(MAX_CODEC_FFI_INPUT_BYTES + 1);
  assertCodecError(() => base64Encode(oversizedRaw), "invalid-input");
  assertCodecError(() => decodePem(oversizedRaw), "invalid-input");
  assertCodecError(
    () => canonicalizeJsonText(" ".repeat(MAX_CODEC_FFI_INPUT_BYTES + 1)),
    "invalid-input",
  );
});

test("WASM string boundaries enforce UTF-8 byte length before Rust string copy", () => {
  const threeByteScalar = "\u0800";
  const jsonText = JSON.stringify(
    threeByteScalar.repeat(Math.floor(MAX_CODEC_FFI_INPUT_BYTES / 3) + 1),
  );
  assert.ok(jsonText.length <= MAX_CODEC_FFI_INPUT_BYTES);
  assert.ok(utf8(jsonText).length > MAX_CODEC_FFI_INPUT_BYTES);
  assertCodecError(() => canonicalizeJsonText(jsonText), "invalid-input");

  const aggregateBoundaryText = threeByteScalar.repeat(
    Math.floor(MAX_CODEC_FFI_INPUT_BYTES / 6) + 1,
  );
  assert.ok(aggregateBoundaryText.length * 2 <= MAX_CODEC_FFI_INPUT_BYTES);
  assert.ok(utf8(aggregateBoundaryText).length * 2 > MAX_CODEC_FFI_INPUT_BYTES);
  assertWasmError(
    () => wasm.bindingTypeMatchesCodec(aggregateBoundaryText, aggregateBoundaryText),
    "invalid-input",
  );
});

test("WASM operation boundaries enforce oversized operation inputs", () => {
  assertCodecError(
    () => processOperation(new Uint8Array(MAX_CODEC_PROTO_MESSAGE_BYTES + 1)),
    "invalid-input",
  );

  assertCodecError(
    () => processOperationJson(new Uint8Array(MAX_CODEC_PROTO_JSON_BYTES + 1)),
    "invalid-input",
  );
});
