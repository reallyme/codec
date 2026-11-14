// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import {
  cpSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  realpathSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";

const repositoryRoot = fileURLToPath(new URL("../", import.meta.url));
const runnerRelativePath = "scripts/run_pinned_release_readiness.mjs";
const checkerRelativePath = "scripts/check_release_readiness.mjs";
const coreRelativePath = ".release-readiness/core.mjs";
const packageRoot = process.env.REALLYME_RELEASE_READINESS_ROOT ?? join(repositoryRoot, ".release-readiness");
const checkerPinDeclaration = /const LOCAL_CHECKER_SHA256 =\n  "[0-9a-f]{64}";/u;
const digest = (bytes) => createHash("sha256").update(bytes).digest("hex");

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), "codec readiness # "));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const runner = join(root, runnerRelativePath);
  const checker = join(root, checkerRelativePath);
  const core = join(root, coreRelativePath);
  mkdirSync(dirname(core), { recursive: true });
  mkdirSync(dirname(runner), { recursive: true });
  cpSync(join(repositoryRoot, runnerRelativePath), runner);
  cpSync(join(packageRoot, "core.mjs"), core);
  cpSync(join(packageRoot, "package.json"), join(root, ".release-readiness/package.json"));
  mkdirSync(join(root, ".release-readiness/scripts"), { recursive: true });
  cpSync(join(packageRoot, "scripts/run-consumer-check.mjs"), join(root, ".release-readiness/scripts/run-consumer-check.mjs"));
  const initialized = spawnSync("git", ["init", "--quiet", "--initial-branch=main"], { cwd: root, encoding: "utf8" });
  assert.equal(initialized.status, 0, initialized.stderr);
  // Isolate pin tests from changes to repository policy. A separate baseline
  // test below checks the real checkout, so a stale pin still fails clearly.
  const checkerSource = [
    'import { writeFileSync } from "node:fs";',
    'writeFileSync("checker-executed", "yes");',
    'console.log(JSON.stringify({ arguments: process.argv.slice(2), cwd: process.cwd() }));',
    'process.exit(Number(process.env.CHECKER_EXIT_STATUS ?? "0"));',
    "",
  ].join("\n");
  writeFileSync(checker, checkerSource);
  writeFileSync(runner, readFileSync(runner, "utf8").replace(
    checkerPinDeclaration,
    `const LOCAL_CHECKER_SHA256 =\n  "${digest(checkerSource)}";`,
  ));
  const preload = join(root, "upstream.mjs");
  writeFileSync(preload, `
    import assert from "node:assert/strict";
    import fs from "node:fs";
    import { readFileSync, writeFileSync } from "node:fs";
    import { syncBuiltinESMExports } from "node:module";
    if (process.env.PIN_FILE_CHANGE !== undefined) {
      const stat = fs.fstatSync;
      const path = ${JSON.stringify(checker)};
      const checkerInode = fs.statSync(path).ino;
      let changed = false;
      fs.fstatSync = (...arguments_) => {
        const status = stat(...arguments_);
        if (!changed && status.ino === checkerInode) {
          changed = true;
          const original = readFileSync(path);
          writeFileSync(path, process.env.PIN_FILE_CHANGE === "grow"
            ? Buffer.concat([original, Buffer.from("extra")])
            : original.subarray(0, original.length - 1));
        }
        return status;
      };
      syncBuiltinESMExports();
    }
    globalThis.fetch = async (url, options) => {
      writeFileSync(${JSON.stringify(join(root, "network-used"))}, "yes");
      assert.match(url, /release-readiness\\/[0-9a-f]{40}\\/core\\.mjs$/u);
      assert.equal(options.redirect, "error");
      assert.equal(options.cache, "no-store");
      assert.ok(options.signal instanceof AbortSignal);
      const core = readFileSync(${JSON.stringify(core)});
      switch (process.env.PIN_UPSTREAM_MODE) {
        case "valid": return new Response(core);
        case "valid-length": return new Response(core, { headers: { "content-length": String(core.length) } });
        case "http-error": return new Response(null, { status: 503 });
        case "no-body": return new Response(null);
        case "bad-length": return new Response(core, { headers: { "content-length": "1x" } });
        case "oversized-length": return new Response(core, { headers: { "content-length": "262145" } });
        case "empty": return new Response(new Uint8Array());
        case "oversized-body": return new Response(new Uint8Array(262145));
        case "chunked-overflow": return new Response(new ReadableStream({
          start(controller) {
            controller.enqueue(new Uint8Array(262144));
            controller.enqueue(new Uint8Array(1));
            controller.close();
          }
        }));
        case "corrupt": return new Response("unreviewed upstream bytes");
        case "read-error": return new Response(new ReadableStream({
          start(controller) { controller.error(new TypeError("fixture stream failure")); }
        }));
        default: throw new TypeError("fixture network unavailable");
      }
    };
  `);
  const env = { ...process.env };
  delete env.CI;
  delete env.REALLYME_RELEASE_READINESS_ROOT;
  delete env.RELEASE_READINESS_CORE_URL;
  delete env.RELEASE_READINESS_ENFORCED_VERSION;
  delete env.RELEASE_READINESS_SOURCE_POLICY_FD;
  const run = (arguments_ = [], overrides = {}) => {
    const result = spawnSync(process.execPath, [
      "--import", pathToFileURL(preload).href, runner, ...arguments_,
    ], {
      cwd: dirname(root),
      encoding: "utf8",
      env: { ...env, ...overrides },
      timeout: 10_000,
    });
    assert.equal(result.error, undefined);
    return result;
  };
  return { root, runner, checker, core, preload, run };
}

