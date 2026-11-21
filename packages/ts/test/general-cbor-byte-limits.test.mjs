// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { create, fromBinary, toBinary, toJsonString } from "@bufbuild/protobuf";
import * as wasm from "../dist/wasm/reallyme_codec_wasm.js";
import { installReallyMeCodecWasmProvider, processOperation, processOperationJson } from "../dist/index.js";
import {
  CodecApiOperationRequestSchema, CodecApiOperationResponseSchema, CodecErrorOrigin,
  CodecGeneralCborErrorReason, CodecGeneralCborSerialization, CodecGeneralCborLengthPolicy,
  CodecGeneralCborFloatPolicy, CodecGeneralCborMapOrdering, CodecGeneralCborInspection,
} from "../dist/proto.js";

wasm.initSync({ module: readFileSync(new URL("../dist/wasm/reallyme_codec_wasm_bg.wasm", import.meta.url)) });
installReallyMeCodecWasmProvider(wasm);

const DEFAULT_BYTES = 1_048_576;
const MAX_BYTES = 2_097_152;
const options = () => ({
  serialization: CodecGeneralCborSerialization.RECEIVED,
  lengths: CodecGeneralCborLengthPolicy.DEFINITE_ONLY,
  floats: CodecGeneralCborFloatPolicy.ALL,
  ordering: CodecGeneralCborMapOrdering.RECEIVED,
  inputBytes: BigInt(MAX_BYTES), outputBytes: BigInt(MAX_BYTES), byteStringBytes: BigInt(MAX_BYTES),
});

function wire(total) {
  const encoded = new Uint8Array(total).fill(0xa5);
  encoded[0] = 0x5a;
  new DataView(encoded.buffer).setUint32(1, total - 5, false);
  return encoded;
}

function execute(operation, check) {
  const request = create(CodecApiOperationRequestSchema, { operation });
  const binary = toBinary(CodecApiOperationRequestSchema, request);
  const json = new TextEncoder().encode(toJsonString(CodecApiOperationRequestSchema, request));
  const response = processOperation(binary);
  const jsonResponse = processOperationJson(json);
  try {
    assert.deepEqual(response, jsonResponse);
    check(fromBinary(CodecApiOperationResponseSchema, response));
  } finally {
    binary.fill(0);
    json.fill(0);
    response.fill(0);
    jsonResponse.fill(0);
  }
}

function failure(expected) {
  return (response) => {
    assert.equal(response.outcome.case, "error");
    assert.equal(response.outcome.value.origin, CodecErrorOrigin.CALLER);
    assert.equal(response.outcome.value.error.case, "generalCbor");
    assert.equal(response.outcome.value.error.value.detail, expected);
  };
}

function success(operation, check = () => {}) {
  return (response) => {
    assert.equal(response.outcome.case, "result");
    assert.equal(response.outcome.value.result.case, operation);
    const result = response.outcome.value.result.value;
    try { check(result); } finally {
      if (result.value instanceof Uint8Array) result.value.fill(0);
    }
  };
}

test("general-CBOR input budgets include headers and retain the one-MiB default through WASM", () => {
  for (const size of [DEFAULT_BYTES, DEFAULT_BYTES + 1, MAX_BYTES]) {
    const encoded = wire(size);
    try {
      execute({ case: "generalCborValidateExact", value: { encoded } }, size > DEFAULT_BYTES
        ? failure(CodecGeneralCborErrorReason.INPUT_LIMIT)
        : success("generalCborValidateExact"));
      for (const operation of ["generalCborFrameExact", "generalCborFramePrefix", "generalCborValidateExact", "generalCborValidatePrefix"]) {
        execute({ case: operation, value: { encoded, options: { ...options(), inputBytes: BigInt(size) } } }, success(operation, (frame) => {
          assert.equal(frame.received.offset, 0n);
          assert.equal(frame.received.length, BigInt(size));
          assert.equal(frame.options.inputBytes, BigInt(size));
        }));
      }
      execute({ case: "generalCborValidateExact", value: { encoded, options: { ...options(), inputBytes: BigInt(size - 1) } } }, failure(CodecGeneralCborErrorReason.INPUT_LIMIT));
      execute({ case: "generalCborValidateExact", value: { encoded, options: { ...options(), byteStringBytes: BigInt(size - 6) } } }, failure(CodecGeneralCborErrorReason.BYTE_STRING_LIMIT));
    } finally { encoded.fill(0); }
  }
  const encoded = wire(MAX_BYTES + 1);
  try {
    execute({ case: "generalCborFrameExact", value: { encoded, options: options() } }, failure(CodecGeneralCborErrorReason.INPUT_LIMIT));
  } finally { encoded.fill(0); }
});

