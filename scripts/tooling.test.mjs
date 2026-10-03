// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
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

test("Swift binary manifest rejects ancestor path components", (t) => {
  const root = fixture(t);
  const script = copy(root, "scripts/prepare_swift_binary_manifest.mjs");
  writeFileSync(join(root, "Package.swift"), [
    'let ffiArtifactChecksum = ""',
    'let ffiArtifactVersion = ""',
    'let ffiArtifactLocalPathOverride = ""',
  ].join("\n"));
  const checksum = "a".repeat(64);
  for (const path of ["artifact/../bundle.zip", "artifact/./bundle.zip"]) {
    const result = spawnSync(process.execPath, [script, "0.3.1", checksum, "--local-artifact-path", path], {
      cwd: root, encoding: "utf8", timeout: 10_000,
    });
    assert.equal(result.error, undefined);
    assert.equal(result.status, 1);
    assert.match(result.stderr, /ancestor or current-directory components/u);
  }
  const accepted = spawnSync(process.execPath, [script, "0.3.1", checksum, "--local-artifact-path", "artifact/bundle.zip"], {
    cwd: root, encoding: "utf8", timeout: 10_000,
  });
  assert.equal(accepted.error, undefined);
  assert.equal(accepted.status, 0, accepted.stderr);
  assert.match(readFileSync(join(root, "Package.swift"), "utf8"), /artifact\/bundle\.zip/u);
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
    writeFileSync(preload, 'Atomics.wait = () => "timed-out"; globalThis.fetch = async () => ({ status: 404 });\n');
    command(root, "cargo", `
      const fs = require("node:fs");
      const path = require("node:path");
      fs.writeFileSync(path.join(process.env.TOOLING_FIXTURE, "working-directory"), fs.realpathSync(process.cwd()));
      if (process.argv[2] === "metadata") {
        console.log(JSON.stringify({ target_directory: path.join(process.env.TOOLING_FIXTURE, "target"), packages: [{ name: "fixture", version: "0.2.3", publish: null, dependencies: [] }] }));
      } else if (process.argv[2] === "package") {
        const directory = path.join(process.env.TOOLING_FIXTURE, "target/package");
        fs.mkdirSync(directory, { recursive: true });
        fs.writeFileSync(path.join(directory, "fixture-0.2.3.crate"), "package fixture");
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

test("crate packaging waits for the preceding workspace release to reach the index", { skip: !posixHost }, (t) => {
  const root = fixture(t);
  const script = copy(root, "scripts/publish_crates_in_order.mjs");
  const preload = join(root, "registry.mjs");
  writeFileSync(preload, 'Atomics.wait = () => "timed-out"; globalThis.fetch = async () => ({ status: 404 });\n');
  command(root, "cargo", `
    const fs = require("node:fs");
    const path = require("node:path");
    const root = process.env.TOOLING_FIXTURE;
    if (process.argv[2] === "metadata") {
      console.log(JSON.stringify({ target_directory: path.join(root, "target"), packages: [
        { name: "base", version: "0.3.1", publish: null, dependencies: [] },
        { name: "fixture", version: "0.3.1", publish: null, dependencies: [
          { name: "base", source: null, path: path.join(root, "base"), req: "=0.3.1" },
        ] },
      ] }));
    } else if (process.argv[2] === "package") {
      const name = process.argv[process.argv.indexOf("-p") + 1];
      if (name === "fixture") {
        const counter = path.join(root, "package-attempts");
        const count = fs.existsSync(counter) ? Number(fs.readFileSync(counter, "utf8")) + 1 : 1;
        fs.writeFileSync(counter, String(count));
        if (count < 3) {
          console.error('failed to select a version for the requirement \`base = "=0.3.1"\`');
          process.exit(101);
        }
      }
      const directory = path.join(root, "target/package");
      fs.mkdirSync(directory, { recursive: true });
      fs.writeFileSync(path.join(directory, name + "-0.3.1.crate"), "package " + name);
    } else {
      const name = process.argv[process.argv.indexOf("-p") + 1];
      fs.appendFileSync(path.join(root, "uploads"), name + "\\n");
    }
  `);
  const result = spawnSync(process.execPath, ["--import", pathToFileURL(preload).href, script, "publish"], {
    cwd: root, env: environment(root), encoding: "utf8", timeout: 10_000,
  });
  assert.equal(result.error, undefined);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(readFileSync(join(root, "package-attempts"), "utf8"), "3");
  assert.equal(readFileSync(join(root, "uploads"), "utf8"), "base\nfixture\n");
});

for (const matches of [false, true]) {
  test(`already published crate ${matches ? "requires a matching" : "rejects a different"} checksum`, { skip: !posixHost }, (t) => {
    const root = fixture(t);
    const script = copy(root, "scripts/publish_crates_in_order.mjs");
    const checksum = createHash("sha256").update("package fixture").digest("hex");
    const registryChecksum = matches ? checksum : "0".repeat(64);
    const preload = join(root, "registry.mjs");
    writeFileSync(preload, `globalThis.fetch = async () => ({ ok: true, status: 200, json: async () => ({ version: { checksum: "${registryChecksum}" } }) });\n`);
    command(root, "cargo", `
      const fs = require("node:fs");
      const path = require("node:path");
      if (process.argv[2] === "metadata") {
        console.log(JSON.stringify({ target_directory: path.join(process.env.TOOLING_FIXTURE, "target"), packages: [{ name: "fixture", version: "0.3.1", publish: null, dependencies: [] }] }));
      } else if (process.argv[2] === "package") {
        const directory = path.join(process.env.TOOLING_FIXTURE, "target/package");
        fs.mkdirSync(directory, { recursive: true });
        fs.writeFileSync(path.join(directory, "fixture-0.3.1.crate"), "package fixture");
      } else {
        fs.writeFileSync(path.join(process.env.TOOLING_FIXTURE, "unexpected-publish"), "yes");
        process.exit(1);
      }
    `);
    const result = spawnSync(process.execPath, ["--import", pathToFileURL(preload).href, script, "publish"], {
      cwd: root, env: environment(root), encoding: "utf8", timeout: 10_000,
    });
    assert.equal(result.error, undefined);
    assert.equal(result.status, matches ? 0 : 1, result.stderr);
    assert.equal(existsSync(join(root, "unexpected-publish")), false);
    if (!matches) assert.match(result.stderr, /published-crate-checksum-mismatch/u);
  });
}

for (const scenario of [
  { name: "already uploaded becomes visible", cargoError: "crate is already uploaded", registryChecksum: "matching", status: 0, queries: 3 },
  { name: "publish index timeout becomes visible", cargoError: "timed out waiting for published package", registryChecksum: "matching", status: 0, queries: 3 },
  { name: "already uploaded has a different checksum", cargoError: "crate is already uploaded", registryChecksum: "different", status: 1, queries: 3 },
  { name: "already uploaded never becomes visible", cargoError: "crate is already uploaded", registryChecksum: "missing", status: 1, queries: 13 },
]) {
  test(`publish verification ${scenario.name}`, { skip: !posixHost }, (t) => {
    const root = fixture(t);
    const script = copy(root, "scripts/publish_crates_in_order.mjs");
    const checksum = createHash("sha256").update("package fixture").digest("hex");
    const registryChecksum = scenario.registryChecksum === "matching" ? checksum : "0".repeat(64);
    const preload = join(root, "registry.mjs");
    writeFileSync(preload, `
      import { readFileSync, writeFileSync } from "node:fs";
      import { join } from "node:path";
      Atomics.wait = () => "timed-out";
      globalThis.fetch = async () => {
        const counter = join(process.env.TOOLING_FIXTURE, "queries");
        const count = Number(readFileSync(counter, "utf8")) + 1;
        writeFileSync(counter, String(count));
        if (count < 3 || ${scenario.registryChecksum === "missing"}) return { status: 404 };
        return { ok: true, status: 200, json: async () => ({ version: { checksum: "${registryChecksum}" } }) };
      };
    `);
    writeFileSync(join(root, "queries"), "0");
    command(root, "cargo", `
      const fs = require("node:fs");
      const path = require("node:path");
      if (process.argv[2] === "metadata") {
        console.log(JSON.stringify({ target_directory: path.join(process.env.TOOLING_FIXTURE, "target"), packages: [{ name: "fixture", version: "0.3.1", publish: null, dependencies: [] }] }));
      } else if (process.argv[2] === "package") {
        const directory = path.join(process.env.TOOLING_FIXTURE, "target/package");
        fs.mkdirSync(directory, { recursive: true });
        fs.writeFileSync(path.join(directory, "fixture-0.3.1.crate"), "package fixture");
      } else {
        const counter = path.join(process.env.TOOLING_FIXTURE, "uploads");
        const count = fs.existsSync(counter) ? Number(fs.readFileSync(counter, "utf8")) + 1 : 1;
        fs.writeFileSync(counter, String(count));
        console.error(${JSON.stringify(scenario.cargoError)});
        process.exit(1);
      }
    `);
    const result = spawnSync(process.execPath, ["--import", pathToFileURL(preload).href, script, "publish"], {
      cwd: root, env: environment(root), encoding: "utf8", timeout: 10_000,
    });
    assert.equal(result.error, undefined);
    assert.equal(result.status, scenario.status, result.stderr);
    assert.equal(Number(readFileSync(join(root, "queries"), "utf8")), scenario.queries);
    assert.equal(Number(readFileSync(join(root, "uploads"), "utf8")), 1);
    if (scenario.registryChecksum === "different") {
      assert.match(result.stderr, /published-crate-checksum-mismatch/u);
    }
    if (scenario.registryChecksum === "missing") {
      assert.match(result.stderr, /published-crate-not-verifiable/u);
    }
  });
}

test("Swift artifact path check accepts runner toolchain paths and rejects unremapped build paths", { skip: !posixHost }, (t) => {
  const root = fixture(t);
  const script = copy(root, "scripts/check_swift_static_library_paths.sh");
  const library = join(root, "fixture.a");
  const stringsOutput = join(root, "strings-output");
  writeFileSync(library, "fixture");
  command(root, "strings", 'process.stdout.write(require("node:fs").readFileSync(process.env.STRINGS_OUTPUT, "utf8"));');
  const env = { ...environment(root), HOME: "/Users/runner", STRINGS_OUTPUT: stringsOutput };
  const args = [script, library, "/Users/runner/work/codec", "/Users/runner/.cargo", "/Users/runner/.rustup"];
  for (const [value, expectedStatus] of [
    ["/Users/runner/work/rust/library/std/src/lib.rs", 0],
    ["/Users/runner/work/codec/crates/ffi/src/lib.rs", 1],
    ["/Users/runner/.cargo/registry/src/package.rs", 1],
    ["/Users/runner/.rustup/toolchains/custom/lib.rs", 1],
  ]) {
    writeFileSync(stringsOutput, `${value}\n`);
    const result = spawnSync("bash", args, { cwd: root, env, encoding: "utf8", timeout: 10_000 });
    assert.equal(result.error, undefined);
    assert.equal(result.status, expectedStatus, result.stderr);
  }
});

test("Linux native glibc floor rejects a newer symbol requirement", { skip: !posixHost }, (t) => {
  const root = fixture(t);
  const script = copy(root, "scripts/verify_linux_glibc_floor.sh");
  const resources = join(root, "resources");
  for (const architecture of ["linux-x86_64", "linux-aarch64"]) {
    const directory = join(resources, "me/really/codec/native", architecture);
    mkdirSync(directory, { recursive: true });
    writeFileSync(join(directory, "libreallyme_codec_ffi.so"), "ELF fixture");
  }
  command(root, "readelf", 'console.log(`Version needs section: Name: GLIBC_${process.env.REQUIRED_GLIBC_VERSION}`);');
  for (const [version, expectedStatus] of [["2.34", 0], ["2.35", 1]]) {
    const result = spawnSync("bash", [script, resources], {
      cwd: root,
      env: { ...environment(root), REQUIRED_GLIBC_VERSION: version },
      encoding: "utf8",
      timeout: 10_000,
    });
    assert.equal(result.error, undefined);
    assert.equal(result.status, expectedStatus, result.stderr);
  }
});

test("protobuf hardening supports checkout paths containing spaces and URL characters", (t) => {
  const root = fixture(t);
  for (const path of [
    "scripts/redact_codec_proto_debug.mjs", "scripts/codec_proto_sensitivity.mjs",
    "crates/proto/proto", "crates/proto/src/generated", "packages/ts/src/proto/generated", "packages/swift/Sources/ReallyMeCodec/GeneratedCodecProto.swift", "gen",
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
    if (android) {
      writeFileSync(
        join(toolchain, "llvm-readelf"),
        "#!/bin/sh\nprintf '  LOAD 0x0 0x0 0x0 0x1000 0x1000 R E 0x4000\\n'\n",
        { mode: 0o700 },
      );
    }
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
    if (android) {
      writeFileSync(
        join(toolchain, "llvm-readelf"),
        "#!/bin/sh\nprintf '  LOAD 0x0 0x0 0x0 0x1000 0x1000 R E 0x1000\\n'\n",
        { mode: 0o700 },
      );
      const misaligned = spawnSync("bash", [script], {
        cwd: elsewhere,
        env: { ...environment(root), ANDROID_NDK_HOME: ndk, CARGO_TARGET_DIR: join(root, "other-target") },
        encoding: "utf8", timeout: 15_000,
      });
      assert.equal(misaligned.status, 1, misaligned.stderr);
      assert.match(misaligned.stderr, /below 16 KB page alignment/u);
    }
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
