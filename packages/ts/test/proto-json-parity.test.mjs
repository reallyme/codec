// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { fromBinary, fromJson, toBinary } from "@bufbuild/protobuf";
import * as wasm from "../dist/wasm/reallyme_codec_wasm.js";
import { installReallyMeCodecWasmProvider, processOperation, processOperationJson } from "../dist/index.js";
import { CodecApiOperationRequestSchema, CodecApiOperationResponseSchema, CodecErrorReason, CodecErrorOrigin } from "../dist/proto.js";

wasm.initSync({ module: readFileSync(new URL("../dist/wasm/reallyme_codec_wasm_bg.wasm", import.meta.url)) });
installReallyMeCodecWasmProvider(wasm);

test("ProtoJSON null and quoted integer forms match protobuf through WASM", () => {
  for (const [value, expected] of [
    [{ arrayValue: { values: null } }, 0x80],
    [{ boolValue: { value: null } }, 0xf4],
    [{ simpleValue: { value: "16" } }, 0xf0],
  ]) {
    const json = { generalCborEncode: { value } };
    const binaryRequest = toBinary(CodecApiOperationRequestSchema, fromJson(CodecApiOperationRequestSchema, json));
    const jsonRequest = new TextEncoder().encode(JSON.stringify(json));
    const binaryResponse = processOperation(binaryRequest);
    const jsonResponse = processOperationJson(jsonRequest);
    try {
      assert.deepEqual(jsonResponse, binaryResponse);
      const response = fromBinary(CodecApiOperationResponseSchema, jsonResponse);
      assert.equal(response.outcome.case, "result");
      assert.equal(response.outcome.value.result.case, "generalCborEncode");
      assert.deepEqual(response.outcome.value.result.value.value, Uint8Array.of(expected));
    } finally {
      binaryRequest.fill(0);
      jsonRequest.fill(0);
      binaryResponse.fill(0);
      jsonResponse.fill(0);
    }
  }
});

test("ProtoJSON invalid list elements and overflowing integers remain typed errors", () => {
  for (const value of [{ arrayValue: { values: [null] } }, { simpleValue: { value: "4294967296" } }]) {
    const request = new TextEncoder().encode(JSON.stringify({ generalCborEncode: { value } }));
    const bytes = processOperationJson(request);
    try {
      const response = fromBinary(CodecApiOperationResponseSchema, bytes);
      assert.equal(response.outcome.case, "error");
      assert.equal(response.outcome.value.origin, CodecErrorOrigin.CALLER);
      assert.equal(response.outcome.value.error.case, "boundary");
      assert.equal(response.outcome.value.error.value.reason, CodecErrorReason.BOUNDARY_MALFORMED_JSON);
    } finally {
      request.fill(0);
      bytes.fill(0);
    }
  }
});
