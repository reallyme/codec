// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { cpSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const repository = fileURLToPath(new URL("../", import.meta.url));
const tools = ["protoc-gen-reallyme-buffa", "protoc-gen-reallyme-buffa-packaging"];

function fixture(t, selected, output, status = 0) {
  const root = mkdtempSync(join(tmpdir(), "codec protocol # "));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "scripts"));
  mkdirSync(join(root, "bin"));
  cpSync(join(repository, "scripts/generate_codec_proto.mjs"), join(root, "scripts/generate_codec_proto.mjs"));
  for (const tool of tools) {
    const text = tool === selected ? output : `${tool} 0.1.1`;
    writeFileSync(join(root, "bin", tool), `#!/usr/bin/env node\nconsole.log(${JSON.stringify(text)}); process.exit(${tool === selected ? status : 0});\n`, { mode: 0o700 });
  }
  return spawnSync(process.execPath, [join(root, "scripts/generate_codec_proto.mjs"), "--check-tools"], {
    cwd: root, encoding: "utf8", timeout: 10_000,
    env: { ...process.env, PATH: `${join(root, "bin")}:${process.env.PATH ?? ""}` },
  });
}

test("both exact published generator identities are accepted in a checkout path with spaces", (t) => {
  const result = fixture(t, undefined, undefined);
  assert.equal(result.error, undefined);
  assert.equal(result.status, 0, result.stderr);
});

for (const tool of tools) {
  for (const [name, output, status] of [
    ["upstream executable identity", "protoc-gen-buffa 0.1.1", 0],
    ["previous generator", `${tool} 0.1.0`, 0],
    ["newer unreviewed generator", `${tool} 0.1.2`, 0],
    ["failed executable with plausible version output", `${tool} 0.1.1`, 1],
  ]) {
    test(`${tool} rejects ${name}`, (t) => {
      const result = fixture(t, tool, output, status);
      assert.equal(result.error, undefined);
      assert.equal(result.status, 1);
      assert.match(result.stderr, /install protoc-gen-reallyme-buffa(?:-packaging)? from its published 0\.1\.1 package/u);
    });
  }
}
