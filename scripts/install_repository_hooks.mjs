#!/usr/bin/env node
// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { spawnSync } from "node:child_process";
import { fchmodSync, mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { FileSnapshotError, openRegularFileSnapshot } from "./open_regular_file_snapshot.mjs";

class HookInstallationError extends Error {
  constructor(reason) {
    super(reason);
    this.name = "HookInstallationError";
  }
}

function git(args, acceptedStatuses = [0]) {
  const result = spawnSync("git", args, { encoding: "utf8" });
  if (result.error !== undefined || !acceptedStatuses.includes(result.status)) {
    throw new HookInstallationError("GIT_COMMAND_FAILED");
  }
  return result.stdout.trimEnd();
}

try {
  if (process.argv.length !== 2) throw new HookInstallationError("UNSUPPORTED_ARGUMENT");
  if (git(["config", "--get", "core.hooksPath"], [0, 1]) !== "") {
    throw new HookInstallationError("CUSTOM_HOOK_PATH_REQUIRES_INTEGRATION");
  }
  const directory = git(["rev-parse", "--path-format=absolute", "--git-path", "hooks"]);
  const hooks = ["pre-commit", "pre-push"].map((name) => ({
    path: join(directory, name),
    contents: readFileSync(new URL(`../.github/hooks/${name}`, import.meta.url)),
  }));
  // Inspect both hooks before writing either. Existing, unrelated hooks need
  // deliberate integration; silently replacing them would discard protection.
  try {
    for (const hook of hooks) {
      try {
        hook.snapshot = openRegularFileSnapshot(hook.path, hook.contents.length);
        if (!hook.snapshot.bytes.equals(hook.contents)) {
          throw new HookInstallationError("EXISTING_HOOK_REQUIRES_INTEGRATION");
        }
      } catch (error) {
        if (!(error instanceof FileSnapshotError) || error.code !== "file-missing") throw error;
      }
    }
    mkdirSync(directory, { recursive: true });
    for (const hook of hooks) {
      if (hook.snapshot !== undefined) {
        hook.snapshot.assertUnchanged();
        fchmodSync(hook.snapshot.descriptor, 0o755);
      } else {
        writeFileSync(hook.path, hook.contents, { flag: "wx", mode: 0o755 });
      }
    }
  } finally {
    let closeFailed = false;
    for (const hook of hooks) {
      try { hook.snapshot?.close(); } catch { closeFailed = true; }
    }
    if (closeFailed) throw new HookInstallationError("HOOK_CLOSE_FAILED");
  }
  console.log("repository privacy hooks installed");
} catch {
  console.error("hook installation failed: preserve existing hooks and integrate the repository guards before committing or pushing");
  process.exitCode = 1;
}
