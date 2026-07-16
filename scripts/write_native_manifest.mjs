#!/usr/bin/env node
// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import { mkdirSync, readdirSync, readFileSync, statSync, writeFileSync } from "node:fs";
import { dirname, extname, relative, resolve, sep } from "node:path";

const usage = "usage: node scripts/write_native_manifest.mjs <native-root> <manifest-path>";

const fail = (message) => {
  console.error(`write native manifest failed: ${message}`);
  process.exit(1);
};

const [, , nativeRootArg, manifestPathArg] = process.argv;
if (nativeRootArg === undefined || manifestPathArg === undefined) {
  fail(usage);
}

const nativeRoot = resolve(nativeRootArg);
const manifestPath = resolve(manifestPathArg);

try {
  if (!statSync(nativeRoot).isDirectory()) {
    fail(`${nativeRoot} is not a directory`);
  }
} catch {
  fail(`${nativeRoot} does not exist`);
}

const nativeExtensions = new Set([".dll", ".dylib", ".so"]);

const walk = (directory) => {
  const entries = [];
  for (const dirent of readdirSync(directory, { withFileTypes: true })) {
    const path = resolve(directory, dirent.name);
    if (dirent.isDirectory()) {
      entries.push(...walk(path));
    } else if (dirent.isFile() && nativeExtensions.has(extname(dirent.name))) {
      entries.push(path);
    }
  }
  return entries;
};

const gitCommitSha = () => {
  if (process.env.GITHUB_SHA !== undefined && /^[0-9a-f]{40}$/.test(process.env.GITHUB_SHA)) {
    return process.env.GITHUB_SHA;
  }
  try {
    return execFileSync("git", ["rev-parse", "HEAD"], {
      cwd: process.cwd(),
      encoding: "utf8",
      stdio: ["ignore", "pipe", "ignore"],
    }).trim();
  } catch {
    fail("unable to determine git commit SHA");
  }
};

const nativeFiles = walk(nativeRoot).sort((left, right) => left.localeCompare(right));
if (nativeFiles.length === 0) {
  fail(`${nativeRoot} does not contain native library files`);
}

const entries = nativeFiles.map((path) => {
  const bytes = readFileSync(path);
  return {
    path: relative(nativeRoot, path).split(sep).join("/"),
    sha256: createHash("sha256").update(bytes).digest("hex"),
    size: bytes.length,
  };
});

const manifest = {
  schemaVersion: 1,
  package: "reallyme-codec-native",
  commitSha: gitCommitSha(),
  entries,
};

mkdirSync(dirname(manifestPath), { recursive: true });
writeFileSync(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
