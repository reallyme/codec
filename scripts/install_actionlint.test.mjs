// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const script = fileURLToPath(new URL("./install_actionlint.sh", import.meta.url));

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), "codec workflow lint # "));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const bin = join(root, "bin");
  const runner = join(root, "runner temp");
  mkdirSync(bin);
  mkdirSync(runner);
  const pathFile = join(root, "github-path");
  writeFileSync(pathFile, "");
  const command = (name, source) => writeFileSync(join(bin, name), `#!/usr/bin/env node\n${source}`, { mode: 0o755 });
  command("uname", 'console.log(process.argv[2] === "-s" ? process.env.LINT_HOST_OS : process.env.LINT_HOST_ARCH);');
  command("curl", `
    const fs = require("node:fs");
    const args = process.argv.slice(2);
    fs.writeFileSync(process.env.LINT_DOWNLOAD_LOG, JSON.stringify(args));
    if (process.env.LINT_DOWNLOAD_FAIL) process.exit(22);
    fs.writeFileSync(args[args.indexOf("--output") + 1], "archive fixture");
  `);
  command("shasum", `
    const fs = require("node:fs");
    fs.writeFileSync(process.env.LINT_CHECKSUM_LOG, fs.readFileSync(0));
    process.exit(process.env.LINT_CHECKSUM_FAIL ? 1 : 0);
  `);
  command("tar", `
    const fs = require("node:fs");
    const path = require("node:path");
    fs.writeFileSync(process.env.LINT_EXTRACT_LOG, "yes");
    if (process.env.LINT_EXTRACT_FAIL) process.exit(2);
    const args = process.argv.slice(2);
    const output = path.join(args[args.indexOf("-C") + 1], "actionlint");
    fs.writeFileSync(output, "#!/usr/bin/env bash\\nprintf '%s\\\\n' '" +
      (process.env.LINT_WRONG_VERSION ? "0.0.0" : "1.7.12") + "'\\n", { mode: 0o755 });
  `);
  const env = {
    ...process.env,
    PATH: `${bin}:${process.env.PATH}`,
    RUNNER_TEMP: runner,
    GITHUB_PATH: pathFile,
    LINT_HOST_OS: "Linux",
    LINT_HOST_ARCH: "x86_64",
    LINT_DOWNLOAD_LOG: join(root, "download"),
    LINT_CHECKSUM_LOG: join(root, "checksum"),
    LINT_EXTRACT_LOG: join(root, "extract"),
  };
  const run = (arguments_ = []) => {
    const result = spawnSync("bash", [script, ...arguments_], { cwd: root, env, encoding: "utf8", timeout: 10_000 });
    assert.equal(result.error, undefined);
    assert.equal(readdirSync(runner).some((name) => name.startsWith("workflow-lint.")), false);
    return result;
  };
  return { root, runner, pathFile, env, run };
}

for (const [os, arch, asset, digest] of [
  ["Linux", "x86_64", "linux_amd64", "8aca8db96f1b94770f1b0d72b6dddcb1ebb8123cb3712530b08cc387b349a3d8"],
  ["Linux", "aarch64", "linux_arm64", "325e971b6ba9bfa504672e29be93c24981eeb1c07576d730e9f7c8805afff0c6"],
  ["Darwin", "x86_64", "darwin_amd64", "5b44c3bc2255115c9b69e30efc0fecdf498fdb63c5d58e17084fd5f16324c644"],
  ["Darwin", "arm64", "darwin_arm64", "aba9ced2dee8d27fecca3dc7feb1a7f9a52caefa1eb46f3271ea66b6e0e6953f"],
]) {
  test(`installs the verified official release for ${os}/${arch}`, (t) => {
    const f = fixture(t);
    f.env.LINT_HOST_OS = os;
    f.env.LINT_HOST_ARCH = arch;
    const result = f.run();
    assert.equal(result.status, 0, result.stderr);
    const args = JSON.parse(readFileSync(f.env.LINT_DOWNLOAD_LOG, "utf8"));
    assert.equal(args.at(-1), `https://github.com/rhysd/actionlint/releases/download/v1.7.12/actionlint_1.7.12_${asset}.tar.gz`);
    assert.ok(args.includes("--fail"));
    assert.ok(args.includes("--retry-max-time"));
    assert.match(readFileSync(f.env.LINT_CHECKSUM_LOG, "utf8"), new RegExp(`^${digest}  .*actionlint_1\\.7\\.12_${asset}\\.tar\\.gz\\n$`, "u"));
    const installPath = readFileSync(f.pathFile, "utf8").trim();
    assert.equal(installPath, join(f.runner, "workflow-lint-bin"));
    assert.equal(existsSync(join(installPath, "actionlint")), true);
  });
}

for (const [variable, message, mayExtract] of [
  ["LINT_DOWNLOAD_FAIL", "pinned release could not be downloaded", false],
  ["LINT_CHECKSUM_FAIL", "pinned release checksum does not match", false],
  ["LINT_EXTRACT_FAIL", "verified release could not be extracted", true],
  ["LINT_WRONG_VERSION", "executable version does not match", true],
]) {
  test(`rejects ${variable} without adding an executable to PATH`, (t) => {
    const f = fixture(t);
    f.env[variable] = "1";
    const result = f.run();
    assert.equal(result.status, 1);
    assert.match(result.stderr, new RegExp(message, "u"));
    assert.equal(readFileSync(f.pathFile, "utf8"), "");
    assert.equal(existsSync(f.env.LINT_EXTRACT_LOG), mayExtract);
    assert.equal(existsSync(join(f.runner, "workflow-lint-bin/actionlint")), false);
  });
}

test("rejects an unsupported host before downloading", (t) => {
  const f = fixture(t);
  f.env.LINT_HOST_OS = "unsupported";
  assert.equal(f.run().status, 1);
  assert.equal(existsSync(f.env.LINT_DOWNLOAD_LOG), false);
});

test("rejects unexpected arguments before downloading", (t) => {
  const f = fixture(t);
  assert.equal(f.run(["--latest"]).status, 1);
  assert.equal(existsSync(f.env.LINT_DOWNLOAD_LOG), false);
});

test("rejects a symlinked installation directory", (t) => {
  const f = fixture(t);
  const other = join(f.root, "other-bin");
  mkdirSync(other);
  symlinkSync(other, join(f.runner, "workflow-lint-bin"));
  const result = f.run();
  assert.equal(result.status, 1);
  assert.match(result.stderr, /installation directory must not be a symlink/u);
  assert.equal(readFileSync(f.pathFile, "utf8"), "");
  assert.equal(existsSync(join(other, "actionlint")), false);
});
