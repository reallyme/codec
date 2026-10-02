// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { readDevToolsActivePort } from "./chrome-devtools-port.mjs";

test("Chrome DevTools port file accepts a valid port and rejects malformed values", (context) => {
  const directory = mkdtempSync(join(tmpdir(), "reallyme-codec-port-test-"));
  context.after(() => rmSync(directory, { recursive: true, force: true }));
  const path = join(directory, "DevToolsActivePort");

  assert.equal(readDevToolsActivePort(path), undefined);
  writeFileSync(path, "41723\n/devtools/browser/id\n");
  assert.equal(readDevToolsActivePort(path), 41723);
  for (const invalid of ["", "0\n", "65536\n", "41723 extra\n", "-1\n"]) {
    writeFileSync(path, invalid);
    assert.throws(() => readDevToolsActivePort(path), /invalid DevTools port/u);
  }
});
