#!/usr/bin/env node
// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const FULL_SHA_PATTERN = /^[0-9a-f]{40}$/u;
const REPOSITORY_PATTERN = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/u;
const VERSION_PATTERN = /^(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)$/u;
const CODE_CHECK_WORKFLOW = "code-checks.yml";
const PREFLIGHT_WORKFLOW_TITLES = Object.freeze({
  "crates-package-preflight.yml": "Crates package preflight",
  "swift-package-preflight.yml": "Swift package preflight",
  "kotlin-android-package-preflight.yml": "Kotlin Android package preflight",
  "npm-package-preflight.yml": "npm package preflight",
});
const REQUIRED_EVENTS = Object.freeze({
  [CODE_CHECK_WORKFLOW]: "push",
  "crates-package-preflight.yml": "workflow_dispatch",
  "swift-package-preflight.yml": "workflow_dispatch",
  "kotlin-android-package-preflight.yml": "workflow_dispatch",
  "npm-package-preflight.yml": "workflow_dispatch",
});
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

const parsePreflightWorkflow = (value) => {
  if (typeof value !== "string" || value.length === 0) {
    fail("missing-release-attestation-preflight-workflow");
  }
  if (!Object.hasOwn(PREFLIGHT_WORKFLOW_TITLES, value)) {
    fail("unsupported-release-attestation-preflight-workflow");
  }
  return value;
};

const expectedDisplayTitle = (workflow, releaseVersion) => {
  const preflightTitle = PREFLIGHT_WORKFLOW_TITLES[workflow];
  if (preflightTitle === undefined) {
    return null;
  }
  return `${preflightTitle} ${releaseVersion}`;
};

export const run = (command, arguments_, options = {}) => {
  const capturesStdout = options.capture !== false;
  const result = spawnSync(command, arguments_, {
    cwd: options.cwd,
    encoding: "utf8",
    env: options.env,
    maxBuffer: MAX_COMMAND_OUTPUT_BYTES,
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
  const { attempt, conclusion, databaseId, displayTitle, event, headBranch, headSha, status } = value;
  if (
    !Number.isSafeInteger(attempt) ||
    attempt < 1 ||
    !Number.isSafeInteger(databaseId) ||
    databaseId < 1 ||
    typeof displayTitle !== "string" ||
    typeof event !== "string" ||
    typeof headBranch !== "string" ||
    headSha !== releaseSha ||
    typeof status !== "string" ||
    (conclusion !== null && typeof conclusion !== "string")
  ) {
    fail("invalid-workflow-run-response");
  }
  return { attempt, conclusion, databaseId, displayTitle, event, headBranch, headSha, status };
};

const selectLatestRun = (rawRuns, releaseSha, workflow, releaseVersion) => {
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
  const expectedTitle = expectedDisplayTitle(workflow, releaseVersion);
  if (expectedTitle !== null && latest.displayTitle !== expectedTitle) {
    fail("preflight-version-mismatch");
  }
  return latest;
};

export const requireLatestSuccessfulRun = (rawRuns, releaseSha, workflow, releaseVersion) => {
  const latest = selectLatestRun(rawRuns, releaseSha, workflow, releaseVersion);
  // A newer queued, running, cancelled, or failed run invalidates an older
  // success. This prevents a stale successful attempt from authorizing a
  // release after the same checks have been re-run with a worse result.
  if (latest.status !== "completed" || latest.conclusion !== "success") {
    fail(`latest-${workflow}-run-not-successful`);
  }
};

const isWaitableWorkflowFailure = (error, workflow) =>
  error instanceof ReleaseAttestationError &&
  (error.code === `missing-${workflow}-run` ||
    error.code === `latest-${workflow}-run-not-successful`);

const queryWorkflowRuns = ({ cwd, env, releaseSha, repository, workflow }) => {
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
      "attempt,conclusion,databaseId,displayTitle,event,headBranch,headSha,status",
    ],
    { cwd, env, errorCode: `query-${workflow}-failed` },
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
  releaseVersion,
  repository,
  workflow,
  waitSeconds,
  pollSeconds,
}) => {
  const deadlineMs = Date.now() + waitSeconds * 1_000;
  for (;;) {
    const rawRuns = queryWorkflowRuns({ cwd, env, releaseSha, repository, workflow });
    try {
      requireLatestSuccessfulRun(rawRuns, releaseSha, workflow, releaseVersion);
      return;
    } catch (error) {
      if (!isWaitableWorkflowFailure(error, workflow) || Date.now() >= deadlineMs) {
        throw error;
      }
      console.error(`release attestation waiting for ${workflow}: ${error.code}`);
      const remainingSeconds = Math.max(1, Math.ceil((deadlineMs - Date.now()) / 1_000));
      sleepSeconds(Math.min(pollSeconds, remainingSeconds));
    }
  }
};

export const verifyReleaseAttestation = ({ cwd = process.cwd(), env = process.env } = {}) => {
  const releaseSha = env.RELEASE_SHA;
  const releaseVersion = env.RELEASE_VERSION;
  const repository = env.GITHUB_REPOSITORY;
  if (typeof releaseSha !== "string" || !FULL_SHA_PATTERN.test(releaseSha)) {
    fail("invalid-release-sha");
  }
  if (typeof releaseVersion !== "string" || !VERSION_PATTERN.test(releaseVersion)) {
    fail("invalid-release-version");
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
  const preflightWorkflow = parsePreflightWorkflow(env.RELEASE_ATTESTATION_PREFLIGHT_WORKFLOW);

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

  for (const workflow of [CODE_CHECK_WORKFLOW, preflightWorkflow]) {
    requireWorkflowWithOptionalWait({
      cwd,
      env,
      releaseSha,
      releaseVersion,
      repository,
      workflow,
      waitSeconds,
      pollSeconds,
    });
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
