// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import test from "node:test";

import {
  ReleaseAttestationError,
  requireLatestSuccessfulRun,
  run,
} from "./verify_release_attestation.mjs";

const releaseSha = "a".repeat(40);

const workflowRun = (overrides = {}) => ({
  attempt: 1,
  conclusion: "success",
  databaseId: 100,
  displayTitle: "Code Checks",
  event: "push",
  headBranch: "main",
  headSha: releaseSha,
  status: "completed",
  ...overrides,
});

test("package preflight attestation is bound to the requested version", () => {
  assert.doesNotThrow(() => {
    requireLatestSuccessfulRun(
      [workflowRun({ displayTitle: "Swift package preflight 0.2.0", event: "workflow_dispatch" })],
      releaseSha,
      "swift-package-preflight.yml",
      "0.2.0",
    );
  });
  assert.throws(
    () => {
      requireLatestSuccessfulRun(
        [
          workflowRun({
            displayTitle: "Kotlin Android package preflight 0.2.0",
            event: "workflow_dispatch",
          }),
        ],
        releaseSha,
        "kotlin-android-package-preflight.yml",
        "0.3.0",
      );
    },
    (error) =>
      error instanceof ReleaseAttestationError && error.code === "preflight-version-mismatch",
  );
});

test("latest successful workflow attempt authorizes release", () => {
  assert.doesNotThrow(() => {
    requireLatestSuccessfulRun(
      [workflowRun({ attempt: 1 }), workflowRun({ attempt: 2 })],
      releaseSha,
      "code-checks.yml",
    );
  });
});

test("newer failed run invalidates an older success", () => {
  assert.throws(
    () => {
      requireLatestSuccessfulRun(
        [workflowRun({ databaseId: 100 }), workflowRun({ conclusion: "failure", databaseId: 101 })],
        releaseSha,
        "code-checks.yml",
      );
    },
    (error) =>
      error instanceof ReleaseAttestationError &&
      error.code === "latest-code-checks.yml-run-not-successful",
  );
});

test("newer in-progress run invalidates an older success", () => {
  assert.throws(
    () => {
      requireLatestSuccessfulRun(
        [
          workflowRun({
            databaseId: 100,
            displayTitle: "npm package preflight 0.2.0",
            event: "workflow_dispatch",
          }),
          workflowRun({
            conclusion: null,
            databaseId: 101,
            displayTitle: "npm package preflight 0.2.0",
            event: "workflow_dispatch",
            status: "in_progress",
          }),
        ],
        releaseSha,
        "npm-package-preflight.yml",
        "0.2.0",
      );
    },
    (error) =>
      error instanceof ReleaseAttestationError &&
      error.code === "latest-npm-package-preflight.yml-run-not-successful",
  );
});

test("pull-request success cannot substitute for a main push check", () => {
  assert.throws(() => {
    requireLatestSuccessfulRun(
      [workflowRun({ databaseId: 101, event: "pull_request", headBranch: "feature" })],
      releaseSha,
      "code-checks.yml",
    );
  }, ReleaseAttestationError);
});

test("malformed or wrong-SHA workflow data fails closed", () => {
  assert.throws(() => {
    requireLatestSuccessfulRun(
      [workflowRun({ headSha: "b".repeat(40) })],
      releaseSha,
      "code-checks.yml",
    );
  }, ReleaseAttestationError);
  assert.throws(() => {
    requireLatestSuccessfulRun({}, releaseSha, "code-checks.yml");
  }, ReleaseAttestationError);
});

test("silent successful command does not require captured stdout", () => {
  assert.equal(run(process.execPath, ["-e", ""], { capture: false }), "");
});
