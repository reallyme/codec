#!/usr/bin/env node
// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { spawnSync } from "node:child_process";
import { mkdirSync, mkdtempSync, readSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve, sep } from "node:path";

const maxGitOutputBytes = 16 * 1024 * 1024;
const maxIgnoreFileBytes = 1024 * 1024;
const inputChunkBytes = 64 * 1024;
const objectIdPattern = /^(?:[a-f0-9]{40}|[a-f0-9]{64})$/u;

class IgnoredFilesError extends Error {
  constructor(reason) {
    super(reason);
    this.name = "IgnoredFilesError";
  }
}

function git(args, {
  maxBuffer = maxGitOutputBytes,
  encoding = "utf8",
  input,
  acceptedStatuses = [0],
} = {}) {
  const result = spawnSync("git", args, {
    encoding,
    maxBuffer,
    input,
    stdio: [input === undefined ? "ignore" : "pipe", "pipe", "pipe"],
  });
  if (result.error !== undefined || !acceptedStatuses.includes(result.status)) {
    throw new IgnoredFilesError("GIT_COMMAND_FAILED");
  }
  return result.stdout;
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

function pushTips(input) {
  const tips = new Set();
  for (const line of input.split("\n").filter(Boolean)) {
    const fields = line.split(" ");
    if (fields.length !== 4 || !objectIdPattern.test(fields[1]) || !objectIdPattern.test(fields[3])) {
      throw new IgnoredFilesError("INVALID_PUSH_INPUT");
    }
    // A deletion publishes no object. Tags must peel to commits so an opaque
    // blob cannot bypass the path policy by being pushed directly as a tag.
    if (!/^0+$/u.test(fields[1])) {
      tips.add(git(["rev-parse", "--verify", `${fields[1]}^{commit}`]).trim());
    }
  }
  return [...tips];
}

let scratch;
try {
  const arguments_ = process.argv.slice(2);
  const history = arguments_.length === 1 && arguments_[0] === "--history";
  const prePush = arguments_.length === 1 && arguments_[0] === "--pre-push";
  const npmPaths = arguments_.length === 1 && arguments_[0] === "--npm-paths";
  const pathInput = npmPaths || (arguments_.length === 1 && arguments_[0] === "--paths");
  if (arguments_.length !== 0 && !history && !prePush && !pathInput) {
    throw new IgnoredFilesError("UNSUPPORTED_ARGUMENT");
  }
  const root = git(["rev-parse", "--show-toplevel"]).trimEnd();
  scratch = mkdtempSync(join(tmpdir(), "codec-ignore-check-"));

  // Read rules from the index. Unstaged edits to an ignore file must not hide
  // a force-added file from the commit hook, and developer/global exclusions
  // must not change the policy enforced by CI.
  const entries = git([
    "-C", root, "ls-files", "--stage", "-z", "--", ".gitignore", "**/.gitignore",
  ]).split("\0").filter(Boolean);
  for (const entry of entries) {
    if (entry.includes("\uFFFD")) throw new IgnoredFilesError("INVALID_IGNORE_PATH");
    const tab = entry.indexOf("\t");
    if (tab < 0) throw new IgnoredFilesError("INVALID_INDEX_ENTRY");
    const [mode, object, stage] = entry.slice(0, tab).split(" ");
    if (stage !== "0" || (mode !== "100644" && mode !== "100755")) {
      throw new IgnoredFilesError("INVALID_IGNORE_FILE");
    }
    const destination = resolve(scratch, entry.slice(tab + 1));
    if (!destination.startsWith(`${scratch}${sep}`)) {
      throw new IgnoredFilesError("INVALID_IGNORE_PATH");
    }
    mkdirSync(dirname(destination), { recursive: true });
    writeFileSync(destination, git(["cat-file", "blob", object], {
      maxBuffer: maxIgnoreFileBytes, encoding: null,
    }));
  }

  // Only ignore files are materialized; no tracked application data is copied.
  // Git evaluates nested rules, negations and forced additions against the
  // existing index, while reading the staged rules from this isolated tree.
  let ignored = git([
    "-C", root, `--work-tree=${scratch}`, "ls-files", "--cached", "--ignored",
    "--exclude-per-directory=.gitignore", "-z",
  ]).split("\0").filter(Boolean);
  // The npm distribution contains compiled JavaScript under dist/. This
  // artifact-only admission never relaxes the preceding Git-index check or
  // any private-file rule, including rules within the distribution directory.
  if (npmPaths) {
    const stagedIgnore = entries.find((entry) => entry.endsWith("\t.gitignore"));
    if (stagedIgnore === undefined) throw new IgnoredFilesError("MISSING_IGNORE_POLICY");
    const object = stagedIgnore.slice(0, stagedIgnore.indexOf("\t")).split(" ")[1];
    const rules = git(["cat-file", "blob", object], { maxBuffer: maxIgnoreFileBytes });
    writeFileSync(join(scratch, ".gitignore"), rules.split("\n").filter((rule) => rule !== "dist/").join("\n"));
  }
  const checkPaths = (paths) => {
    if (paths.length === 0) return;
    const matches = git([
      "-C", root, `--work-tree=${scratch}`, "check-ignore", "--no-index", "-z", "--stdin",
    ], { input: `${paths.join("\0")}\0`, acceptedStatuses: [0, 1] });
    ignored = [...new Set([...ignored, ...matches.split("\0").filter(Boolean)])];
  };
  const tips = prePush ? pushTips(readInput()) : history ? ["HEAD"] : [];
  if (tips.length !== 0) {
    // Inspect every parent diff, including roots and merges. Looking only at
    // the final tree would miss a private file added and removed before push.
    if (git(["rev-parse", "--is-shallow-repository"]).trim() !== "false") {
      throw new IgnoredFilesError("COMPLETE_HISTORY_REQUIRED");
    }
    const paths = git([
      "log", "--format=", "--name-only", "-z", "--no-renames", "--root", "-m", ...tips, "--",
    ]);
    if (paths.includes("\uFFFD")) throw new IgnoredFilesError("INVALID_HISTORY_PATH");
    const unique = [...new Set(paths.split("\0").filter(Boolean))];
    checkPaths(unique);
  }
  if (pathInput) {
    const input = readInput();
    if (input !== "" && !input.endsWith("\0")) throw new IgnoredFilesError("INVALID_PATH_INPUT");
    const paths = input.split("\0").filter(Boolean);
    for (const path of paths) {
      if (path.includes("\uFFFD") || path.includes("\\") || path.startsWith("/") ||
          path.split("/").some((part) => part === "." || part === "..")) {
        throw new IgnoredFilesError("INVALID_PATH_INPUT");
      }
    }
    checkPaths(paths);
  }
  if (ignored.length !== 0) {
    console.error("ignored-file check failed: tracked files match committed ignore rules");
    for (const path of ignored) console.error(JSON.stringify(path));
    process.exitCode = 1;
  } else {
    console.log("tracked files respect committed ignore rules");
  }
} catch (error) {
  const reason = error instanceof IgnoredFilesError ? error.message : "CHECK_FAILED";
  console.error(`ignored-file check failed: ${reason}`);
  process.exitCode = 1;
} finally {
  if (scratch !== undefined) {
    try {
      rmSync(scratch, { recursive: true, force: true });
    } catch {
      console.error("ignored-file check failed: CLEANUP_FAILED");
      process.exitCode = 1;
    }
  }
}
