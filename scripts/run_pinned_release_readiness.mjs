#!/usr/bin/env node
// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { createHash, timingSafeEqual } from "node:crypto";
import { lstatSync, readFileSync } from "node:fs";
import { spawnSync } from "node:child_process";

// This full commit, rather than a mutable branch or tag, identifies the public
// upstream core. The local checker still carries repository policy, so this
// runner verifies the fetched upstream bytes, compares them directly with the
// vendored core, and separately pins the local checker bytes before executing.
const RELEASE_READINESS_COMMIT = "f27973caf9d3a12847cac4032c361f5f553c97e9";
const RELEASE_READINESS_CORE_SHA256 =
  "70cc78721738cf352024938e8fc86e73380e71b2cdf7a9a733687543167cbaae";
const LOCAL_CHECKER_SHA256 =
  "57c8454fea4a1ceac1664be82cd6126f40160a181159c218ac1203051f028065";
const RELEASE_READINESS_CORE_URL =
  `https://raw.githubusercontent.com/reallyme/release-readiness/${RELEASE_READINESS_COMMIT}/core.mjs`;
const VENDORED_CORE_PATH = "scripts/release-readiness/core.mjs";
const LOCAL_CHECKER_PATH = "scripts/check_release_readiness.mjs";
const MAX_CORE_BYTES = 262_144;
const MAX_CHECKER_BYTES = 524_288;
const FETCH_TIMEOUT_MILLISECONDS = 30_000;

const fail = (message) => {
  console.error(`pinned release readiness failed: ${message}`);
  process.exit(1);
};

const sha256 = (value) => createHash("sha256").update(value).digest();

const expectedDigest = Buffer.from(RELEASE_READINESS_CORE_SHA256, "hex");
if (expectedDigest.length !== 32) {
  fail("configured core digest is invalid");
}
const expectedCheckerDigest = Buffer.from(LOCAL_CHECKER_SHA256, "hex");
if (expectedCheckerDigest.length !== 32) {
  fail("configured local checker digest is invalid");
}

let localCore;
let localChecker;
try {
  const checkerStatus = lstatSync(LOCAL_CHECKER_PATH);
  if (checkerStatus.isSymbolicLink() || !checkerStatus.isFile()) {
    fail("local checker must be a regular file");
  }
  if (checkerStatus.size === 0 || checkerStatus.size > MAX_CHECKER_BYTES) {
    fail("local checker size is outside the accepted boundary");
  }
  const status = lstatSync(VENDORED_CORE_PATH);
  if (status.isSymbolicLink() || !status.isFile()) {
    fail("vendored core must be a regular file");
  }
  if (status.size === 0 || status.size > MAX_CORE_BYTES) {
    fail("vendored core size is outside the accepted boundary");
  }
  localChecker = readFileSync(LOCAL_CHECKER_PATH);
  localCore = readFileSync(VENDORED_CORE_PATH);
} catch {
  fail("release readiness inputs are missing or inaccessible");
}
if (!timingSafeEqual(sha256(localChecker), expectedCheckerDigest)) {
  fail("local checker does not match the reviewed repository policy pin");
}
if (!timingSafeEqual(sha256(localCore), expectedDigest)) {
  fail("vendored core does not match the reviewed upstream pin");
}

let response;
try {
  response = await fetch(RELEASE_READINESS_CORE_URL, {
    cache: "no-store",
    redirect: "error",
    signal: AbortSignal.timeout(FETCH_TIMEOUT_MILLISECONDS),
  });
} catch {
  fail("pinned upstream core could not be fetched");
}
if (!response.ok || response.body === null) {
  fail("pinned upstream core returned an invalid response");
}

const contentLength = response.headers.get("content-length");
if (contentLength !== null) {
  if (!/^[1-9][0-9]*$/u.test(contentLength)) {
    fail("pinned upstream core returned an invalid content length");
  }
  const parsedLength = Number.parseInt(contentLength, 10);
  if (!Number.isSafeInteger(parsedLength) || parsedLength <= 0 || parsedLength > MAX_CORE_BYTES) {
    fail("pinned upstream core length is outside the accepted boundary");
  }
}

const reader = response.body.getReader();
const chunks = [];
let totalLength = 0;
while (true) {
  let result;
  try {
    result = await reader.read();
  } catch {
    fail("pinned upstream core response could not be read");
  }
  if (result.done) {
    break;
  }
  const chunk = result.value;
  if (!(chunk instanceof Uint8Array) || chunk.length > MAX_CORE_BYTES - totalLength) {
    fail("pinned upstream core exceeds the accepted boundary");
  }
  chunks.push(chunk);
  totalLength += chunk.length;
}
if (totalLength === 0) {
  fail("pinned upstream core is empty");
}
const upstreamCore = Buffer.concat(chunks, totalLength);
if (!timingSafeEqual(sha256(upstreamCore), expectedDigest)) {
  fail("pinned upstream core digest does not match the reviewed commit");
}
if (
  localCore.length !== upstreamCore.length ||
  !timingSafeEqual(localCore, upstreamCore)
) {
  fail("vendored core bytes do not match the pinned upstream core");
}

const checker = spawnSync(process.execPath, [LOCAL_CHECKER_PATH, ...process.argv.slice(2)], {
  env: process.env,
  stdio: "inherit",
});
if (checker.error !== undefined) {
  fail("local release readiness checker could not be started");
}
if (!Number.isInteger(checker.status)) {
  fail("local release readiness checker ended without a deterministic status");
}
process.exit(checker.status);
