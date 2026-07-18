// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const script = fileURLToPath(new URL("./verify_swift_release_artifact.mjs", import.meta.url));

test("Swift release verifier accepts matching archive, sidecar, manifest, and version", () => {
  const root = mkdtempSync(join(tmpdir(), "reallyme-swift-release-"));
  try {
    const archive = join(root, "artifact.zip");
    const sidecar = join(root, "artifact.checksum");
    const manifest = join(root, "Package.swift");
    const archiveBytes = Buffer.from("deterministic Swift artifact fixture", "utf8");
    const checksum = createHash("sha256").update(archiveBytes).digest("hex");
    writeFileSync(archive, archiveBytes);
    writeFileSync(sidecar, `${checksum}\n`);
    writeFileSync(
      manifest,
      `let ffiArtifactChecksum = "${checksum}"
let ffiArtifactVersion = "0.2.0"
let ffiArtifactLocalPathOverride = ""
.binaryTarget(
    name: "ReallyMeCodecFFI",
    url: "https://github.com/reallyme/codec/releases/download/v\\(ffiArtifactVersion)/ReallyMeCodecFFI.xcframework.zip",
    checksum: ffiArtifactChecksum
)
`,
    );
    assert.doesNotThrow(() => {
      execFileSync(process.execPath, [script, archive, sidecar, manifest, "0.2.0"], {
        stdio: "pipe",
      });
    });
  } finally {
    rmSync(root, { force: true, recursive: true });
  }
});

test("Swift release verifier recomputes bytes and rejects a forged sidecar", () => {
  const root = mkdtempSync(join(tmpdir(), "reallyme-swift-release-"));
  try {
    const archive = join(root, "artifact.zip");
    const sidecar = join(root, "artifact.checksum");
    const manifest = join(root, "Package.swift");
    writeFileSync(archive, "not a valid zip");
    writeFileSync(sidecar, `${"0".repeat(64)}\n`);
    writeFileSync(
      manifest,
      `let ffiArtifactChecksum = "${"0".repeat(64)}"
let ffiArtifactVersion = "0.2.0"
let ffiArtifactLocalPathOverride = ""
.binaryTarget(
    name: "ReallyMeCodecFFI",
    url: "https://github.com/reallyme/codec/releases/download/v\\(ffiArtifactVersion)/ReallyMeCodecFFI.xcframework.zip",
    checksum: ffiArtifactChecksum
)
`,
    );
    assert.throws(() => {
      execFileSync(process.execPath, [script, archive, sidecar, manifest, "0.2.0"], {
        stdio: "pipe",
      });
    });
  } finally {
    rmSync(root, { force: true, recursive: true });
  }
});

test("Swift release verifier rejects unused manifest checksum variables", () => {
  const root = mkdtempSync(join(tmpdir(), "reallyme-swift-release-"));
  try {
    const archive = join(root, "artifact.zip");
    const sidecar = join(root, "artifact.checksum");
    const manifest = join(root, "Package.swift");
    const archiveBytes = Buffer.from("deterministic Swift artifact fixture", "utf8");
    const checksum = createHash("sha256").update(archiveBytes).digest("hex");
    writeFileSync(archive, archiveBytes);
    writeFileSync(sidecar, `${checksum}\n`);
    writeFileSync(
      manifest,
      `let ffiArtifactChecksum = "${checksum}"
let ffiArtifactVersion = "0.2.0"
let ffiArtifactLocalPathOverride = ""
.binaryTarget(
    name: "ReallyMeCodecFFI",
    url: "https://github.com/reallyme/codec/releases/download/v0.2.0/ReallyMeCodecFFI.xcframework.zip",
    checksum: "${checksum}"
)
`,
    );
    assert.throws(() => {
      execFileSync(process.execPath, [script, archive, sidecar, manifest, "0.2.0"], {
        stdio: "pipe",
      });
    });
  } finally {
    rmSync(root, { force: true, recursive: true });
  }
});
