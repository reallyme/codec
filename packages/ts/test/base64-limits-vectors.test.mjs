// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { fromBinary, fromJson, toBinary } from "@bufbuild/protobuf";
import * as wasm from "../dist/wasm/reallyme_codec_wasm.js";
import { installReallyMeCodecWasmProvider, processOperation } from "../dist/index.js";
import {
  CodecApiOperationRequestSchema,
  CodecApiOperationResponseSchema,
  CodecErrorOrigin,
  CodecErrorReason,
} from "../dist/proto.js";

wasm.initSync({ module: readFileSync(new URL("../dist/wasm/reallyme_codec_wasm_bg.wasm", import.meta.url)) });
installReallyMeCodecWasmProvider(wasm);

const manifest = JSON.parse(readFileSync(new URL("../../../vectors/codec-vectors.json", import.meta.url), "utf8"));
assert.equal(manifest.schemaVersion, 2);
assert.equal(manifest.base64DecodeWithLimits.fixtureClass, "golden-operation");
assert.ok(Array.isArray(manifest.base64DecodeWithLimits.cases));
assert.ok(manifest.base64DecodeWithLimits.cases.length > 0);

for (const fixture of manifest.base64DecodeWithLimits.cases) {
  test(`shared bounded Base64 vector: ${fixture.name}`, () => {
    assert.match(fixture.requestHex, /^(?:[0-9a-f]{2})+$/u);
    assert.match(fixture.responseHex, /^(?:[0-9a-f]{2})+$/u);
    const request = Uint8Array.from(Buffer.from(fixture.requestHex, "hex"));
    const expectedBytes = Uint8Array.from(Buffer.from(fixture.responseHex, "hex"));
    const parsedRequest = fromJson(CodecApiOperationRequestSchema, fixture.request);
    assert.equal(parsedRequest.operation.case, "base64DecodeWithLimits");
    assert.deepEqual(toBinary(CodecApiOperationRequestSchema, parsedRequest), request);
    const expected = fromJson(CodecApiOperationResponseSchema, fixture.response);
    assert.deepEqual(toBinary(CodecApiOperationResponseSchema, expected), expectedBytes);
    const responseBytes = processOperation(request);
    try {
      assert.deepEqual(responseBytes, expectedBytes);
      const response = fromBinary(CodecApiOperationResponseSchema, responseBytes);
      assert.deepEqual(response, expected);
      if (response.outcome.case === "result") {
        assert.equal(response.outcome.value.result.case, "base64DecodeWithLimits");
      } else {
        assert.equal(response.outcome.case, "error");
        assert.equal(response.outcome.value.origin, CodecErrorOrigin.CALLER);
        assert.ok(["baseEncoding"].includes(response.outcome.value.error.case));
        assert.notEqual(response.outcome.value.error.value.reason, CodecErrorReason.UNSPECIFIED);
      }
    } finally {
      request.fill(0);
      responseBytes.fill(0);
      expectedBytes.fill(0);
    }
  });
}
