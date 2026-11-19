// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const script = fileURLToPath(new URL("./prepare-android-avd.sh", import.meta.url));

function fixture(t, { installed = false } = {}) {
  const root = mkdtempSync(join(tmpdir(), "codec Android tools # "));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const writeTool = (path, source) => {
    const destination = join(root, path);
    mkdirSync(dirname(destination), { recursive: true });
    writeFileSync(destination, `#!/usr/bin/env node\n${source}`, { mode: 0o700 });
  };
  writeTool("tools/sdkmanager", `
    import assert from "node:assert/strict";
    import { appendFileSync } from "node:fs";
    const args = process.argv.slice(2);
    const event = (value) => appendFileSync(process.env.FIXTURE_ROOT + "/events", value + "\\n");
    if (args[0] === "--version") {
      event("version");
      console.log(process.env.FIXTURE_VERSION ?? "23.0");
      process.exit(Number(process.env.FIXTURE_VERSION_EXIT ?? "0"));
    }
    assert.deepEqual(args, ["emulator", "platforms;android-37.0", "system-images;android-37.1;google_apis_ps16k;x86_64"]);
    event("packages");
    process.exit(Number(process.env.FIXTURE_PACKAGES_EXIT ?? "0"));
  `);
  writeTool("tools/avdmanager", `
    import assert from "node:assert/strict";
    import { appendFileSync, writeFileSync } from "node:fs";
    import { join } from "node:path";
    assert.deepEqual(process.argv.slice(2), ["create", "avd", "--force", "-n", "consumer-test", "-k",
      "system-images;android-37.1;google_apis_ps16k;x86_64", "--device", "pixel"]);
    appendFileSync(process.env.FIXTURE_ROOT + "/events", "avd\\n");
    if (process.env.FIXTURE_TARGET !== "missing") {
      writeFileSync(join(process.env.ANDROID_AVD_HOME, "consumer-test.ini"),
        "target=" + (process.env.FIXTURE_TARGET ?? "android-37.1") + "\\n");
    }
  `);
  writeTool("sdk/cmdline-tools/latest/bin/sdkmanager", `
    import assert from "node:assert/strict";
    import { appendFileSync, cpSync, mkdirSync } from "node:fs";
    import { join } from "node:path";
    assert.deepEqual(process.argv.slice(2), ["--install", "cmdline-tools;23.0"]);
    const root = process.env.FIXTURE_ROOT;
    appendFileSync(root + "/events", "install\\n");
    if (process.env.FIXTURE_INSTALL_EXIT !== undefined) process.exit(Number(process.env.FIXTURE_INSTALL_EXIT));
    const destination = join(process.env.ANDROID_HOME, "cmdline-tools/23.0/bin");
    mkdirSync(destination, { recursive: true });
    for (const tool of ["sdkmanager", "avdmanager"]) cpSync(join(root, "tools", tool), join(destination, tool));
  `);
  if (installed) {
    mkdirSync(join(root, "sdk/cmdline-tools/23.0/bin"), { recursive: true });
    for (const tool of ["sdkmanager", "avdmanager"]) {
      cpSync(join(root, "tools", tool), join(root, "sdk/cmdline-tools/23.0/bin", tool));
    }
  }
  const run = (overrides = {}, args = ["consumer-test"]) => spawnSync("bash", [script, ...args], {
    encoding: "utf8", timeout: 30_000,
    env: {
      ...process.env, FIXTURE_ROOT: root,
      ANDROID_HOME: join(root, "sdk"), ANDROID_AVD_HOME: join(root, "avd"), ...overrides,
    },
  });
  const events = () => existsSync(join(root, "events")) ? readFileSync(join(root, "events"), "utf8").trim().split("\n") : [];
  return { run, events };
}

test("AVD creation installs and selects pinned tools before validating the image target", (t) => {
  const { run, events } = fixture(t);
  const result = run();
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(events(), ["install", "version", "packages", "avd"]);
});

test("already installed pinned tools are checked without using the hosted latest tools", (t) => {
  const { run, events } = fixture(t, { installed: true });
  const result = run({ FIXTURE_INSTALL_EXIT: "37" });
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(events(), ["version", "packages", "avd"]);
});

test("a failed pinned tools installation cannot fall back to the hosted tools", (t) => {
  const { run, events } = fixture(t);
  assert.equal(run({ FIXTURE_INSTALL_EXIT: "37" }).status, 37);
  assert.deepEqual(events(), ["install"]);
});

test("wrong versions and unsuccessful version probes stop before package or AVD creation", (t) => {
  for (const overrides of [{ FIXTURE_VERSION: "12.0" }, { FIXTURE_VERSION_EXIT: "38" }]) {
    const { run, events } = fixture(t, { installed: true });
    const result = run(overrides);
    assert.equal(result.status, 1);
    assert.match(result.stderr, /unexpected version/u);
    assert.deepEqual(events(), ["version"]);
  }
});

test("failed image installation stops before AVD creation", (t) => {
  const { run, events } = fixture(t, { installed: true });
  assert.equal(run({ FIXTURE_PACKAGES_EXIT: "39" }).status, 39);
  assert.deepEqual(events(), ["version", "packages"]);
});

test("android-0, other image targets and missing metadata fail before emulator boot", (t) => {
  for (const target of ["android-0", "android-37.0", "missing"]) {
    const { run } = fixture(t, { installed: true });
    const result = run({ FIXTURE_TARGET: target });
    assert.equal(result.status, 1);
    assert.match(result.stderr, /target does not match/u);
  }
});

test("unsafe AVD names and relative SDK paths fail before invoking tools", (t) => {
  const { run, events } = fixture(t);
  for (const args of [[], ["../escape"], ["bad;name"]]) assert.equal(run({}, args).status, 2);
  assert.equal(run({ ANDROID_HOME: "relative/sdk" }).status, 2);
  assert.equal(run({ ANDROID_AVD_HOME: "relative/avd" }).status, 2);
  assert.deepEqual(events(), []);
});
