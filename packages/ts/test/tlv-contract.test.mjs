// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { create, fromBinary, fromJsonString, toBinary, toJsonString } from "@bufbuild/protobuf";
import * as wasm from "../dist/wasm/reallyme_codec_wasm.js";
import { installReallyMeCodecWasmProvider, processOperation, processOperationJson } from "../dist/index.js";
import { CodecApiOperationRequestSchema, CodecApiOperationResponseSchema, CodecTlvErrorReason, CodecTlvProfile } from "../dist/proto.js";

const manifest = JSON.parse(readFileSync(new URL("../../../vectors/tlv-framing-vectors.json", import.meta.url), "utf8"));
wasm.initSync({ module: readFileSync(new URL("../dist/wasm/reallyme_codec_wasm_bg.wasm", import.meta.url)) });
installReallyMeCodecWasmProvider(wasm);

test("TLV acceptance receipts and typed failures match independent wire goldens on wasm32", () => {
  assert.equal(manifest.schemaVersion, 1);
  for (const entry of manifest.cases) {
    const request = Uint8Array.from(Buffer.from(entry.requestHex, "hex"));
    const expected = Uint8Array.from(Buffer.from(entry.responseHex, "hex"));
    const json = new TextEncoder().encode(entry.requestJson);
    const binaryResponse = processOperation(request);
    const jsonResponse = processOperationJson(json);
    try {
      assert.deepEqual(toBinary(CodecApiOperationRequestSchema, fromJsonString(CodecApiOperationRequestSchema, entry.requestJson)), request);
      assert.deepEqual(binaryResponse, expected, `${entry.name} ${entry.operation} binary`);
      assert.deepEqual(jsonResponse, expected, `${entry.name} ${entry.operation} JSON`);
      assert.deepEqual(toBinary(CodecApiOperationResponseSchema, fromBinary(CodecApiOperationResponseSchema, expected)), expected);
    } finally {
      request.fill(0);
      json.fill(0);
      expected.fill(0);
      binaryResponse.fill(0);
      jsonResponse.fill(0);
    }
  }
});

test("TLV opts into a full 16 MiB source through binary and ProtoJSON on WASM", () => {
  const maximum = 16 * 1024 * 1024;
  const encoded = new Uint8Array(maximum + 1);
  encoded[0] = 4;
  const options = { profile: CodecTlvProfile.ISO7816_DEFINITE_BER, limits: { inputBytes: BigInt(maximum) } };
  const request = create(CodecApiOperationRequestSchema, {
    operation: { case: "tlvReadHeader", value: { encoded: encoded.subarray(0, maximum), options } },
  });
  const binary = toBinary(CodecApiOperationRequestSchema, request);
  const json = new TextEncoder().encode(toJsonString(CodecApiOperationRequestSchema, request));
  const response = processOperation(binary);
  try {
    assert.deepEqual(processOperationJson(json), response);
    const outcome = fromBinary(CodecApiOperationResponseSchema, response).outcome;
    assert.equal(outcome.case, "result");
    assert.equal(outcome.value.result.case, "tlvReadHeader");
    assert.equal(outcome.value.result.value.consumedBytes, 2n);
    assert.equal(outcome.value.result.value.declaredValueLength, 0n);

    request.operation.value.encoded = encoded;
    const excessive = toBinary(CodecApiOperationRequestSchema, request);
    try {
      const error = fromBinary(CodecApiOperationResponseSchema, processOperation(excessive)).outcome;
      assert.equal(error.case, "error");
      assert.equal(error.value.error.case, "tlv");
      assert.equal(error.value.error.value.detail, CodecTlvErrorReason.INPUT_LIMIT);
    } finally {
      excessive.fill(0);
    }
  } finally {
    encoded.fill(0);
    binary.fill(0);
    json.fill(0);
    response.fill(0);
  }
});
