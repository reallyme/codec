// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import test from "node:test";

import {
  ReleaseAttestationError,
  requireLatestSuccessfulRun,
} from "./verify_release_attestation.mjs";

const releaseSha = "a".repeat(40);

const run = (overrides = {}) => ({
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
  assert.doesNotThrow(() => {
    requireLatestSuccessfulRun(
      [run({ attempt: 1 }), run({ attempt: 2 })],
      releaseSha,
      "code-checks.yml",
    );
  });
});

test("newer failed run invalidates an older success", () => {
  assert.throws(
    () => {
      requireLatestSuccessfulRun(
        [run({ databaseId: 100 }), run({ conclusion: "failure", databaseId: 101 })],
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
  assert.throws(() => {
    requireLatestSuccessfulRun(
      [
        run({ databaseId: 100, event: "workflow_dispatch" }),
        run({
          conclusion: null,
          databaseId: 101,
          event: "workflow_dispatch",
          status: "in_progress",
        }),
      ],
      releaseSha,
      "release-preflight.yml",
    );
  }, ReleaseAttestationError);
});

test("pull-request success cannot substitute for a main push check", () => {
  assert.throws(() => {
    requireLatestSuccessfulRun(
      [run({ databaseId: 101, event: "pull_request", headBranch: "feature" })],
      releaseSha,
      "code-checks.yml",
    );
  }, ReleaseAttestationError);
});

test("malformed or wrong-SHA workflow data fails closed", () => {
  assert.throws(() => {
    requireLatestSuccessfulRun([run({ headSha: "b".repeat(40) })], releaseSha, "code-checks.yml");
  }, ReleaseAttestationError);
  assert.throws(() => {
    requireLatestSuccessfulRun({}, releaseSha, "code-checks.yml");
  }, ReleaseAttestationError);
});
