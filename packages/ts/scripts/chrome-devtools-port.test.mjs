// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { test } from "node:test";
import { readDevToolsActivePort, readDevToolsListeningPort } from "./chrome-devtools-port.mjs";

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

test("Chrome DevTools startup output accepts only a valid loopback port", () => {
  const first = "[startup] DevTools listening on ws://127.0.0.1:";
  const second = "41723/devtools/browser/id\n";
  assert.equal(readDevToolsListeningPort(first), undefined);
  assert.equal(readDevToolsListeningPort(first + second), 41723);
  assert.equal(readDevToolsListeningPort("DevTools listening on ws://127.0.0.1:0/id"), undefined);
  assert.equal(readDevToolsListeningPort("DevTools listening on ws://127.0.0.1:65536/id"), undefined);
  assert.equal(readDevToolsListeningPort("DevTools listening on ws://example.com:41723/id"), undefined);
});
