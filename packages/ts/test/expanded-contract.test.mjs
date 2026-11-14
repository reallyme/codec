// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import * as wasm from "../dist/wasm/reallyme_codec_wasm.js";
import { installReallyMeCodecWasmProvider, processOperation } from "../dist/index.js";
import { CodecApiOperationRequestSchema, CodecApiOperationResponseSchema, CodecGeneralCborFloatWidth } from "../dist/proto.js";

wasm.initSync({ module: readFileSync(new URL("../dist/wasm/reallyme_codec_wasm_bg.wasm", import.meta.url)) });
installReallyMeCodecWasmProvider(wasm);

const inspectResponse = (operation, inspect) => {
  const request = toBinary(CodecApiOperationRequestSchema, create(CodecApiOperationRequestSchema, { operation }));
  const response = processOperation(request);
  try {
    inspect(fromBinary(CodecApiOperationResponseSchema, response));
  } finally {
    request.fill(0);
    response.fill(0);
  }
};

test("expanded generated operations retain scalar and IEEE bytes through WASM", () => {
  inspectResponse({ case: "base64Encode", value: { value: Uint8Array.of(0xfb, 0xff) } }, (response) => {
    assert.equal(response.outcome.case, "result");
    assert.equal(response.outcome.value.result.case, "base64Encode");
    assert.equal(response.outcome.value.result.value.value, "+/8=");
  });
  inspectResponse({ case: "generalCborEncode", value: { value: { value: { case: "floatValue", value: { width: CodecGeneralCborFloatWidth.BINARY16, bits: 0xfe01n } } } } }, (response) => {
    assert.equal(response.outcome.case, "result");
    assert.equal(response.outcome.value.result.case, "generalCborEncode");
    assert.deepEqual(response.outcome.value.result.value.value, Uint8Array.of(0xf9, 0xfe, 1));
  });
});
