// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { fromBinary, fromJsonString, toBinary, toJson } from "@bufbuild/protobuf";
import * as wasm from "../dist/wasm/reallyme_codec_wasm.js";
import { installReallyMeCodecWasmProvider, processOperation, processOperationJson } from "../dist/index.js";
import { CodecApiOperationRequestSchema, CodecApiOperationResponseSchema, CodecErrorOrigin, CodecErrorReason } from "../dist/proto.js";

const manifest = JSON.parse(readFileSync(new URL("../../../vectors/codec-operation-vectors.json", import.meta.url), "utf8"));
wasm.initSync({ module: readFileSync(new URL("../dist/wasm/reallyme_codec_wasm_bg.wasm", import.meta.url)) });
installReallyMeCodecWasmProvider(wasm);

test("every schema operation preserves binary and ProtoJSON responses through WASM", () => {
  assert.equal(manifest.schemaVersion, 1);
  assert.deepEqual(manifest.cases.map((entry) => entry.fieldNumber).sort((a, b) => a - b), CodecApiOperationRequestSchema.fields.map((field) => field.number).sort((a, b) => a - b));
  for (const entry of manifest.cases) {
    const request = Uint8Array.from(Buffer.from(entry.requestHex, "hex"));
    assert.deepEqual(toBinary(CodecApiOperationRequestSchema, fromJsonString(CodecApiOperationRequestSchema, entry.requestJson)), request);
    const json = new TextEncoder().encode(entry.requestJson);
    const binaryResponse = processOperation(request);
    const jsonResponse = processOperationJson(json);
    try {
      assert.deepEqual(jsonResponse, binaryResponse, entry.operation);
      const decoded = fromBinary(CodecApiOperationResponseSchema, binaryResponse);
      assert.equal(decoded.outcome.case, "result", entry.operation);
      assert.equal(decoded.outcome.value.result.case, entry.operation);
      const result = toJson(CodecApiOperationResponseSchema, decoded).result[entry.operation];
      for (const [pointer, expected] of Object.entries(entry.checks)) {
        let actual = result;
        for (const field of pointer.split("/").slice(1)) actual = actual[field];
        assert.deepEqual(actual, expected, `${entry.operation} ${pointer}`);
      }
      const invalid = JSON.parse(entry.requestJson);
      invalid[entry.operation].unexpected = true;
      const invalidRequest = new TextEncoder().encode(JSON.stringify(invalid));
      const invalidResponse = processOperationJson(invalidRequest);
      try {
        const response = fromBinary(CodecApiOperationResponseSchema, invalidResponse);
        assert.equal(response.outcome.case, "error");
        assert.equal(response.outcome.value.origin, CodecErrorOrigin.CALLER);
        assert.equal(response.outcome.value.error.case, "boundary");
        assert.equal(response.outcome.value.error.value.reason, CodecErrorReason.BOUNDARY_MALFORMED_JSON);
      } finally {
        invalidRequest.fill(0);
        invalidResponse.fill(0);
      }
    } finally {
      request.fill(0);
      json.fill(0);
      binaryResponse.fill(0);
      jsonResponse.fill(0);
    }
  }
});
