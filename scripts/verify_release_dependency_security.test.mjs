// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import test from "node:test";

import {
  ReleaseAttestationError,
  requireLatestSuccessfulRun,
  run,
} from "./verify_release_dependency_security.mjs";

const releaseSha = "a".repeat(40);

const workflowRun = (overrides = {}) => ({
  attempt: 1,
  conclusion: "success",
  databaseId: 100,
  event: "push",
  headBranch: "main",
  headSha: releaseSha,
  status: "completed",
  ...overrides,
});

test("latest successful workflow attempt authorizes release", () => {
  const latest = requireLatestSuccessfulRun(
    [workflowRun({ attempt: 1 }), workflowRun({ attempt: 2 })],
    releaseSha,
  );
  assert.equal(latest.databaseId, 100);
  assert.equal(latest.attempt, 2);
});

test("newer failed run invalidates an older success", () => {
  assert.throws(
    () => {
      requireLatestSuccessfulRun(
        [workflowRun({ databaseId: 100 }), workflowRun({ conclusion: "failure", databaseId: 101 })],
        releaseSha,
        );
    },
    (error) =>
      error instanceof ReleaseAttestationError &&
      error.code === "latest-dependency-security.yml-run-not-successful",
  );
});

test("newer in-progress run invalidates an older success", () => {
  assert.throws(
    () => {
      requireLatestSuccessfulRun(
        [workflowRun(), workflowRun({ conclusion: null, databaseId: 101, status: "in_progress" })],
        releaseSha,
      );
    },
    (error) =>
      error instanceof ReleaseAttestationError &&
      error.code === "latest-dependency-security.yml-run-not-successful",
  );
});

test("pull-request success cannot substitute for a main push check", () => {
  assert.throws(() => {
    requireLatestSuccessfulRun(
      [workflowRun({ databaseId: 101, event: "pull_request", headBranch: "feature" })],
      releaseSha,
    );
  }, ReleaseAttestationError);
});

test("malformed or wrong-SHA workflow data fails closed", () => {
  assert.throws(() => {
    requireLatestSuccessfulRun(
      [workflowRun({ headSha: "b".repeat(40) })],
      releaseSha,
    );
  }, ReleaseAttestationError);
  assert.throws(() => {
    requireLatestSuccessfulRun({}, releaseSha);
  }, ReleaseAttestationError);
});

test("silent successful command does not require captured stdout", () => {
  assert.equal(run(process.execPath, ["-e", ""], { capture: false }), "");
});