test("the checkout has matching pins without accessing the network or running policy", (t) => {
  const f = fixture(t);
  const result = spawnSync(process.execPath, [
    "--import", pathToFileURL(f.preload).href,
    join(repositoryRoot, runnerRelativePath), "--check-pins",
  ], { cwd: f.root, encoding: "utf8", timeout: 10_000 });
  assert.equal(result.error, undefined);
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /release readiness pins verified/u);
  assert.equal(existsSync(join(f.root, "network-used")), false);
  assert.equal(existsSync(join(f.root, "checker-executed")), false);
});

test("the direct checker rejects a stale policy pin before running repository checks", (t) => {
  const f = fixture(t);
  for (const path of [checkerRelativePath, "scripts/verify_release_version.mjs", "scripts/codec_proto_sensitivity.mjs"]) {
    cpSync(join(repositoryRoot, path), join(f.root, path));
  }
  // The core requires a tracked checkout. No commit, identity, or remote is
  // needed to exercise the real local entry point's integrity boundary.
  for (const arguments_ of [
    ["init", "--quiet", "--initial-branch=main"],
    ["add", coreRelativePath],
  ]) {
    const result = spawnSync("git", arguments_, { cwd: f.root, encoding: "utf8" });
    assert.equal(result.error, undefined);
    assert.equal(result.status, 0, result.stderr);
  }
  const result = spawnSync(process.execPath, [f.checker], {
    cwd: f.root, encoding: "utf8", timeout: 10_000,
    env: { ...process.env, REALLYME_RELEASE_READINESS_ROOT: join(f.root, ".release-readiness") },
  });
  assert.equal(result.error, undefined);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /local checker does not match the reviewed repository policy pin/u);
  assert.equal(existsSync(join(f.root, "checker-executed")), false);
});

test("the direct checker authenticates the core before executing its top-level code", (t) => {
  const f = fixture(t);
  cpSync(join(repositoryRoot, checkerRelativePath), f.checker);
  writeFileSync(f.core, `${readFileSync(f.core, "utf8")}\nprocess.exit(94);\n`);
  const result = spawnSync(process.execPath, [f.checker], {
    cwd: f.root, encoding: "utf8", timeout: 10_000,
    env: { ...process.env, REALLYME_RELEASE_READINESS_ROOT: join(f.root, ".release-readiness") },
  });
  assert.equal(result.error, undefined);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /package core does not match the reviewed upstream pin/u);
});

