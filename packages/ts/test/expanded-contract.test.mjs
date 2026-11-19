// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import * as wasm from "../dist/wasm/reallyme_codec_wasm.js";
import { installReallyMeCodecWasmProvider, processOperation } from "../dist/index.js";
import { CodecApiOperationRequestSchema, CodecApiOperationResponseSchema, CodecGeneralCborFloatWidth, CodecErrorReason, CodecPemDecodeScope } from "../dist/proto.js";

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

test("generated PEM bundle operation preserves bytes and typed bounds through WASM", () => {
  const block = "-----BEGIN CERTIFICATE-----\rAQID\r-----END CERTIFICATE-----\r";
  const pem = new TextEncoder().encode(block + block);
  try {
    inspectResponse({ case: "pemBundleDecode", value: { pem } }, (response) => {
      assert.equal(response.outcome.case, "result");
      assert.equal(response.outcome.value.result.case, "pemBundleDecode");
      const blocks = response.outcome.value.result.value.blocks;
      assert.equal(blocks.length, 2);
      assert.equal(blocks[0].label, "CERTIFICATE");
      assert.deepEqual(blocks[0].der, Uint8Array.of(1, 2, 3));
    });
    inspectResponse({ case: "pemBundleDecode", value: { pem, options: { scope: CodecPemDecodeScope.FIRST } } }, (response) => {
      assert.equal(response.outcome.case, "result");
      assert.equal(response.outcome.value.result.value.blocks.length, 1);
    });
    inspectResponse({ case: "pemBundleDecode", value: { pem, options: { maxBlocks: 1 } } }, (response) => {
      assert.equal(response.outcome.case, "error");
      assert.equal(response.outcome.value.error.case, "pem");
      assert.equal(response.outcome.value.error.value.reason, CodecErrorReason.PEM_RESOURCE_LIMIT_EXCEEDED);
    });
  } finally { pem.fill(0); }
});


test("guarded JSON operation preserves escaped Unicode and typed duplicate rejection through WASM", () => {
  const encoded = new TextEncoder().encode('{"hello":"world\\u0021"}');
  try {
    inspectResponse({ case: "jsonDecode", value: { encoded } }, (response) => {
      assert.equal(response.outcome.case, "result");
      assert.equal(response.outcome.value.result.case, "jsonDecode");
      const kind = response.outcome.value.result.value.value.kind;
      assert.equal(kind.case, "objectValue");
      assert.equal(kind.value.members[0].name, "hello");
      assert.equal(kind.value.members[0].value.kind.case, "stringValue");
      assert.equal(kind.value.members[0].value.kind.value, "world!");
    });
  } finally { encoded.fill(0); }
  const duplicate = new TextEncoder().encode('{"x":1,"\\u0078":2}');
  try {
    inspectResponse({ case: "jsonDecode", value: { encoded: duplicate } }, (response) => {
      assert.equal(response.outcome.case, "error");
      assert.equal(response.outcome.value.error.case, "canonicalization");
      assert.equal(response.outcome.value.error.value.reason, CodecErrorReason.CANONICAL_JSON_DUPLICATE_PROPERTY);
    });
  } finally { duplicate.fill(0); }
});
