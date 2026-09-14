// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, realpathSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { request } from "node:http";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath, pathToFileURL } from "node:url";
import { startStaticServer } from "../packages/ts/scripts/browser-test-server.mjs";

const repositoryRoot = fileURLToPath(new URL("..", import.meta.url));
const posixHost = process.platform !== "win32";

const fixture = (t) => {
  const root = mkdtempSync(join(tmpdir(), "codec tooling # "));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "bin"));
  return root;
};

const copy = (root, path) => {
  const destination = join(root, path);
  mkdirSync(dirname(destination), { recursive: true });
  cpSync(join(repositoryRoot, path), destination, { recursive: true });
  return destination;
};

const command = (root, name, source) => {
  writeFileSync(join(root, "bin", name), `#!/usr/bin/env node\n${source}`, { mode: 0o700 });
};

const environment = (root) => ({
  ...process.env,
  PATH: `${join(root, "bin")}:${process.env.PATH}`,
  TOOLING_FIXTURE: root,
  TMPDIR: root,
});

for (const [tamperedPath, expectedFailure] of [
  [
    "scripts/check_release_readiness.mjs",
    "local checker does not match the reviewed repository policy pin",
  ],
  [
    "scripts/release-readiness/core.mjs",
    "vendored core does not match the reviewed upstream pin",
  ],
]) {
  test(`pinned release readiness rejects tampered ${tamperedPath}`, (t) => {
    const root = fixture(t);
    const runner = copy(root, "scripts/run_pinned_release_readiness.mjs");
    copy(root, "scripts/check_release_readiness.mjs");
    const core = copy(root, "scripts/release-readiness/core.mjs");
    const target = join(root, tamperedPath);
    writeFileSync(target, `${readFileSync(target, "utf8")}\n// tampered\n`);

    const result = spawnSync(process.execPath, [runner], {
      cwd: root,
      encoding: "utf8",
      timeout: 10_000,
    });

    assert.equal(result.error, undefined);
    assert.equal(result.status, 1);
    assert.match(result.stderr, new RegExp(expectedFailure, "u"));
    assert.equal(existsSync(core), true);
  });
}

for (const succeeds of [false, true]) {
  test(`publish retries ${succeeds ? "accept a later success" : "fail after exhaustion"}`, { skip: !posixHost }, (t) => {
    const root = fixture(t);
    const script = copy(root, "scripts/publish_crates_in_order.mjs");
    const preload = join(root, "skip-wait.mjs");
    // Keep the real retry loop, but omit wall-clock sleeps in the child process.
    writeFileSync(preload, 'Atomics.wait = () => "timed-out";\n');
    command(root, "cargo", `
      const fs = require("node:fs");
      const path = require("node:path");
      fs.writeFileSync(path.join(process.env.TOOLING_FIXTURE, "working-directory"), fs.realpathSync(process.cwd()));
      if (process.argv[2] === "metadata") {
        console.log(JSON.stringify({ packages: [{ name: "fixture", version: "0.2.3", publish: null, dependencies: [] }] }));
      } else {
        const counter = path.join(process.env.TOOLING_FIXTURE, "attempts");
        const count = fs.existsSync(counter) ? Number(fs.readFileSync(counter, "utf8")) + 1 : 1;
        fs.writeFileSync(counter, String(count));
        if (${succeeds} && count === 3) process.exit(0);
        console.error("too many requests; try again after Thu, 01 Jan 1970 00:00:00 GMT");
        process.exit(1);
      }
    `);
    const result = spawnSync(process.execPath, ["--import", pathToFileURL(preload).href, script, "publish"], {
      cwd: dirname(root), env: environment(root), encoding: "utf8", timeout: 10_000,
    });
    assert.equal(result.error, undefined);
    assert.equal(result.status, succeeds ? 0 : 1, result.stderr);
    assert.equal(readFileSync(join(root, "working-directory"), "utf8"), realpathSync(root));
    assert.equal(Number(readFileSync(join(root, "attempts"), "utf8")), succeeds ? 3 : 12);
    if (!succeeds) assert.match(result.stderr, /publish retry limit exhausted/u);
  });
}

test("protobuf hardening supports checkout paths containing spaces and URL characters", (t) => {
  const root = fixture(t);
  for (const path of [
    "scripts/redact_codec_proto_debug.mjs", "scripts/codec_proto_sensitivity.mjs",
    "crates/proto/proto", "crates/proto/src/generated", "packages/ts/src/proto/generated", "gen",
  ]) copy(root, path);
  const result = spawnSync(process.execPath, [join(root, "scripts/redact_codec_proto_debug.mjs"), "--check-idempotent"], {
    cwd: root, encoding: "utf8", timeout: 30_000,
  });
  assert.equal(result.error, undefined);
  assert.equal(result.status, 0, result.stderr);
});

test("ABI test rejects unrelated build failures and cleans private temporary files", { skip: !posixHost }, (t) => {
  const root = fixture(t);
  const script = copy(root, "scripts/test_ffi_abi_release_artifact.sh");
  command(root, "cargo", 'console.error("network unavailable"); process.exit(1);');
  const result = spawnSync("bash", [script], { cwd: root, env: environment(root), encoding: "utf8" });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /failed before verifying the panic=unwind requirement/u);
  assert.deepEqual(readdirSync(root).filter((name) => name.startsWith("reallyme-ffi-abi.")), []);
});

