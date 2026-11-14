// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { create, fromBinary, toBinary, toJsonString } from "@bufbuild/protobuf";
import * as wasm from "../dist/wasm/reallyme_codec_wasm.js";
import { installReallyMeCodecWasmProvider, processOperation, processOperationJson } from "../dist/index.js";
import { CodecApiOperationRequestSchema, CodecApiOperationResponseSchema, CodecTlvErrorReason, CodecTlvProfile } from "../dist/proto.js";

wasm.initSync({ module: readFileSync(new URL("../dist/wasm/reallyme_codec_wasm_bg.wasm", import.meta.url)) });
installReallyMeCodecWasmProvider(wasm);

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
