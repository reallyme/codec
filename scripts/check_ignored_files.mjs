#!/usr/bin/env node
// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { spawnSync } from "node:child_process";
import { readSync } from "node:fs";
import { fileURLToPath } from "node:url";

const packageRoot = fileURLToPath(new URL("../", import.meta.url));
const maxGitOutputBytes = 16 * 1024 * 1024;
const inputChunkBytes = 64 * 1024;

class IgnoredFilesError extends Error {
  constructor(reason) {
    super(reason);
    this.name = "IgnoredFilesError";
  }
}

function git(args) {
  const result = spawnSync("git", args, { encoding: "utf8", maxBuffer: maxGitOutputBytes });
  if (result.error !== undefined || result.status !== 0) {
    throw new IgnoredFilesError("GIT_COMMAND_FAILED");
  }
  return result.stdout.trim();
}

function readInput() {
  const buffer = Buffer.alloc(inputChunkBytes);
  const chunks = [];
  let total = 0;
  try {
    for (;;) {
      const length = readSync(0, buffer, 0, buffer.length, null);
      if (length === 0) break;
      if (length > maxGitOutputBytes - total) {
        throw new IgnoredFilesError("INPUT_TOO_LARGE");
      }
      total += length;
      chunks.push(Buffer.from(buffer.subarray(0, length)));
    }
    const contents = Buffer.concat(chunks, total);
    try {
      return contents.toString("utf8");
    } finally {
      contents.fill(0);
    }
  } finally {
    buffer.fill(0);
    for (const chunk of chunks) chunk.fill(0);
  }
}

try {
  const args = process.argv.slice(2);
  if (args.length > 1 || (args.length === 1 && !["--history", "--pre-push"].includes(args[0]))) {
    throw new IgnoredFilesError("UNSUPPORTED_ARGUMENT");
  }
  const authenticated = spawnSync(process.execPath, ["scripts/run_pinned_release_readiness.mjs", "--check-pins"], {
    cwd: packageRoot, encoding: "utf8", maxBuffer: maxGitOutputBytes,
  });
  if (authenticated.error !== undefined || authenticated.status !== 0) {
    throw new IgnoredFilesError("UNAUTHENTICATED_PRIVACY_PACKAGE");
  }
  const { assertRepositoryPrivacy, assertPushPrivacy } =
    await import("../.release-readiness/scripts/repository-privacy.mjs");
  const { assertDocumentationPolicy } =
    await import("../.release-readiness/scripts/documentation-policy.mjs");
  if (args[0] === "--pre-push") {
    assertPushPrivacy({ updates: readInput() });
  } else if (args[0] === "--history") {
    // CI audits the complete ancestry with the same shared policy used for a
    // new remote branch. A shallow checkout cannot prove deleted files absent.
    if (git(["rev-parse", "--is-shallow-repository"]) !== "false") {
      throw new IgnoredFilesError("COMPLETE_HISTORY_REQUIRED");
    }
    const tip = git(["rev-parse", "HEAD"]);
    assertPushPrivacy({ updates: `HEAD ${tip} refs/heads/main ${"0".repeat(tip.length)}\n` });
  } else {
    assertRepositoryPrivacy();
    // Match the exact staged bytes that Git will commit, including policy
    // declarations; unstaged edits must not hide a staged work report.
    assertDocumentationPolicy({ staged: true });
  }
  console.log("tracked files respect committed ignore rules");
} catch (error) {
  const reason = error !== null && typeof error === "object" && "code" in error
    ? error.code : error instanceof IgnoredFilesError ? error.message : "PRIVACY_CHECK_FAILED";
  console.error(`repository privacy check failed: ${reason}`);
  if (error !== null && typeof error === "object" && "paths" in error && Array.isArray(error.paths)) {
    for (const path of error.paths) console.error(JSON.stringify(path));
  }
  process.exitCode = 1;
}
