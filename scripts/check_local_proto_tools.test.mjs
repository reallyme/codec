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
function fixture(t, { remote = false, protoc = "libprotoc 36.2", swift = "protoc-gen-swift 1.41.0", es = "protoc-gen-es v2.17.0" } = {}) {
  const root = mkdtempSync(join(tmpdir(), "local protocol # "));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "scripts"));
  mkdirSync(join(root, "bin"));
  const esDirectory = join(root, "packages/ts/node_modules/@bufbuild/protoc-gen-es/bin");
  mkdirSync(esDirectory, { recursive: true });
  cpSync(join(repository, "scripts/check_local_proto_tools.mjs"), join(root, "scripts/check_local_proto_tools.mjs"));
  writeFileSync(join(root, "buf.gen.yaml"), `version: v2\nplugins:\n  - ${remote ? "remote: example.invalid/generator:v1" : "local: protoc-gen-swift"}\n`);
  for (const [path, output] of [[join(root, "bin/protoc"), protoc], [join(root, "bin/protoc-gen-swift"), swift], [join(esDirectory, "protoc-gen-es"), es]]) {
    writeFileSync(path, `#!/usr/bin/env node\nconsole.log(${JSON.stringify(output)});\n`, { mode: 0o700 });
  }
  return spawnSync(process.execPath, [join(root, "scripts/check_local_proto_tools.mjs")], {
    cwd: root, encoding: "utf8", timeout: 60_000,
    env: { ...process.env, PATH: `${join(root, "bin")}:${process.env.PATH ?? ""}` },
  });
}

test("local generation accepts exactly pinned tools in paths with spaces", (t) => {
  const result = fixture(t);
  assert.equal(result.error, undefined);
  assert.equal(result.status, 0, result.stderr);
});
for (const [name, options] of [
  ["remote plugin", { remote: true }],
  ["wrong protoc", { protoc: "libprotoc 37.0" }],
  ["wrong Swift plugin", { swift: "protoc-gen-swift 1.39.0" }],
  ["wrong TypeScript plugin", { es: "protoc-gen-es v2.18.0" }],
]) {
  test(`local generation rejects ${name}`, (t) => {
    const result = fixture(t, options);
    assert.equal(result.error, undefined);
    assert.equal(result.status, 1);
    assert.match(result.stderr, /pinned local tools and local plugins/u);
  });
}
