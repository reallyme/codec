// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { strict as assert } from "node:assert";
import test from "node:test";

import {
  ReleaseVersionError,
  verifyReleaseVersion,
  verifyWorkspacePathRequirements,
} from "./verify_release_version.mjs";

const sources = Object.freeze({
  swift: 'let packageVersion = "0.3.1"\n',
  swiftFfi: 'private let expectedCodecPackageMajor: UInt32 = 0\nprivate let expectedCodecPackageMinor: UInt32 = 3\nprivate let expectedCodecPackagePatch: UInt32 = 1\n',
  kotlin: 'version = "0.3.1"\n',
  android: 'version = "0.3.1"\n',
  npm: '{"version":"0.3.1"}',
  npmLock: '{"version":"0.3.1","packages":{"":{"version":"0.3.1"}}}',
});

test("matching package versions are accepted", () => {
  assert.doesNotThrow(() => verifyReleaseVersion("0.3.1", sources, ["0.3.1", "0.3.1"]));
});

test("mistyped release and stale internal package versions fail closed", () => {
  for (const [version, manifests, workspaceVersions, reason] of [
    ["0.3.2", sources, ["0.3.1"], "workspace-version-mismatch"],
    ["0.3.1", sources, ["0.3.1", "0.2.9"], "workspace-version-mismatch"],
    ["0.3.1", { ...sources, swift: 'let packageVersion = "0.2.9"\n' }, ["0.3.1"], "swift-version-mismatch"],
    ["0.3.1", { ...sources, swiftFfi: sources.swiftFfi.replace("Minor: UInt32 = 3", "Minor: UInt32 = 2") }, ["0.3.1"], "swift-ffi-version-mismatch"],
    ["0.3.1", { ...sources, npmLock: '{"version":"0.3.1","packages":{"":{"version":"0.2.9"}}}' }, ["0.3.1"], "npm-lock-root-version-mismatch"],
  ]) {
    assert.throws(
      () => verifyReleaseVersion(version, manifests, workspaceVersions),
      (error) => error instanceof ReleaseVersionError && error.code === reason,
    );
  }
});

test("workspace path dependencies require the exact release version", () => {
  const packages = [
    { id: "base", name: "reallyme-base", version: "0.3.1", dependencies: [] },
    {
      id: "codec",
      name: "reallyme-codec",
      version: "0.3.1",
      dependencies: [{ source: null, path: "/workspace/base", name: "reallyme-base", req: "=0.3.1" }],
    },
  ];
  assert.doesNotThrow(() => verifyWorkspacePathRequirements("0.3.1", packages, ["base", "codec"]));
  for (const req of ["^0.3.1", "=0.2.9"]) {
    const stale = structuredClone(packages);
    stale[1].dependencies[0].req = req;
    assert.throws(
      () => verifyWorkspacePathRequirements("0.3.1", stale, ["base", "codec"]),
      (error) => error instanceof ReleaseVersionError && error.code === "workspace-path-requirement-mismatch",
    );
  }
});
