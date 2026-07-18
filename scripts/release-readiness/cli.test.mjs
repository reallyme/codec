// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import test from "node:test";

const testDirectory = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = resolve(testDirectory, "../..");
const readinessScript = "scripts/check_release_readiness.mjs";
const readinessSource = readFileSync(
  resolve(repositoryRoot, readinessScript),
  "utf8",
);

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

test("repository-local source policy is verified before it executes", () => {
  const policyPath = "scripts/release-readiness/source-policy.mjs";
  const trackedCheck = readinessSource.indexOf(`requireTracked("${policyPath}")`);
  const dynamicImport = readinessSource.indexOf(
    'await import("./release-readiness/source-policy.mjs")',
  );

  assert.equal(
    readinessSource.includes('from "./release-readiness/source-policy.mjs"'),
    false,
  );
  assert.ok(trackedCheck >= 0);
  assert.ok(dynamicImport > trackedCheck);
});

test("tracked-mode file listing rejects missing directories", () => {
  const result = spawnSync(
    process.execPath,
    [
      "--input-type=module",
      "-e",
      `
        import { pathToFileURL } from "node:url";
        import { createReleaseReadinessContext } from "./scripts/release-readiness/core.mjs";

        const { listFiles } = createReleaseReadinessContext({
          scriptUrl: pathToFileURL(\`\${process.cwd()}/README.md\`).href,
          repoRoot: ".",
          requireTrackedFiles: true,
        });
        listFiles("release-readiness-definitely-missing-directory");
      `,
    ],
    {
      cwd: repositoryRoot,
      encoding: "utf8",
      stdio: "pipe",
    },
  );

  assert.equal(result.error, undefined);
  assert.equal(result.status, 1);
  assert.equal(result.stdout, "");
  assert.match(result.stderr, /release readiness check failed:/u);
  assert.doesNotMatch(result.stderr, /has no tracked files/u);
});
