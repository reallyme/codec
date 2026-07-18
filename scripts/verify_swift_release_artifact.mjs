#!/usr/bin/env node
// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { execFileSync } from "node:child_process";
import { lstatSync, readFileSync } from "node:fs";

const CHECKSUM_PATTERN = /^[0-9a-f]{64}$/u;
const VERSION_PATTERN = /^(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)$/u;
const MAX_ARCHIVE_BYTES = 536_870_912;
const MAX_MANIFEST_BYTES = 1_048_576;

const fail = (message) => {
  console.error(`Swift release artifact verification failed: ${message}`);
  process.exit(1);
};

const [, , archivePath, sidecarPath, manifestPath, expectedVersion] = process.argv;
if (
  archivePath === undefined ||
  sidecarPath === undefined ||
  manifestPath === undefined ||
  expectedVersion === undefined ||
  !VERSION_PATTERN.test(expectedVersion)
) {
  fail("expected an archive, checksum sidecar, Package.swift, and semantic version");
}

const assertRegularFile = (path, maximumBytes, label) => {
  let status;
  try {
    status = lstatSync(path);
  } catch {
    fail(`${label} is inaccessible`);
  }
  if (status.isSymbolicLink() || !status.isFile() || status.size === 0 || status.size > maximumBytes) {
    fail(`${label} is not a bounded regular file`);
  }
};

const readRegularFile = (path, maximumBytes, label) => {
  assertRegularFile(path, maximumBytes, label);
  try {
    return readFileSync(path);
  } catch {
    fail(`${label} could not be read`);
  }
};

assertRegularFile(archivePath, MAX_ARCHIVE_BYTES, "xcframework archive");
const sidecarText = readRegularFile(sidecarPath, 128, "checksum sidecar").toString("utf8");
const sidecar = sidecarText.endsWith("\n") ? sidecarText.slice(0, -1) : "";
const manifest = readRegularFile(manifestPath, MAX_MANIFEST_BYTES, "Swift package manifest").toString("utf8");
if (!CHECKSUM_PATTERN.test(sidecar) || sidecarText !== `${sidecar}\n`) {
  fail("checksum sidecar is malformed");
}

let computedChecksum;
try {
  computedChecksum = execFileSync("swift", ["package", "compute-checksum", archivePath], {
    encoding: "utf8",
    maxBuffer: 1024,
    stdio: ["ignore", "pipe", "ignore"],
  }).trim();
} catch {
  fail("SwiftPM could not compute the archive checksum");
}
if (!CHECKSUM_PATTERN.test(computedChecksum)) {
  fail("SwiftPM returned a malformed archive checksum");
}
if (computedChecksum !== sidecar) {
  fail("downloaded archive does not match its checksum sidecar");
}

const exactAssignment = (name, valuePattern) => {
  const expression = new RegExp(`^let ${name} = "(${valuePattern})"$`, "gmu");
  const matches = [...manifest.matchAll(expression)];
  if (matches.length !== 1) {
    fail(`Package.swift must define exactly one ${name} assignment`);
  }
  return matches[0][1];
};

if (exactAssignment("ffiArtifactChecksum", "[0-9a-f]{64}") !== computedChecksum) {
  fail("Package.swift checksum does not match the downloaded archive");
}
if (exactAssignment("ffiArtifactVersion", "(?:0|[1-9][0-9]*)\\.(?:0|[1-9][0-9]*)\\.(?:0|[1-9][0-9]*)") !== expectedVersion) {
  fail("Package.swift version does not match the requested release");
}
if (exactAssignment("ffiArtifactLocalPathOverride", "") !== "") {
  fail("Package.swift local artifact override must be empty for release verification");
}
if (
  !manifest.includes(
    'url: "https://github.com/reallyme/codec/releases/download/v\\(ffiArtifactVersion)/ReallyMeCodecFFI.xcframework.zip"',
  )
) {
  fail("Package.swift binary target URL is not bound to ffiArtifactVersion");
}
if (!/^\s*checksum:\s*ffiArtifactChecksum\s*$/mu.test(manifest)) {
  fail("Package.swift binary target checksum is not bound to ffiArtifactChecksum");
}

console.log("Swift release archive, sidecar, and Package.swift are byte-bound and consistent");