test("an alternate core URL cannot replace the authenticated package core", (t) => {
  const f = fixture(t);
  const alternate = join(f.root, "alternate.mjs");
  cpSync(f.core, alternate);
  const result = f.run(["--check-pins"], { RELEASE_READINESS_CORE_URL: pathToFileURL(alternate).href });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /supplied core URL does not identify the authenticated package core/u);
  assert.equal(existsSync(join(f.root, "checker-executed")), false);
});

test("verified local bytes run offline from any working directory and forward policy modes", (t) => {
  const f = fixture(t);
  const result = f.run(["--generated-freshness", "--release-packages"]);
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(JSON.parse(result.stdout), {
    arguments: ["--generated-freshness", "--release-packages"],
    cwd: realpathSync(f.root),
  });
  assert.equal(existsSync(join(f.root, "network-used")), false);
  assert.equal(existsSync(join(f.root, "checker-executed")), true);
});

test("checker failures preserve their exit status", (t) => {
  const f = fixture(t);
  assert.equal(f.run([], { CHECKER_EXIT_STATUS: "37" }).status, 37);
});

for (const [extension, language] of [["rs", "rust"], ["ts", "typescript"], ["swift", "swift"], ["kt", "kotlin"]]) {
  test(`the official runner requires proof of the shared ${language} policy`, (t) => {
    const f = fixture(t);
    writeFileSync(join(f.root, `source.${extension}`), "// synthetic source\n");
    const tracked = spawnSync("git", ["add", `source.${extension}`], { cwd: f.root, encoding: "utf8" });
    assert.equal(tracked.status, 0, tracked.stderr);
    const result = f.run();
    assert.equal(result.status, 1);
    assert.match(result.stderr, new RegExp(`did not enforce the shared ${language} source policy`, "u"));
  });
}

test("pin-only verification does not execute the checker", (t) => {
  const f = fixture(t);
  assert.equal(f.run(["--check-pins"]).status, 0);
  assert.equal(existsSync(join(f.root, "checker-executed")), false);
});

for (const [key, message, limit] of [
  ["checker", "local checker", 524_288],
  ["core", "package core", 262_144],
]) {
  test(`tampered ${key} is rejected before execution or upstream access`, (t) => {
    const f = fixture(t);
    writeFileSync(f[key], `${readFileSync(f[key], "utf8")}\n// tampered\n`);
    const result = f.run(["--verify-upstream"]);
    assert.equal(result.status, 1);
    assert.match(result.stderr, new RegExp(`${message} does not match the reviewed`, "u"));
    assert.equal(existsSync(join(f.root, "network-used")), false);
    assert.equal(existsSync(join(f.root, "checker-executed")), false);
  });

  for (const scenario of ["missing", "empty", "oversized", "directory", "symlink"]) {
    test(`${key} rejects ${scenario} input`, { skip: scenario === "symlink" && process.platform === "win32" }, (t) => {
      const f = fixture(t);
      const path = f[key];
      if (scenario === "empty" || scenario === "oversized") {
        writeFileSync(path, Buffer.alloc(scenario === "empty" ? 0 : limit + 1));
      } else {
        const bytes = readFileSync(path);
        rmSync(path);
        if (scenario === "directory") mkdirSync(path);
        if (scenario === "symlink") {
          const target = join(f.root, "linked-input");
          writeFileSync(target, bytes);
          symlinkSync(target, path);
        }
      }
      const result = f.run(["--check-pins"]);
      assert.equal(result.status, 1);
      const reason = scenario === "missing" || scenario === "symlink" ? "inputs are missing or inaccessible"
        : scenario === "empty" || scenario === "oversized" ? `${message} size is outside`
          : `${message} must be a regular file`;
      assert.match(result.stderr, new RegExp(reason, "u"));
      assert.equal(existsSync(join(f.root, "checker-executed")), false);
    });
  }
}

for (const change of ["grow", "shrink"]) {
  test(`a checker that changes size during its read fails closed: ${change}`, (t) => {
    const f = fixture(t);
    const result = f.run(["--check-pins"], { PIN_FILE_CHANGE: change });
    assert.equal(result.status, 1);
    assert.match(result.stderr, /local checker size is outside the accepted boundary/u);
  });
}

