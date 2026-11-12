#!/usr/bin/env node
// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { createHash, timingSafeEqual } from "node:crypto";
import { closeSync, constants, fstatSync, openSync, readSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

// This full commit, rather than a mutable branch or tag, identifies the public
// upstream core. The local checker still carries repository policy, so this
// runner checks both local digests before executing. Re-downloading immutable
// bytes adds availability risk without strengthening that digest check, so
// upstream provenance verification is an explicit maintenance operation.
// The checker pins every local module it imports or spawns, so those bytes
// are covered transitively by the checker pin. This is a change-detection
// tripwire; review of the full change set remains necessary before treating
// the complete policy implementation as trusted.
const RELEASE_READINESS_COMMIT = "18a6d91d99837e80f92e41fe04effb0b043d754e";
const RELEASE_READINESS_CORE_SHA256 =
  "7614b0cbf9e36a4653c2fa9008bee0d1e22542de0e4a08a10e640c836a5c93db";
const LOCAL_CHECKER_SHA256 =
  "02f8470a7e211c873089c27b91c60a2a7b580501ff8ab6cae3d8f13831e4a7b2";
const RELEASE_READINESS_CORE_URL =
  `https://raw.githubusercontent.com/reallyme/release-readiness/${RELEASE_READINESS_COMMIT}/core.mjs`;
const VENDORED_CORE_PATH = "scripts/release-readiness/core.mjs";
const LOCAL_CHECKER_PATH = "scripts/check_release_readiness.mjs";
const MAX_CORE_BYTES = 262_144;
const MAX_CHECKER_BYTES = 524_288;
const FETCH_TIMEOUT_MILLISECONDS = 30_000;
const repositoryUrl = new URL("../", import.meta.url);
const repositoryRoot = fileURLToPath(repositoryUrl);

const fail = (message) => {
  console.error(`pinned release readiness failed: ${message}`);
  process.exit(1);
};

const sha256 = (value) => createHash("sha256").update(value).digest();

const readBoundedRegularFile = (path, maximumBytes, label) => {
  if (!Number.isInteger(constants.O_NOFOLLOW)) {
    fail("this platform cannot enforce no-follow file reads");
  }
  let descriptor;
  try {
    // Nonblocking opens let us reject special files without waiting on a FIFO.
    descriptor = openSync(new URL(path, repositoryUrl),
      constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK);
    const status = fstatSync(descriptor);
    if (!status.isFile()) {
      fail(`${label} must be a regular file`);
    }
    if (!Number.isSafeInteger(status.size) || status.size <= 0 || status.size > maximumBytes) {
      fail(`${label} size is outside the accepted boundary`);
    }
    // Keep the validated descriptor and one extra byte to detect growth. A
    // replaced path cannot redirect this read or cause an unbounded allocation.
    const buffer = Buffer.alloc(status.size + 1);
    let totalLength = 0;
    while (totalLength < buffer.length) {
      const bytesRead = readSync(descriptor, buffer, totalLength, buffer.length - totalLength, null);
      if (bytesRead === 0) {
        break;
      }
      totalLength += bytesRead;
      if (totalLength > status.size) {
        fail(`${label} size is outside the accepted boundary`);
      }
    }
    if (totalLength !== status.size) {
      fail(`${label} size is outside the accepted boundary`);
    }
    return buffer.subarray(0, totalLength);
  } finally {
    if (descriptor !== undefined) {
      closeSync(descriptor);
    }
  }
};

const supportedArguments = new Set([
  "--check-pins",
  "--refresh-checker-pin",
  "--verify-upstream",
  "--generated-freshness",
  "--release-packages",
]);
const arguments_ = new Set();
for (const argument of process.argv.slice(2)) {
  if (!supportedArguments.has(argument) || arguments_.has(argument)) {
    fail("unsupported or duplicate argument");
  }
  arguments_.add(argument);
}
const refreshCheckerPin = arguments_.has("--refresh-checker-pin");
if (refreshCheckerPin && arguments_.size !== 1) {
  fail("checker pin refresh cannot be combined with checks");
}
if (
  arguments_.has("--check-pins") &&
  (arguments_.has("--generated-freshness") || arguments_.has("--release-packages"))
) {
  fail("pin-only verification cannot be combined with policy modes");
}
// CI must detect stale policy pins, never silently approve changed policy.
if (refreshCheckerPin && process.env.CI !== undefined) {
  fail("checker pin refresh is a local review operation and is forbidden in CI");
}

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
  localChecker = readBoundedRegularFile(LOCAL_CHECKER_PATH, MAX_CHECKER_BYTES, "local checker");
  localCore = readBoundedRegularFile(VENDORED_CORE_PATH, MAX_CORE_BYTES, "vendored core");
} catch {
  fail("release readiness inputs are missing or inaccessible");
}
if (!timingSafeEqual(sha256(localCore), expectedDigest)) {
  fail("vendored core does not match the reviewed upstream pin");
}

if (refreshCheckerPin) {
  try {
    const source = readBoundedRegularFile(import.meta.url, MAX_CHECKER_BYTES, "runner").toString("utf8");
    const declaration = /const LOCAL_CHECKER_SHA256 =\n  "[0-9a-f]{64}";/gu;
    if ([...source.matchAll(declaration)].length !== 1) {
      fail("local checker pin declaration is missing or ambiguous");
    }
    const updated = source.replace(
      declaration,
      `const LOCAL_CHECKER_SHA256 =\n  "${sha256(localChecker).toString("hex")}";`,
    );
    if (updated !== source) {
      // The replacement has the same fixed width; do not truncate the runner
      // or follow a symlink substituted after its contents were validated.
      writeFileSync(new URL(import.meta.url), updated, {
        flag: constants.O_WRONLY | constants.O_NOFOLLOW,
      });
    }
  } catch {
    fail("local checker pin could not be refreshed");
  }
  console.log("local checker pin refreshed; review the checker and pin diff together");
  process.exit(0);
}

if (!timingSafeEqual(sha256(localChecker), expectedCheckerDigest)) {
  fail("local checker does not match the reviewed repository policy pin; after reviewing the policy change, run node scripts/run_pinned_release_readiness.mjs --refresh-checker-pin");
}

const verifyUpstream = async () => {
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
};

if (arguments_.has("--verify-upstream")) {
  await verifyUpstream();
}

if (arguments_.has("--check-pins")) {
  console.log("release readiness pins verified");
  process.exit(0);
}

const checkerArguments = [...arguments_].filter((argument) =>
  argument === "--generated-freshness" || argument === "--release-packages",
);
const checker = spawnSync(process.execPath, [LOCAL_CHECKER_PATH, ...checkerArguments], {
  cwd: repositoryRoot,
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
