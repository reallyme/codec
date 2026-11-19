// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import * as wasm from "../dist/wasm/reallyme_codec_wasm.js";
import { installReallyMeCodecWasmProvider, processOperation } from "../dist/index.js";
import { CodecApiOperationRequestSchema, CodecApiOperationResponseSchema } from "../dist/proto.js";

wasm.initSync({ module: readFileSync(new URL("../dist/wasm/reallyme_codec_wasm_bg.wasm", import.meta.url)) });
installReallyMeCodecWasmProvider(wasm);

test("CBOR cursor shares options and standalone frames retain options through WASM", () => {
  for (const operation of ["generalCborCursor", "generalCborFrameExact"]) {
    const encoded = operation === "generalCborCursor" ? Uint8Array.of(0, 1) : Uint8Array.of(0);
    const request = toBinary(CodecApiOperationRequestSchema, create(CodecApiOperationRequestSchema, {
      operation: { case: operation, value: { encoded } },
    }));
    const response = processOperation(request);
    try {
      const decoded = fromBinary(CodecApiOperationResponseSchema, response);
      assert.equal(decoded.outcome.case, "result");
      const result = decoded.outcome.value.result;
      assert.equal(result.case, operation);
      assert.equal(result.value.options.entries, 4096n);
      assert.equal(result.value.options.nodes, 65536n);
      if (operation === "generalCborCursor") {
        assert.equal(result.value.values.length, 2);
        assert.equal(result.value.consumedBytes, 2n);
        assert.ok(result.value.values.every((frame) => frame.options === undefined));
      }
    } finally {
      encoded.fill(0);
      request.fill(0);
      response.fill(0);
    }
  }
});
