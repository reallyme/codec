#!/usr/bin/env node
// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { readFileSync, writeFileSync } from "node:fs";
import { posix, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const packageManifest = resolve(root, "Package.swift");
const usage =
  "usage: node scripts/prepare_swift_binary_manifest.mjs <version> <swiftpm-checksum> [--local-artifact-path <relative-path>]";

const fail = (message) => {
  console.error(`prepare Swift binary manifest failed: ${message}`);
  process.exit(1);
};

const [, , version, checksum, ...options] = process.argv;
if (version === undefined || checksum === undefined) {
  fail(usage);
}
if (options.length !== 0 && options.length !== 2) {
  fail(usage);
}
if (!/^\d+\.\d+\.\d+$/.test(version)) {
  fail("version must be a semantic version without a leading v");
}
if (!/^[0-9a-f]{64}$/.test(checksum)) {
  fail("checksum must be a 64-character lowercase SHA-256 hex string");
}

let localArtifactPath = "";
if (options.length === 2) {
  const [flag, value] = options;
  if (flag !== "--local-artifact-path") {
    fail(usage);
  }
  if (value === undefined || value.length === 0) {
    fail("local artifact path must not be empty");
  }
  if (value.startsWith("/") || value.includes("\\")) {
    fail("local artifact path must be a relative POSIX path");
  }
  if (!/^[A-Za-z0-9._/-]+$/.test(value)) {
    fail("local artifact path contains unsupported characters");
  }
  const normalized = posix.normalize(value);
  if (normalized === "." || normalized.startsWith("../") || normalized.includes("/../")) {
    fail("local artifact path must stay inside the package root");
  }
  localArtifactPath = normalized;
}

const replaceSingleAssignment = (source, variableName, value) => {
  const pattern = new RegExp(`let ${variableName} = "[^"]*"`);
  if (!pattern.test(source)) {
    fail(`Package.swift does not contain ${variableName}`);
  }
  return source.replace(pattern, `let ${variableName} = "${value}"`);
};

let manifest = readFileSync(packageManifest, "utf8");
manifest = replaceSingleAssignment(manifest, "ffiArtifactChecksum", checksum);
manifest = replaceSingleAssignment(manifest, "ffiArtifactVersion", version);
manifest = replaceSingleAssignment(manifest, "ffiArtifactLocalPathOverride", localArtifactPath);
writeFileSync(packageManifest, manifest);
