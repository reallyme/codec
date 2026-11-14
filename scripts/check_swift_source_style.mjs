// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const mode = process.argv[2];
if (process.argv.length !== 3 || !["format", "lint"].includes(mode)) {
  console.error("Swift source style requires a format or lint check");
  process.exit(1);
}
const cwd = fileURLToPath(new URL("../", import.meta.url));
const tracked = spawnSync("git", ["ls-files", "-z", "packages/swift/Sources", "packages/swift/Tests"], {
  cwd, encoding: "utf8", maxBuffer: 1024 * 1024,
});
if (tracked.error !== undefined || tracked.status !== 0) process.exit(1);
const files = tracked.stdout.split("\0").filter((path) => path.endsWith(".swift") && !path.includes("/Generated/"));
if (files.length === 0) process.exit(1);
if (mode === "lint") {
  const result = spawnSync("swift", ["format", "lint", "--strict", "--configuration", ".swift-format", ...files], {
    cwd, stdio: "inherit",
  });
  process.exit(result.error === undefined && Number.isInteger(result.status) ? result.status : 1);
}
for (const path of files) {
  const input = readFileSync(new URL(`../${path}`, import.meta.url));
  const result = spawnSync("swift", ["format", "--configuration", ".swift-format", path], {
    cwd, maxBuffer: 1024 * 1024,
  });
  if (result.error !== undefined || result.status !== 0 || !input.equals(result.stdout)) {
    console.error(`Swift format check failed for ${path}`);
    process.exitCode = 1;
  }
}
