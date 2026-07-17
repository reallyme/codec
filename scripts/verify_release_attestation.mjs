#!/usr/bin/env node
// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const FULL_SHA_PATTERN = /^[0-9a-f]{40}$/u;
const REPOSITORY_PATTERN = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/u;
const REQUIRED_WORKFLOWS = Object.freeze(["code-checks.yml", "release-preflight.yml"]);
const REQUIRED_EVENTS = Object.freeze({
  "code-checks.yml": "push",
  "release-preflight.yml": "workflow_dispatch",
});
const MAX_COMMAND_OUTPUT_BYTES = 1_048_576;

export class ReleaseAttestationError extends Error {
  constructor(code) {
    super(code);
    this.name = "ReleaseAttestationError";
    this.code = code;
  }
}

const fail = (code) => {
  throw new ReleaseAttestationError(code);
};

const run = (command, arguments_, options = {}) => {
  const result = spawnSync(command, arguments_, {
    cwd: options.cwd,
    encoding: "utf8",
    env: options.env,
    maxBuffer: MAX_COMMAND_OUTPUT_BYTES,
    stdio: options.capture === false ? ["ignore", "ignore", "ignore"] : ["ignore", "pipe", "ignore"],
  });
  if (result.error !== undefined || result.status !== 0) {
    fail(options.errorCode ?? "command-failed");
  }
  return result.stdout.trim();
};

const validateRun = (value, releaseSha) => {
  if (value === null || typeof value !== "object" || Array.isArray(value)) {
    fail("invalid-workflow-run-response");
  }
  const { attempt, conclusion, databaseId, event, headBranch, headSha, status } = value;
  if (
    !Number.isSafeInteger(attempt) ||
    attempt < 1 ||
    !Number.isSafeInteger(databaseId) ||
    databaseId < 1 ||
    typeof event !== "string" ||
    typeof headBranch !== "string" ||
    headSha !== releaseSha ||
    typeof status !== "string" ||
    (conclusion !== null && typeof conclusion !== "string")
  ) {
    fail("invalid-workflow-run-response");
  }
  return { attempt, conclusion, databaseId, event, headBranch, headSha, status };
};

export const requireLatestSuccessfulRun = (rawRuns, releaseSha, workflow) => {
  if (!Array.isArray(rawRuns)) {
    fail("invalid-workflow-run-response");
  }
  const expectedEvent = REQUIRED_EVENTS[workflow];
  if (expectedEvent === undefined) {
    fail("unsupported-required-workflow");
  }
  const runs = rawRuns
    .map((runValue) => validateRun(runValue, releaseSha))
    .filter((runValue) => runValue.event === expectedEvent && runValue.headBranch === "main");
  runs.sort((left, right) => {
    if (left.databaseId !== right.databaseId) {
      return right.databaseId - left.databaseId;
    }
    return right.attempt - left.attempt;
  });
  const latest = runs[0];
  if (latest === undefined) {
    fail(`missing-${workflow}-run`);
  }
  // A newer queued, running, cancelled, or failed run invalidates an older
  // success. This prevents a stale successful attempt from authorizing a
  // release after the same checks have been re-run with a worse result.
  if (latest.status !== "completed" || latest.conclusion !== "success") {
    fail(`latest-${workflow}-run-not-successful`);
  }
};

export const verifyReleaseAttestation = ({ cwd = process.cwd(), env = process.env } = {}) => {
  const releaseSha = env.RELEASE_SHA;
  const repository = env.GITHUB_REPOSITORY;
  if (typeof releaseSha !== "string" || !FULL_SHA_PATTERN.test(releaseSha)) {
    fail("invalid-release-sha");
  }
  if (typeof repository !== "string" || !REPOSITORY_PATTERN.test(repository)) {
    fail("invalid-github-repository");
  }
  if (typeof env.GH_TOKEN !== "string" || env.GH_TOKEN.length === 0) {
    fail("missing-github-token");
  }

  const checkedOutSha = run("git", ["rev-parse", "HEAD"], {
    cwd,
    env,
    errorCode: "git-head-unavailable",
  });
  if (checkedOutSha !== releaseSha) {
    fail("checkout-does-not-match-release-sha");
  }
  run("git", ["fetch", "--no-tags", "origin", "main"], {
    cwd,
    env,
    capture: false,
    errorCode: "origin-main-fetch-failed",
  });
  const mainSha = run("git", ["rev-parse", "origin/main"], {
    cwd,
    env,
    errorCode: "origin-main-unavailable",
  });
  if (mainSha !== releaseSha) {
    fail("release-sha-is-not-current-main");
  }

  for (const workflow of REQUIRED_WORKFLOWS) {
    const encoded = run(
      "gh",
      [
        "run",
        "list",
        "--repo",
        repository,
        "--workflow",
        workflow,
        "--commit",
        releaseSha,
        "--limit",
        "100",
        "--json",
        "attempt,conclusion,databaseId,event,headBranch,headSha,status",
      ],
      { cwd, env, errorCode: `query-${workflow}-failed` },
    );
    let rawRuns;
    try {
      rawRuns = JSON.parse(encoded);
    } catch {
      fail("invalid-workflow-run-response");
    }
    requireLatestSuccessfulRun(rawRuns, releaseSha, workflow);
  }
};

const isMain = process.argv[1] !== undefined && fileURLToPath(import.meta.url) === process.argv[1];
if (isMain) {
  try {
    verifyReleaseAttestation();
    console.log("release attestation verified for current main and latest required workflow runs");
  } catch (error) {
    const code = error instanceof ReleaseAttestationError ? error.code : "unexpected-failure";
    console.error(`release attestation failed: ${code}`);
    process.exit(1);
  }
}
