// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { test } from "node:test";
import { create, fromBinary, toBinary } from "@bufbuild/protobuf";
import "./wasm-module-test-hook.mjs";
const wasm = await import("../dist/wasm/reallyme_codec_wasm.js");
const {
  ReallyMeCodecError,
  installReallyMeCodecWasmProvider,
  multicodecLookupPrefix,
  multicodecPrefixForName,
  multicodecTable,
  processOperation,
  processOperationJson,
} = await import("../dist/index.js");
import {
  MAX_CODEC_PROTO_JSON_BYTES,
  MAX_CODEC_PROTO_MESSAGE_BYTES,
} from "../dist/boundary.js";
import {
  CodecKeyMaterialKind,
  CodecMulticodecLookupResultSchema,
  CodecMulticodecSpecSchema,
  CodecMulticodecTableResultSchema,
  CodecOperationResponseSchema,
  CodecOperationResultSchema,
  CodecErrorReason,
  CodecErrorOrigin,
  CodecTag,
} from "../dist/proto.js";

const MAX_MULTICODEC_TABLE_ENTRIES = 1_024;

let currentResult = create(CodecOperationResultSchema);
let lastProviderInput = new Uint8Array(0);
let lastProviderOutput = new Uint8Array(0);

const operationHandler = (request) => {
    lastProviderInput = request;
    lastProviderOutput = toBinary(
      CodecOperationResponseSchema,
      create(CodecOperationResponseSchema, {
        outcome: {
          case: "result",
          value: currentResult,
        },
      }),
    );
    return lastProviderOutput;
};

wasm.setOperationHandler(operationHandler);
installReallyMeCodecWasmProvider(wasm);

const metadata = (prefix = Uint8Array.of(0xed, 0x01)) =>
  create(CodecMulticodecSpecSchema, {
    name: "ed25519-pub",
    algorithmName: "Ed25519",
    tag: CodecTag.KEY,
    keyMaterialKind: CodecKeyMaterialKind.PUBLIC_KEY,
    prefix,
    fixedLength: 32,
  });

const operationResult = (result) =>
  create(CodecOperationResultSchema, { result });

const assertProviderFailure = (operation) => {
  assert.throws(
    operation,
    (error) =>
      error instanceof ReallyMeCodecError && error.code === "provider-failure",
  );
  assert.ok(lastProviderOutput.length > 0);
  assert.ok(lastProviderOutput.every((byte) => byte === 0));
};

test("public contract processors pass providers a wiped SDK-owned snapshot", () => {
  const callerBytes = Uint8Array.of(0xff);
  const responseBytes = processOperation(callerBytes);
  assert.notEqual(lastProviderInput.buffer, callerBytes.buffer);
  assert.deepEqual(callerBytes, Uint8Array.of(0xff));
  assert.ok(lastProviderInput.every((byte) => byte === 0));
  responseBytes.fill(0);

  const callerJson = new TextEncoder().encode("{}");
  const jsonResponseBytes = processOperationJson(callerJson);
  assert.notEqual(lastProviderInput.buffer, callerJson.buffer);
  assert.deepEqual(callerJson, new TextEncoder().encode("{}"));
  assert.ok(lastProviderInput.every((byte) => byte === 0));
  jsonResponseBytes.fill(0);
});

test("public contract processors reject oversized input before provider invocation", () => {
  lastProviderInput = Uint8Array.of(0xa5);
  for (const excess of [1, 16]) {
    const binary = fromBinary(
      CodecOperationResponseSchema,
      processOperation(new Uint8Array(MAX_CODEC_PROTO_MESSAGE_BYTES + excess)),
    );
    assert.equal(binary.outcome.case, "error");
    assert.equal(binary.outcome.value.error.case, "boundary");
    assert.equal(binary.outcome.value.error.value.reason, CodecErrorReason.BOUNDARY_RESOURCE_LIMIT_EXCEEDED);
    assert.equal(binary.outcome.value.origin, CodecErrorOrigin.CALLER);
    assert.deepEqual(lastProviderInput, Uint8Array.of(0xa5));

    const json = fromBinary(
      CodecOperationResponseSchema,
      processOperationJson(new Uint8Array(MAX_CODEC_PROTO_JSON_BYTES + excess)),
    );
    assert.deepEqual(json, binary);
    assert.deepEqual(lastProviderInput, Uint8Array.of(0xa5));
  }
});

test("public contract processors pass exact-cap requests to providers", () => {
  const binary = processOperation(new Uint8Array(MAX_CODEC_PROTO_MESSAGE_BYTES));
  assert.equal(lastProviderInput.length, MAX_CODEC_PROTO_MESSAGE_BYTES);
  assert.ok(lastProviderInput.every((byte) => byte === 0));
  binary.fill(0);

  const json = processOperationJson(new Uint8Array(MAX_CODEC_PROTO_JSON_BYTES));
  assert.equal(lastProviderInput.length, MAX_CODEC_PROTO_JSON_BYTES);
  assert.ok(lastProviderInput.every((byte) => byte === 0));
  json.fill(0);
});

test("structured SDK methods reject a valid result for the wrong operation", () => {
  currentResult = operationResult({
    case: "multicodecTable",
    value: create(CodecMulticodecTableResultSchema),
  });
  assertProviderFailure(() => multicodecPrefixForName("ed25519-pub"));
});

test("structured SDK lookup rejects unknown provider fields and clears output", () => {
  const lookup = create(CodecMulticodecLookupResultSchema, {
    name: "ed25519-pub",
    prefixLength: 2,
    metadata: metadata(),
  });
  lookup.$unknown = [{ data: Uint8Array.of(0x01), no: 99, wireType: 0 }];
  currentResult = operationResult({
    case: "multicodecLookupPrefix",
    value: lookup,
  });
  assertProviderFailure(() => multicodecLookupPrefix(Uint8Array.of(0xed, 0x01)));
});

test("structured SDK table rejects excess entries and clears output", () => {
  const entries = Array.from(
    { length: MAX_MULTICODEC_TABLE_ENTRIES + 1 },
    () => metadata(Uint8Array.of(0xed, 0x01)),
  );
  currentResult = operationResult({
    case: "multicodecTable",
    value: create(CodecMulticodecTableResultSchema, { entries }),
  });
  try {
    assertProviderFailure(multicodecTable);
  } finally {
    for (const entry of entries) {
      entry.prefix.fill(0);
    }
  }
});
