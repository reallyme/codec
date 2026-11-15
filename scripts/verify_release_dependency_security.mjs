#!/usr/bin/env node
// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const FULL_SHA_PATTERN = /^[0-9a-f]{40}$/u;
const REPOSITORY_PATTERN = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/u;
const DEPENDENCY_SECURITY_WORKFLOW = "dependency-security.yml";
const MAX_COMMAND_OUTPUT_BYTES = 1_048_576;
const MAX_WAIT_SECONDS = 7_200;
const DEFAULT_POLL_SECONDS = 20;
const MAX_POLL_SECONDS = 300;
const NON_NEGATIVE_INTEGER_PATTERN = /^(0|[1-9][0-9]*)$/u;

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

const parseSeconds = (value, defaultValue, errorCode, maximum) => {
  if (value === undefined || value === "") {
    return defaultValue;
  }
  if (typeof value !== "string" || !NON_NEGATIVE_INTEGER_PATTERN.test(value)) {
    fail(errorCode);
  }
  const seconds = Number(value);
  if (!Number.isSafeInteger(seconds) || seconds > maximum) {
    fail(errorCode);
  }
  return seconds;
};

const sleepSeconds = (seconds) => {
  if (seconds === 0) {
    return;
  }
  const waitBuffer = new SharedArrayBuffer(4);
  const waitView = new Int32Array(waitBuffer);
  Atomics.wait(waitView, 0, 0, seconds * 1_000);
};

export const run = (command, arguments_, options = {}) => {
  const capturesStdout = options.capture !== false;
  const result = spawnSync(command, arguments_, {
    cwd: options.cwd,
    encoding: "utf8",
    env: options.env,
    maxBuffer: MAX_COMMAND_OUTPUT_BYTES,
    timeout: 30_000,
    stdio: capturesStdout ? ["ignore", "pipe", "ignore"] : ["ignore", "ignore", "ignore"],
  });
  if (result.error !== undefined || result.status !== 0) {
    fail(options.errorCode ?? "command-failed");
  }
  if (!capturesStdout) {
    return "";
  }
  if (typeof result.stdout !== "string") {
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

const selectLatestRun = (rawRuns, releaseSha) => {
  if (!Array.isArray(rawRuns)) {
    fail("invalid-workflow-run-response");
  }
  const runs = rawRuns
    .map((runValue) => validateRun(runValue, releaseSha))
    .filter((runValue) => runValue.event === "push" && runValue.headBranch === "main");
  runs.sort((left, right) => {
    if (left.databaseId !== right.databaseId) {
      return right.databaseId - left.databaseId;
    }
    return right.attempt - left.attempt;
  });
  const latest = runs[0];
  if (latest === undefined) {
    fail(`missing-${DEPENDENCY_SECURITY_WORKFLOW}-run`);
  }
  return latest;
};

export const requireLatestSuccessfulRun = (rawRuns, releaseSha) => {
  const latest = selectLatestRun(rawRuns, releaseSha);
  // A newer queued, running, cancelled, or failed run invalidates an older
  // success. This prevents a stale successful attempt from authorizing a
  // release after the same checks have been re-run with a worse result.
  if (latest.status !== "completed" || latest.conclusion !== "success") {
    fail(`latest-${DEPENDENCY_SECURITY_WORKFLOW}-run-not-successful`);
  }
  return latest;
};

const isWaitableWorkflowFailure = (error) =>
  error instanceof ReleaseAttestationError &&
  (error.code === `missing-${DEPENDENCY_SECURITY_WORKFLOW}-run` ||
    error.code === `latest-${DEPENDENCY_SECURITY_WORKFLOW}-run-not-successful`);

const queryWorkflowRuns = ({ cwd, env, releaseSha, repository }) => {
  const encoded = run(
    "gh",
    [
      "run",
      "list",
      "--repo",
      repository,
      "--workflow",
      DEPENDENCY_SECURITY_WORKFLOW,
      "--commit",
      releaseSha,
      "--limit",
      "100",
      "--json",
      "attempt,conclusion,databaseId,event,headBranch,headSha,status",
    ],
    { cwd, env, errorCode: `query-${DEPENDENCY_SECURITY_WORKFLOW}-failed` },
  );
  try {
    return JSON.parse(encoded);
  } catch {
    fail("invalid-workflow-run-response");
  }
};

const requireWorkflowWithOptionalWait = ({
  cwd,
  env,
  releaseSha,
  repository,
  waitSeconds,
  pollSeconds,
}) => {
  const deadlineMs = Date.now() + waitSeconds * 1_000;
  for (;;) {
    const rawRuns = queryWorkflowRuns({ cwd, env, releaseSha, repository });
    try {
      return requireLatestSuccessfulRun(rawRuns, releaseSha);
    } catch (error) {
      if (!isWaitableWorkflowFailure(error) || Date.now() >= deadlineMs) {
        throw error;
      }
      console.error(`release attestation waiting for ${DEPENDENCY_SECURITY_WORKFLOW}: ${error.code}`);
      const remainingSeconds = Math.max(1, Math.ceil((deadlineMs - Date.now()) / 1_000));
      sleepSeconds(Math.min(pollSeconds, remainingSeconds));
    }
  }
};

export const verifyReleaseDependencySecurity = ({ cwd = process.cwd(), env = process.env } = {}) => {
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
  const waitSeconds = parseSeconds(
    env.RELEASE_ATTESTATION_WAIT_SECONDS,
    0,
    "invalid-release-attestation-wait-seconds",
    MAX_WAIT_SECONDS,
  );
  const pollSeconds = parseSeconds(
    env.RELEASE_ATTESTATION_POLL_SECONDS,
    DEFAULT_POLL_SECONDS,
    "invalid-release-attestation-poll-seconds",
    MAX_POLL_SECONDS,
  );

  const checkedOutSha = run("git", ["rev-parse", "HEAD"], {
    cwd,
    env,
    errorCode: "git-head-unavailable",
  });
  if (checkedOutSha !== releaseSha) {
    fail("checkout-does-not-match-release-sha");
  }
  return requireWorkflowWithOptionalWait({ cwd, env, releaseSha, repository, waitSeconds, pollSeconds });
};

const isMain = process.argv[1] !== undefined && fileURLToPath(import.meta.url) === process.argv[1];
if (isMain) {
  try {
    if (process.argv.length !== 2) {
      fail("invalid-dependency-security-arguments");
    }
    const run = verifyReleaseDependencySecurity();
    console.log(`Verified dependency security run ${run.databaseId}, attempt ${run.attempt}`);
  } catch (error) {
    const code = error instanceof ReleaseAttestationError ? error.code : "unexpected-verification-failure";
    console.error(`Release dependency security failed: ${code}`);
    process.exitCode = 1;
  }
}
