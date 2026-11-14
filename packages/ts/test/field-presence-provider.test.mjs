// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { test } from "node:test";
import { create, fromBinary, fromJson, toBinary, toJson } from "@bufbuild/protobuf";
import "./wasm-module-test-hook.mjs";
const wasm = await import("../dist/wasm/reallyme_codec_wasm.js");
const {
  ReallyMeCodecError,
  installReallyMeCodecWasmProvider,
  multicodecPrefixForName,
  multicodecLookupPrefix,
  multicodecTable,
  multikeyParse,
} = await import("../dist/index.js");
const {
  CodecDerValueSchema,
  CodecGeneralCborOptionsSchema,
  CodecKeyMaterialKind,
  CodecOperationResponseSchema,
  CodecTag,
  CodecValidateKeyBindingRequestSchema,
} = await import("../dist/proto.js");

let result;
let providerOutput;
wasm.setOperationHandler(() => {
  providerOutput = toBinary(CodecOperationResponseSchema, create(CodecOperationResponseSchema, {
    outcome: { case: "result", value: { result } },
  }));
  return providerOutput;
});
installReallyMeCodecWasmProvider(wasm);

const metadata = (fixedLength, variableLength) => ({
  name: "ed25519-pub",
  algorithmName: "Ed25519",
  tag: CodecTag.KEY,
  keyMaterialKind: CodecKeyMaterialKind.PUBLIC_KEY,
  prefix: Uint8Array.of(0xed, 1),
  fixedLength,
  variableLength,
});

const assertProviderFailure = (action) => {
  assert.throws(action, (error) => error instanceof ReallyMeCodecError && error.code === "provider-failure");
  assert.ok(providerOutput.length > 0);
  assert.ok(providerOutput.every((byte) => byte === 0));
};

test("all metadata adapters reject contradictory length rules and wipe provider output", () => {
  const invalid = metadata(32, true);
  result = { case: "multicodecPrefixForName", value: invalid };
  assertProviderFailure(() => multicodecPrefixForName("ed25519-pub"));
  result = { case: "multicodecLookupPrefix", value: { name: "ed25519-pub", prefixLength: 2, metadata: invalid } };
  assertProviderFailure(() => multicodecLookupPrefix(Uint8Array.of(0xed, 1)));
  result = { case: "multicodecTable", value: { entries: [metadata(32, false), invalid] } };
  assertProviderFailure(multicodecTable);
});

test("metadata adapters retain all three valid length states", () => {
  for (const [fixed, variable, expected] of [[32, false, 32], [0, true, undefined], [0, false, undefined]]) {
    result = { case: "multicodecPrefixForName", value: metadata(fixed, variable) };
    assert.equal(multicodecPrefixForName("ed25519-pub").expectedKeyLength, expected);
  }
});

test("parsed keys reject contradictory or mismatched lengths and preserve valid rules", () => {
  for (const [expected, variable] of [[32, true], [0, false], [31, false]]) {
    result = { case: "multikeyParse", value: {
      codecName: "ed25519-pub", algorithmName: "Ed25519", publicKey: new Uint8Array(32),
      expectedPublicKeyLength: expected, variablePublicKeyLength: variable,
    } };
    assertProviderFailure(() => multikeyParse("z"));
  }
  for (const [expected, variable] of [[32, false], [0, true]]) {
    result = { case: "multikeyParse", value: {
      codecName: "ed25519-pub", algorithmName: "Ed25519", publicKey: new Uint8Array(32),
      expectedPublicKeyLength: expected, variablePublicKeyLength: variable,
    } };
    const parsed = multikeyParse("z");
    assert.equal(parsed.expectedPublicKeyLength, variable ? undefined : expected);
    assert.equal(parsed.publicKey.length, 32);
    parsed.publicKey.fill(0);
  }
});

test("generated optional fields preserve absent and present-default values in binary and JSON", () => {
  for (const [schema, field, value, literal] of [
    [CodecValidateKeyBindingRequestSchema, "algorithm", "", Uint8Array.of(0x12, 0)],
    [CodecGeneralCborOptionsSchema, "inputBytes", 0n, Uint8Array.of(0x28, 0)],
    [CodecDerValueSchema, "boolValue", false, Uint8Array.of(0x28, 0)],
  ]) {
    assert.equal(fromBinary(schema, new Uint8Array())[field], undefined);
    const message = create(schema, { [field]: value });
    assert.deepEqual(toBinary(schema, message), literal);
    assert.equal(fromBinary(schema, literal)[field], value);
    assert.equal(fromJson(schema, toJson(schema, message))[field], value);
    assert.equal(fromJson(schema, { [field]: null })[field], undefined);
  }
});
