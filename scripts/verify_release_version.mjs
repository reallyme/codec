#!/usr/bin/env node
// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const versionPattern = /^(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)$/u;

export class ReleaseVersionError extends Error {
  constructor(code) {
    super(code);
    this.name = "ReleaseVersionError";
    this.code = code;
  }
}

const fail = (code) => {
  throw new ReleaseVersionError(code);
};

const read = (cwd, path) => readFileSync(resolve(cwd, path), "utf8");

const declaredVersion = (source, declaration) => {
  const matches = [...source.matchAll(declaration)];
  if (matches.length !== 1 || typeof matches[0][1] !== "string") {
    fail("missing-or-duplicate-version-declaration");
  }
  return matches[0][1];
};

export const verifyReleaseVersion = (expected, sources, workspaceVersions) => {
  if (typeof expected !== "string" || !versionPattern.test(expected)) {
    fail("invalid-release-version");
  }
  if (!Array.isArray(workspaceVersions) || workspaceVersions.length === 0) {
    fail("missing-workspace-packages");
  }
  if (workspaceVersions.some((version) => version !== expected)) {
    fail("workspace-version-mismatch");
  }
  for (const [source, pattern, code] of [
    [sources.swift, /^let packageVersion = "([^"]+)"$/gmu, "swift-version-mismatch"],
    [sources.kotlin, /^version = "([^"]+)"$/gmu, "kotlin-version-mismatch"],
    [sources.android, /^version = "([^"]+)"$/gmu, "android-version-mismatch"],
  ]) {
    if (declaredVersion(source, pattern) !== expected) {
      fail(code);
    }
  }
  const components = expected.split(".");
  for (const [name, component] of [
    ["Major", components[0]],
    ["Minor", components[1]],
    ["Patch", components[2]],
  ]) {
    const declaration = new RegExp(`^private let expectedCodecPackage${name}: UInt32 = ([0-9]+)$`, "gmu");
    if (declaredVersion(sources.swiftFfi, declaration) !== component) {
      fail("swift-ffi-version-mismatch");
    }
  }
  for (const [source, code] of [
    [sources.npm, "npm-version-mismatch"],
    [sources.npmLock, "npm-lock-version-mismatch"],
  ]) {
    const manifest = JSON.parse(source);
    if (manifest.version !== expected) {
      fail(code);
    }
  }
  const lock = JSON.parse(sources.npmLock);
  if (lock.packages?.[""]?.version !== expected) {
    fail("npm-lock-root-version-mismatch");
  }
};

export const verifyWorkspacePathRequirements = (expected, packages, workspaceMembers) => {
  if (!Array.isArray(packages) || !Array.isArray(workspaceMembers)) {
    fail("invalid-workspace-metadata");
  }
  const members = new Set(workspaceMembers);
  const memberPackages = packages.filter((pkg) => members.has(pkg.id));
  if (memberPackages.length !== members.size || memberPackages.length === 0) {
    fail("invalid-workspace-metadata");
  }
  const memberNames = new Set(memberPackages.map((pkg) => pkg.name));
  for (const pkg of memberPackages) {
    if (pkg.version !== expected || !Array.isArray(pkg.dependencies)) {
      fail("workspace-version-mismatch");
    }
    for (const dependency of pkg.dependencies) {
      if (dependency.source !== null || typeof dependency.path !== "string") {
        continue;
      }
      const packageName = dependency.package ?? dependency.name;
      if (!memberNames.has(packageName) || dependency.req !== `=${expected}`) {
        fail("workspace-path-requirement-mismatch");
      }
    }
  }
};

export const verifyCheckedOutReleaseVersion = (expected, cwd = root) => {
  const encoded = execFileSync(
    "cargo",
    ["metadata", "--format-version", "1", "--no-deps", "--locked"],
    { cwd, encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] },
  );
  const metadata = JSON.parse(encoded);
  const members = new Set(metadata.workspace_members);
  verifyWorkspacePathRequirements(expected, metadata.packages, metadata.workspace_members);
  const versions = metadata.packages
    .filter((pkg) => members.has(pkg.id))
    .map((pkg) => pkg.version);
  verifyReleaseVersion(
    expected,
    {
      swift: read(cwd, "Package.swift"),
      swiftFfi: read(cwd, "packages/swift/Sources/ReallyMeCodec/CallCodecWithRustCAbi.swift"),
      kotlin: read(cwd, "packages/kotlin/build.gradle.kts"),
      android: read(cwd, "packages/kotlin-android/build.gradle.kts"),
      npm: read(cwd, "packages/ts/package.json"),
      npmLock: read(cwd, "packages/ts/package-lock.json"),
    },
    versions,
  );
};

if (process.argv[1] !== undefined && fileURLToPath(import.meta.url) === process.argv[1]) {
  try {
    verifyCheckedOutReleaseVersion(process.argv[2]);
    console.log("release version matches every package manifest");
  } catch (error) {
    const code = error instanceof ReleaseVersionError ? error.code : "release-version-check-failed";
    console.error(`release version check failed: ${code}`);
    process.exit(1);
  }
}
