// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { closeSync, constants, fstatSync, lstatSync, openSync, readSync } from "node:fs";

export class FileSnapshotError extends Error {
  constructor(code) {
    super(code);
    this.name = "FileSnapshotError";
    this.code = code;
  }
}

const fail = (code) => { throw new FileSnapshotError(code); };

const sameFile = (first, second) => second.isFile() &&
  first.dev === second.dev && first.ino === second.ino && first.size === second.size &&
  first.mtimeNs === second.mtimeNs && first.ctimeNs === second.ctimeNs;

/** Keep the validated descriptor and bounded bytes together until the caller finishes. */
class FileSnapshot {
  constructor(path, descriptor, status, bytes) {
    this.path = path;
    this.descriptor = descriptor;
    this.status = status;
    this.bytes = bytes;
  }

  assertUnchanged() {
    try {
      if (this.descriptor === undefined ||
          !sameFile(this.status, fstatSync(this.descriptor, { bigint: true })) ||
          !sameFile(this.status, lstatSync(this.path, { bigint: true }))) {
        fail("file-changed");
      }
    } catch (error) {
      if (error instanceof FileSnapshotError) throw error;
      fail("file-changed");
    }
  }

  close() {
    const descriptor = this.descriptor;
    this.descriptor = undefined;
    this.bytes.fill(0);
    if (descriptor !== undefined) {
      try { closeSync(descriptor); } catch { fail("file-close-failed"); }
    }
  }
}

export const openRegularFileSnapshot = (path, maximumBytes) => {
  if (!Number.isSafeInteger(maximumBytes) || maximumBytes <= 0) fail("invalid-file-limit");
  if (!Number.isInteger(constants.O_NOFOLLOW)) fail("no-follow-unavailable");
  let descriptor;
  let bytes;
  try {
    // A descriptor prevents pathname replacement from redirecting later reads or chmods.
    // Nonblocking opens also let us reject a FIFO without waiting for a writer.
    descriptor = openSync(path, constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK);
    const status = fstatSync(descriptor, { bigint: true });
    if (!status.isFile() || status.size <= 0n || status.size > BigInt(maximumBytes)) {
      fail("invalid-file-size-or-type");
    }
    const length = Number(status.size);
    bytes = Buffer.alloc(length);
    let offset = 0;
    while (offset < length) {
      const count = readSync(descriptor, bytes, offset, length - offset, null);
      if (count === 0) fail("file-changed");
      offset += count;
    }
    const snapshot = new FileSnapshot(path, descriptor, status, bytes);
    snapshot.assertUnchanged();
    descriptor = undefined;
    bytes = undefined;
    return snapshot;
  } catch (error) {
    if (error instanceof FileSnapshotError) throw error;
    if (error !== null && typeof error === "object" && "code" in error && error.code === "ENOENT") {
      fail("file-missing");
    }
    fail("file-unavailable");
  } finally {
    bytes?.fill(0);
    if (descriptor !== undefined) {
      try { closeSync(descriptor); } catch { fail("file-close-failed"); }
    }
  }
};
