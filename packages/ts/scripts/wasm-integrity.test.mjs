// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { createHash, randomBytes } from "node:crypto";
import test from "node:test";

import { reallymeCodecSha256Hex, wasmIntegritySource } from "./wasm-integrity.mjs";

const nodeDigest = (bytes) => createHash("sha256").update(bytes).digest("hex");

test("embedded SHA-256 matches node:crypto across block boundaries", () => {
  assert.equal(
    reallymeCodecSha256Hex(new Uint8Array(0)),
    "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
  );
  assert.equal(
    reallymeCodecSha256Hex(new TextEncoder().encode("abc")),
    "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
  );
  for (const length of [1, 55, 56, 63, 64, 65, 119, 120, 127, 128, 1000, 65_537]) {
    const bytes = randomBytes(length);
    assert.equal(reallymeCodecSha256Hex(bytes), nodeDigest(bytes), String(length));
  }
});

test("integrity glue admits only pinned bytes", () => {
  const pinned = randomBytes(96);
  const verify = new Function(`${wasmIntegritySource(nodeDigest(pinned))}
return __reallymeVerifiedWasmBytes;`)();
  const verified = verify(pinned);
  assert.deepEqual(Buffer.from(verified), pinned);
  assert.notEqual(verified.buffer, pinned.buffer);
  assert.deepEqual(Buffer.from(verify(pinned.buffer.slice(pinned.byteOffset, pinned.byteOffset + 96))), pinned);

  const tampered = Buffer.from(pinned);
  tampered[17] ^= 0x01;
  assert.throws(() => verify(tampered), /do not match/u);
  assert.throws(() => verify("not bytes"), /supplied as bytes/u);
  assert.throws(() => wasmIntegritySource("not a digest"), /64 lowercase/u);
});
