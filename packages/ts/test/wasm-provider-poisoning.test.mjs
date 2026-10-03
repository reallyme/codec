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

test("a RangeError escaping a WASM call replaces the instance", () => {
  const recoveriesBefore = wasm.recoveryCount();
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
  assert.equal(wasm.recoveryCount(), recoveriesBefore + 1);
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

test("a WASM trap is recovered before the next call", () => {
  wasm.setOperationHandler(() => Uint8Array.of(0));
  assert.deepEqual(processOperation(request), Uint8Array.of(0));
  const recoveriesBefore = wasm.recoveryCount();
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
  assert.deepEqual(processOperation(request), Uint8Array.of(0));
  assert.equal(calls, callsBeforeTrap + 2);
  assert.equal(wasm.recoveryCount(), recoveriesBefore + 1);
});

test("failed recovery stays closed and can retry when an instance is available", () => {
  wasm.setOperationHandler(() => {
    throw new RangeError("stack overflow");
  });
  assertCode("provider-failure");
  const recoveriesBefore = wasm.recoveryCount();
  wasm.setInitialized(false);
  assertCode("provider-failure");
  assert.equal(wasm.recoveryCount(), recoveriesBefore);
  wasm.setInitialized(true);
  wasm.setOperationHandler(() => Uint8Array.of(0));
  assert.deepEqual(processOperation(request), Uint8Array.of(0));
  assert.equal(wasm.recoveryCount(), recoveriesBefore + 1);
});
