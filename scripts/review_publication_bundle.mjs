// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { openRegularFileSnapshot } from "./open_regular_file_snapshot.mjs";

const MAX_BUNDLE_BYTES = 536_870_912;

export const withReviewedPublicationBundle = async (path, inspect, publish) => {
  const directory = mkdtempSync(join(tmpdir(), "codec-maven-upload-"));
  let snapshot;
  try {
    const privatePath = join(directory, "bundle.zip");
    const source = openRegularFileSnapshot(path, MAX_BUNDLE_BYTES);
    try {
      writeFileSync(privatePath, source.bytes, { flag: "wx", mode: 0o600 });
    } finally {
      source.close();
    }
    // Inspection and upload share one private snapshot. Reopening the original
    // pathname after inspection could upload bytes that the policy never reviewed.
    snapshot = openRegularFileSnapshot(privatePath, MAX_BUNDLE_BYTES);
    await inspect(privatePath);
    snapshot.assertUnchanged();
    return await publish(snapshot.bytes);
  } finally {
    try { snapshot?.close(); } finally { rmSync(directory, { recursive: true, force: true }); }
  }
};
