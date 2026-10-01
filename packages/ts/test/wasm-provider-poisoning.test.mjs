// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { test } from "node:test";
import "./wasm-module-test-hook.mjs";

const wasm = await import("../dist/wasm/reallyme_codec_wasm.js");
const {
  ReallyMeCodecError,
  installReallyMeCodecWasmProvider,
  processOperation,
  requireSupportedMulticodec,
  tryParseCid,
} = await import("../dist/index.js");

const request = Uint8Array.of(0);
let calls = 0;
wasm.setOperationHandler(() => {
  calls += 1;
  throw new WebAssembly.RuntimeError("trap");
});

const assertCode = (expected) => {
  assert.throws(
    () => processOperation(request),
    (error) => error instanceof ReallyMeCodecError && error.code === expected,
  );
};

test("installation before WASM initialization does not poison a later installation", () => {
  wasm.setInitialized(false);
  assert.throws(
    () => installReallyMeCodecWasmProvider(wasm),
    (error) => error instanceof ReallyMeCodecError && error.code === "provider-failure",
  );
  wasm.setInitialized(true);
  installReallyMeCodecWasmProvider(wasm);
});

test("a host RangeError does not poison the initialized WASM provider", () => {
  wasm.setOperationHandler(() => {
    calls += 1;
    throw new RangeError("host stack error");
  });
  assertCode("provider-failure");
  const callsAfterHostError = calls;
  wasm.setOperationHandler(() => {
    calls += 1;
    return Uint8Array.of(0);
  });
  assert.deepEqual(processOperation(request), Uint8Array.of(0));
  assert.equal(calls, callsAfterHostError + 1);
});

test("void provider responses reject unexpected values without poisoning", () => {
  wasm.setVoidHandler(() => "unexpected");
  assert.throws(
    () => requireSupportedMulticodec("dag-cbor"),
    (error) => error instanceof ReallyMeCodecError && error.code === "provider-failure",
  );
  wasm.setVoidHandler(() => undefined);
  assert.equal(requireSupportedMulticodec("dag-cbor"), undefined);
});

test("CID parsing returns undefined only for an invalid input error", () => {
  assert.equal(tryParseCid(42), undefined);
  wasm.setCidHandler(() => {
    throw new ReallyMeCodecError("provider-failure");
  });
  assert.throws(
    () => tryParseCid("invalid"),
    (error) => error instanceof ReallyMeCodecError && error.code === "provider-failure",
  );
});

test("a WASM trap poisons the installed provider", () => {
  wasm.setOperationHandler(() => {
    calls += 1;
    throw new WebAssembly.RuntimeError("trap");
  });
  const callsBeforeTrap = calls;
  assertCode("provider-failure");
  assert.equal(calls, callsBeforeTrap + 1);

  wasm.setOperationHandler(() => {
    calls += 1;
    return Uint8Array.of(0);
  });
  assertCode("provider-failure");
  assert.equal(calls, callsBeforeTrap + 1);
});