test("ABI test stops immediately if private temporary storage cannot be created", { skip: !posixHost }, (t) => {
  const root = fixture(t);
  const script = copy(root, "scripts/test_ffi_abi_release_artifact.sh");
  command(root, "mktemp", "process.exit(1);");
  const result = spawnSync("bash", [script], { cwd: root, env: environment(root), encoding: "utf8" });
  assert.equal(result.status, 1);
  assert.equal(result.stderr, "");
});

for (const android of [false, true]) {
  test(`${android ? "Android" : "Kotlin"} builder stages this checkout's fresh artifacts from another working directory`, {
    skip: !["darwin", "linux"].includes(process.platform),
  }, (t) => {
    const root = fixture(t);
    const scriptName = android ? "build_android_native_resources.sh" : "build_kotlin_native_resource.sh";
    const script = copy(root, `scripts/${scriptName}`);
    copy(root, "scripts/stage_kotlin_native_resource.mjs");
    writeFileSync(join(root, "Cargo.toml"), "[workspace]\n");
    const elsewhere = join(root, "elsewhere");
    mkdirSync(elsewhere);
    command(root, "cargo", `
      const fs = require("node:fs");
      const path = require("node:path");
      const args = process.argv.slice(2);
      const option = (name) => args.includes(name) ? args[args.indexOf(name) + 1] : undefined;
      const manifest = option("--manifest-path") ?? path.join(process.cwd(), "Cargo.toml");
      if (!fs.existsSync(manifest)) process.exit(101);
      const targetDir = option("--target-dir") ?? process.env.CARGO_TARGET_DIR;
      const target = option("--target");
      const output = path.join(targetDir, target ?? "", "release");
      fs.mkdirSync(output, { recursive: true });
      const name = target || process.platform === "linux" ? "libreallyme_codec_ffi.so" : "libreallyme_codec_ffi.dylib";
      fs.writeFileSync(path.join(output, name), "fresh fixture library");
    `);
    command(root, "rustup", "process.exit(0);");
    const ndk = join(root, "ndk");
    const toolchain = join(ndk, "toolchains/llvm/prebuilt", process.platform === "darwin" ? "darwin-x86_64" : "linux-x86_64", "bin");
    mkdirSync(toolchain, { recursive: true });
    writeFileSync(join(toolchain, "llvm-strip"), "#!/bin/sh\nexit 0\n", { mode: 0o700 });
    const result = spawnSync("bash", [script], {
      cwd: elsewhere,
      env: { ...environment(root), ANDROID_NDK_HOME: ndk, CARGO_TARGET_DIR: join(root, "other-target") },
      encoding: "utf8", timeout: 15_000,
    });
    assert.equal(result.error, undefined);
    assert.equal(result.status, 0, result.stderr);
    const directories = android
      ? ["arm64-v8a", "armeabi-v7a", "x86_64", "x86"].map((abi) => join(root, "packages/kotlin-android/build/generated/android-jniLibs", abi))
      : [join(root, "build/kotlin-native-resources/me/really/codec/native", `${process.platform === "darwin" ? "macos" : "linux"}-${process.arch === "arm64" ? "aarch64" : "x86_64"}`)];
    const libraryName = android || process.platform === "linux" ? "libreallyme_codec_ffi.so" : "libreallyme_codec_ffi.dylib";
    for (const directory of directories) {
      assert.equal(readFileSync(join(directory, libraryName), "utf8"), "fresh fixture library");
    }
    assert.equal(existsSync(join(root, "packages/kotlin/native")), false);
  });
}

test("browser server exposes only test assets and rejects malformed URLs, rebinding hosts, and escaping symlinks", { skip: !posixHost }, async (t) => {
  const root = fixture(t);
  mkdirSync(join(root, "dist"));
  writeFileSync(join(root, "dist/index.js"), "export const fixture = true;");
  writeFileSync(join(root, ".npmrc"), "private fixture");
  writeFileSync(join(root, "outside.js"), "private fixture");
  symlinkSync(join(root, "outside.js"), join(root, "dist/escape.js"));
  const { server, port } = await startStaticServer({ packageDirectory: root, testPage: () => "test page" });
  try {
    for (const [path, status] of [
      ["/browser-wasm-test.html", 200], ["/dist/index.js", 200], ["/.npmrc", 404],
      ["/outside.js", 404], ["/dist/escape.js", 404], ["/%", 400], ["/dist/%2e%2e/.npmrc", 404],
    ]) {
      const response = await fetch(`http://127.0.0.1:${port}${path}`);
      assert.equal(response.status, status, path);
      await response.arrayBuffer();
    }
    const status = await new Promise((resolveStatus, rejectStatus) => {
      const req = request({ host: "127.0.0.1", port, path: "/dist/index.js", headers: { Host: "untrusted.example" } }, (response) => {
        response.resume();
        resolveStatus(response.statusCode);
      });
      req.once("error", rejectStatus);
      req.end();
    });
    assert.equal(status, 403);
  } finally {
    await new Promise((resolveClose) => server.close(resolveClose));
  }
});