test("a FIFO input is rejected without blocking", { skip: process.platform === "win32" }, (t) => {
  const f = fixture(t);
  rmSync(f.checker);
  const fifo = spawnSync("mkfifo", [f.checker], { encoding: "utf8", timeout: 10_000 });
  assert.equal(fifo.error, undefined);
  assert.equal(fifo.status, 0, fifo.stderr);
  const result = f.run(["--check-pins"]);
  assert.equal(result.status, 1);
  assert.match(result.stderr, /local checker must be a regular file/u);
});

for (const arguments_ of [
  ["--unknown"],
  ["--check-pins", "--check-pins"],
  ["--refresh-checker-pin", "--verify-upstream"],
  ["--check-pins", "--generated-freshness"],
  ["--check-pins", "--release-packages"],
]) {
  test(`invalid runner arguments fail closed: ${arguments_.join(" ")}`, (t) => {
    const f = fixture(t);
    assert.equal(f.run(arguments_).status, 1);
    assert.equal(existsSync(join(f.root, "checker-executed")), false);
    assert.equal(existsSync(join(f.root, "network-used")), false);
  });
}

test("explicit local refresh changes only the checker pin and is idempotent", (t) => {
  const f = fixture(t);
  const originalRunner = readFileSync(f.runner, "utf8");
  writeFileSync(f.checker, `${readFileSync(f.checker, "utf8")}\n// reviewed policy edit\n`);
  assert.equal(f.run(["--check-pins"]).status, 1);
  const result = f.run(["--refresh-checker-pin"]);
  assert.equal(result.status, 0, result.stderr);
  const refreshedRunner = readFileSync(f.runner, "utf8");
  assert.equal(refreshedRunner, originalRunner.replace(
    checkerPinDeclaration,
    `const LOCAL_CHECKER_SHA256 =\n  "${digest(readFileSync(f.checker))}";`,
  ));
  assert.equal(f.run(["--check-pins"]).status, 0);
  assert.equal(f.run(["--refresh-checker-pin"]).status, 0);
  assert.equal(readFileSync(f.runner, "utf8"), refreshedRunner);
  assert.equal(existsSync(join(f.root, "checker-executed")), false);
  assert.equal(existsSync(join(f.root, "network-used")), false);
});

test("CI cannot refresh a checker pin", (t) => {
  const f = fixture(t);
  const original = readFileSync(f.runner);
  const result = f.run(["--refresh-checker-pin"], { CI: "true" });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /forbidden in CI/u);
  assert.deepEqual(readFileSync(f.runner), original);
});

test("checker refresh cannot approve a changed upstream core", (t) => {
  const f = fixture(t);
  const original = readFileSync(f.runner);
  writeFileSync(f.core, "unreviewed core");
  assert.equal(f.run(["--refresh-checker-pin"]).status, 1);
  assert.deepEqual(readFileSync(f.runner), original);
});

for (const mode of ["valid", "valid-length"]) {
  test(`explicit upstream verification accepts ${mode} pinned bytes`, (t) => {
    const f = fixture(t);
    const result = f.run(["--verify-upstream", "--check-pins"], { PIN_UPSTREAM_MODE: mode });
    assert.equal(result.status, 0, result.stderr);
    assert.equal(existsSync(join(f.root, "network-used")), true);
    assert.equal(existsSync(join(f.root, "checker-executed")), false);
  });
}

for (const [mode, reason] of [
  ["offline", "could not be fetched"],
  ["http-error", "invalid response"],
  ["no-body", "invalid response"],
  ["bad-length", "invalid content length"],
  ["oversized-length", "length is outside the accepted boundary"],
  ["empty", "core is empty"],
  ["oversized-body", "exceeds the accepted boundary"],
  ["chunked-overflow", "exceeds the accepted boundary"],
  ["corrupt", "digest does not match the reviewed commit"],
  ["read-error", "response could not be read"],
]) {
  test(`explicit upstream verification rejects ${mode} without executing policy`, (t) => {
    const f = fixture(t);
    const result = f.run(["--verify-upstream"], { PIN_UPSTREAM_MODE: mode });
    assert.equal(result.status, 1);
    assert.match(result.stderr, new RegExp(reason, "u"));
    assert.equal(existsSync(join(f.root, "checker-executed")), false);
  });
}