test("general-CBOR output and aggregate budgets are independently enforced through WASM", () => {
  const encoded = wire(MAX_BYTES);
  const value = { value: { case: "bytesValue", value: { value: encoded.subarray(5) } } };
  try {
    execute({ case: "generalCborEncode", value: { value, options: { ...options(), inputBytes: 0n } } }, success("generalCborEncode", (result) => assert.deepEqual(result.value, encoded)));
    execute({ case: "generalCborEncode", value: { value, options: { ...options(), outputBytes: BigInt(MAX_BYTES - 1) } } }, failure(CodecGeneralCborErrorReason.OUTPUT_LIMIT));
    execute({ case: "generalCborEncode", value: { value, options: { ...options(), byteStringBytes: BigInt(encoded.length - 6) } } }, failure(CodecGeneralCborErrorReason.BYTE_STRING_LIMIT));
    execute({ case: "generalCborEncode", value: { value } }, failure(CodecGeneralCborErrorReason.BYTE_STRING_LIMIT));
    const received = { value: { case: "receivedValue", value: { value: encoded } } };
    execute({ case: "generalCborEncode", value: { value: received, options: { ...options(), inputBytes: BigInt(DEFAULT_BYTES) } } }, failure(CodecGeneralCborErrorReason.INPUT_LIMIT));
  } finally { encoded.fill(0); }
});

test("general-CBOR cursor and child receipts retain raised budgets through WASM", () => {
  const encoded = wire(DEFAULT_BYTES + 5);
  const parent = new Uint8Array(encoded.length + 1);
  parent[0] = 0x81;
  parent.set(encoded, 1);
  try {
    execute({ case: "generalCborCursor", value: { encoded, options: options() } }, success("generalCborCursor", (result) => {
      assert.equal(result.values.length, 1);
      assert.equal(result.values[0].received.length, BigInt(encoded.length));
      assert.equal(result.options.inputBytes, BigInt(MAX_BYTES));
    }));
    execute({ case: "generalCborInspect", value: { encoded: parent, options: options(), validate: true, inspection: CodecGeneralCborInspection.CHILDREN } }, success("generalCborInspect", (result) => {
      assert.equal(result.value.case, "children");
      assert.equal(result.value.value.values[0].received.offset, 1n);
      assert.equal(result.value.value.options.byteStringBytes, BigInt(MAX_BYTES));
    }));
  } finally { encoded.fill(0); parent.fill(0); }
});

test("general-CBOR ceiling-plus-one and uint64 overflow-sized limits fail through WASM", () => {
  for (const value of [BigInt(MAX_BYTES + 1), 18_446_744_073_709_551_615n]) {
    for (const field of ["inputBytes", "outputBytes", "byteStringBytes"]) {
      execute({ case: "generalCborFrameExact", value: { encoded: Uint8Array.of(0), options: { ...options(), [field]: value } } }, failure(CodecGeneralCborErrorReason.INVALID_LIMIT));
    }
  }
});

test("general-CBOR aggregate and embedded budgets remain separate through WASM", () => {
  const child = wire(DEFAULT_BYTES / 2 + 5);
  const array = new Uint8Array(child.length * 2 + 1);
  array[0] = 0x82;
  array.set(child, 1);
  array.set(child, child.length + 1);
  const inner = wire(DEFAULT_BYTES + 5);
  const outer = new Uint8Array(inner.length + 7);
  outer.set([0xd8, 0x18, 0x5a]);
  new DataView(outer.buffer).setUint32(3, inner.length, false);
  outer.set(inner, 7);
  try {
    execute({ case: "generalCborValidateExact", value: { encoded: array, options: { ...options(), byteStringBytes: BigInt(DEFAULT_BYTES) } } }, success("generalCborValidateExact"));
    execute({ case: "generalCborValidateExact", value: { encoded: array, options: { ...options(), byteStringBytes: BigInt(DEFAULT_BYTES - 1) } } }, failure(CodecGeneralCborErrorReason.BYTE_STRING_LIMIT));
    execute({ case: "generalCborValidateEmbedded", value: { encoded: outer, outerOptions: options() } }, failure(CodecGeneralCborErrorReason.INPUT_LIMIT));
    execute({ case: "generalCborValidateEmbedded", value: { encoded: outer, outerOptions: options(), embeddedOptions: options() } }, success("generalCborValidateEmbedded", (frame) => {
      assert.equal(frame.received.offset, 7n);
      assert.equal(frame.received.length, BigInt(inner.length));
    }));
  } finally { child.fill(0); array.fill(0); inner.fill(0); outer.fill(0); }
});
