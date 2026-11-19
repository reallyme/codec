#!/usr/bin/env node
// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { createHash, timingSafeEqual } from "node:crypto";
import { closeSync, constants, fstatSync, openSync, readSync, realpathSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath, pathToFileURL } from "node:url";
import { resolve, sep } from "node:path";

// Verify the immutable official package before executing its runner. Local
// checker/module digests remain review tripwires; the package runner supplies
// the core and enforces proof that every applicable source policy was invoked.
const RELEASE_READINESS_COMMIT = "e323ea52bc1f76e67df03fc95e19d596fb54bd03";
const RELEASE_READINESS_CORE_SHA256 =
  "825ebca65a1324ea7cf42713ca5c4c67b0d929444d24dcaf7500fc9fa0ca7e20";
const LOCAL_CHECKER_SHA256 =
  "def0bc8edb898cb1fe18aa9c6dd5801d4ecce618a72623c391b980882600c1f9";
const RELEASE_READINESS_CORE_URL =
  `https://raw.githubusercontent.com/reallyme/release-readiness/${RELEASE_READINESS_COMMIT}/core.mjs`;
const RELEASE_READINESS_RUNNER_SHA256 =
  "3486696ab93190b627438a6aa7248e972872c83b4efc62286aecb1f60ee3783c";
const RELEASE_READINESS_PACKAGE_SHA256 =
  "c5de23339bcd17107f63a2b467c1c0441a2695f0c12a786ced2b491d7c7a425f";
const RELEASE_READINESS_SUPPORT_FILES = [
  ["scripts/publication-paths.py", "45d3485bc03f4a73f955942648b13e5fba8f06d450326eb13c7b0df2ad518caf"],
  ["scripts/repository-privacy.mjs", "6c8f44bf810e947fcbe3e0a9e45cdaac007009a5fe719f0ca928bc5bbd04c346"],
  ["scripts/public-ci.mjs", "9b64390152f9b440f2045a6f2bd91965ce7c09c12b5b0eb41e20298bbab22686"],
  ["scripts/public-release.mjs", "187ac87a288d14bdfaec38a2e22a547dcda8fa7df8f3a8701063306ea2bd78e0"],
  ["scripts/public-repo-policy.mjs", "fdad8fe9c0e85043b764a0321558af7a85324bceb7724c693a2824d955b0a204"],
  ["scripts/android-runtime.mjs", "5e80d5dee4a1ac225dfe155dd6d5477e1c52bc18be103f7b6fef181a0d4cb3c9"],
  ["scripts/android-publication.mjs", "539660a91b7b9150a9d85cafff35c5bbefd97a57541d2343f55f1d1243e3be7b"],
  ["templates/cla.yaml", "3c5e89c5753d128643e5862e6baeb6b97c4f0ce1d6ef158cf46db3deeec6b955"],
];
const LOCAL_CHECKER_PATH = "scripts/check_release_readiness.mjs";
const MAX_CORE_BYTES = 262_144;
const MAX_CHECKER_BYTES = 524_288;
const FETCH_TIMEOUT_MILLISECONDS = 30_000;
const repositoryUrl = new URL("../", import.meta.url);
const repositoryRoot = fileURLToPath(repositoryUrl);
const packageUrl = process.env.REALLYME_RELEASE_READINESS_ROOT === undefined
  ? new URL("../.release-readiness/", import.meta.url)
  : pathToFileURL(`${resolve(process.env.REALLYME_RELEASE_READINESS_ROOT)}${sep}`);
const packageCoreUrl = new URL("core.mjs", packageUrl);
const officialRunnerUrl = new URL("scripts/run-consumer-check.mjs", packageUrl);

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
  "--print-commit",
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
if (arguments_.has("--print-commit")) {
  if (arguments_.size !== 1) {
    fail("printing the package commit cannot be combined with checks");
  }
  console.log(RELEASE_READINESS_COMMIT);
  process.exit(0);
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
  localCore = readBoundedRegularFile(packageCoreUrl, MAX_CORE_BYTES, "package core");
  for (const [url, maximum, label, digest] of [
    [officialRunnerUrl, MAX_CORE_BYTES, "package runner", RELEASE_READINESS_RUNNER_SHA256],
    [new URL("package.json", packageUrl), MAX_CORE_BYTES, "package manifest", RELEASE_READINESS_PACKAGE_SHA256],
    ...RELEASE_READINESS_SUPPORT_FILES.map(([path, digest]) => [
      new URL(path, packageUrl), MAX_CORE_BYTES, `package ${path}`, digest,
    ]),
  ]) {
    if (!timingSafeEqual(sha256(readBoundedRegularFile(url, maximum, label)), Buffer.from(digest, "hex"))) {
      fail(`${label} does not match the reviewed upstream pin`);
    }
  }
} catch {
  fail("release readiness inputs are missing or inaccessible");
}
if (!timingSafeEqual(sha256(localCore), expectedDigest)) {
  fail("package core does not match the reviewed upstream pin");
}

// The official runner supplies the canonical core URL. Never import a
// different module just because its path was provided in the environment.
const suppliedCoreUrl = process.env.RELEASE_READINESS_CORE_URL;
if (suppliedCoreUrl !== undefined) {
  const expectedCoreUrl = pathToFileURL(realpathSync(packageCoreUrl)).href;
  if (suppliedCoreUrl !== expectedCoreUrl) {
    fail("supplied core URL does not identify the authenticated package core");
  }
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
    fail("package core bytes do not match the pinned upstream core");
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
const checker = spawnSync(process.execPath, [fileURLToPath(officialRunnerUrl), ...checkerArguments], {
  cwd: repositoryRoot,
  env: {
    ...process.env,
    RELEASE_READINESS_PUBLIC_REPO: process.env.RELEASE_READINESS_PUBLIC_REPO ?? "true",
  },
  stdio: "inherit",
});
if (checker.error !== undefined) {
  fail("official release readiness runner could not be started");
}
if (!Number.isInteger(checker.status)) {
  fail("local release readiness checker ended without a deterministic status");
}
process.exit(checker.status);
