// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import test from "node:test";

const testDirectory = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = resolve(testDirectory, "../..");
const readinessScript = "scripts/check_release_readiness.mjs";

const runReadiness = (arguments_) =>
  spawnSync(process.execPath, [readinessScript, ...arguments_], {
    cwd: repositoryRoot,
    encoding: "utf8",
    stdio: "pipe",
  });

test("release readiness rejects unknown arguments before running checks", () => {
  const result = runReadiness(["--release-package"]);

  assert.equal(result.error, undefined);
  assert.equal(result.status, 1);
  assert.equal(result.stdout, "");
  assert.match(
    result.stderr,
    /release readiness check failed: unsupported argument --release-package/u,
  );
});

test("release readiness rejects duplicate arguments", () => {
  const result = runReadiness([
    "--generated-freshness",
    "--generated-freshness",
  ]);

  assert.equal(result.error, undefined);
  assert.equal(result.status, 1);
  assert.equal(result.stdout, "");
  assert.match(
    result.stderr,
    /release readiness check failed: argument --generated-freshness was specified more than once/u,
  );
});
