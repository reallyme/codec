// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, realpathSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { delimiter, join } from "node:path";
import test from "node:test";
import { MavenPublicationError, publicationLayout, stageMavenPublications } from "./stage_maven_publications.mjs";

function fixture(t) {
  const root = realpathSync(mkdtempSync(join(tmpdir(), "reallyme-maven-bundle-")));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const repo = join(root, "repo");
  const tools = join(root, "tools");
  const output = join(root, "kept bundles");
  mkdirSync(join(repo, "scripts"), { recursive: true });
  mkdirSync(tools);
  mkdirSync(output);
  if (process.platform === "darwin") {
    // Exercise the system Bash used on macOS runners, even with Homebrew Bash installed.
    symlinkSync("/bin/bash", join(tools, "bash"));
  }
  const manifest = join(repo, "packages/kotlin/build.gradle.kts");
  mkdirSync(join(repo, "packages/kotlin"), { recursive: true });
  writeFileSync(manifest, 'version = "1.2.3"\n');
  const helper = join(repo, "scripts/maven-central-bundle.local.sh");
  const calls = join(root, "gpg-calls");
  const pom = "<project>sample release</project>\n";
  writeFileSync(helper, readFileSync(new URL("./maven-central-bundle.local.sh", import.meta.url)), { mode: 0o755 });
  const executable = (path, script) => writeFileSync(path, `#!/usr/bin/env bash\nset -euo pipefail\n${script}\n`, { mode: 0o755 });
  executable(join(repo, "scripts/stage-maven-central-bundle.sh"), `
    [[ -z "\${MAVEN_SIGNING_PASSWORD:-}" && -z "\${signing_passphrase:-}" ]]
    version="$1"
    test "$2" = "$(git rev-parse HEAD)"
    if [[ "$BUNDLE_SCENARIO" == wrong-version ]]; then version=9.0.0; fi
    publication="$3/me/really/example/$version"
    mkdir -p -- "$publication"
    printf '%s\\n' '<project>sample release</project>' >"$publication/example-$version.pom"
    if [[ "$BUNDLE_SCENARIO" == symlink ]]; then
      ln -s /dev/null "$publication/example-$version.jar"
    else
      printf '%s\\n' 'example payload' >"$publication/example-$version.jar"
    fi
  `);
  executable(join(tools, "gpg"), `
    [[ -z "\${MAVEN_SIGNING_PASSWORD:-}" && -z "\${signing_passphrase:-}" ]]
    if [[ " $* " == *" --verify "* ]]; then
      printf 'verify\\n' >>"$GPG_CALLS"
      test -s "\${@: -2:1}"
      test -f "\${@: -1}"
      [[ "$BUNDLE_SCENARIO" != bad-signature ]]
      exit
    fi
    printf 'sign\\n' >>"$GPG_CALLS"
    IFS= read -r passphrase
    test "$passphrase" = 'fixture-only'
    while [[ "$1" != --output ]]; do shift; done
    printf '%s\\n' 'fixture signature' >"$2"
  `);
  executable(join(tools, "zip"), `
    # Bash 3.2 can ignore errexit in this conditional subprocess.
    if [[ "$BUNDLE_SCENARIO" == zip-error ]]; then exit 1; fi
    exec "$REAL_ZIP" "$@"
  `);
  const zip = spawnSync("sh", ["-c", "command -v zip"], { encoding: "utf8" });
  assert.equal(zip.status, 0, zip.stderr);
  const sha = "1234567890abcdef".repeat(2) + "12345678";
  executable(join(tools, "git"), `
    if [[ "$1" == -C ]]; then
      test "$2" = "$FIXTURE_REPOSITORY" || exit 79
      shift 2
    fi
    case "\${1}" in
      rev-parse)
        if [[ "$BUNDLE_SCENARIO" == missing-head ]]; then exit 1; fi
        printf '%s\\n' '${sha}' ;;
      diff) [[ "$BUNDLE_SCENARIO" != dirty ]] ;;
      ls-files) exit 0 ;;
      *) exit 79 ;;
    esac
  `);
  writeFileSync(calls, "");
  const run = (version, overrides = {}, source = sha) => spawnSync(helper, version === undefined ? [] : [version, source], {
    cwd: root,
    encoding: "utf8",
    env: {
      ...process.env, PATH: `${tools}${delimiter}${process.env.PATH}`,
      GPG_CALLS: calls, REAL_ZIP: zip.stdout.trim(), BUNDLE_SCENARIO: "",
      FIXTURE_REPOSITORY: repo, MAVEN_RELEASE_VERSION: "", MAVEN_CENTRAL_WORK_DIR: "",
      MAVEN_SIGNING_KEY_ID: "1234567890ABCDEF", MAVEN_SIGNING_PASSWORD: "fixture-only",
      MAVEN_CENTRAL_OUTPUT_DIR: output, ...overrides,
    },
  });
  return { run, root, output, calls, pom, manifest };
}

test("Maven bundle template verifies each signature and retains every completed ZIP", { skip: process.platform === "win32" }, (t) => {
  const { run, output, calls, pom } = fixture(t);
  const bundle = (version, suffix = "") => {
    const result = run(version);
    assert.equal(result.status, 0, result.stderr);
    const path = join(output, `reallyme-maven-central-${version}${suffix}.zip`);
    assert.equal(result.stdout.trim(), path);
    return path;
  };
  const first = bundle("1.2.3");
  const original = readFileSync(first);
  bundle("1.2.3", "-2");
  bundle("1.2.4");
  assert.deepEqual(readFileSync(first), original);
  assert.deepEqual(readdirSync(output).sort(), [
    "reallyme-maven-central-1.2.3-2.zip", "reallyme-maven-central-1.2.3.zip", "reallyme-maven-central-1.2.4.zip",
  ]);
  assert.equal(readFileSync(calls, "utf8"), "sign\nsign\nverify\nverify\n".repeat(3));
  const entry = "me/really/example/1.2.3/example-1.2.3.pom";
  const extract = (path) => {
    const result = spawnSync("unzip", ["-p", first, path], { encoding: "utf8" });
    assert.equal(result.status, 0, result.stderr);
    return result.stdout;
  };
  assert.equal(extract(entry), pom);
  assert.equal(extract(`${entry}.asc`), "fixture signature\n");
  assert.equal(extract(`${entry}.sha256`), `${createHash("sha256").update(pom).digest("hex")}\n`);
});

test("Maven bundle template preserves prior archives when staging, signing, or ZIP creation fails", { skip: process.platform === "win32" }, (t) => {
  const { run, output } = fixture(t);
  const original = join(output, "reallyme-maven-central-1.2.3.zip");
  writeFileSync(original, "previous archive");
  for (const [version, env, diagnostic, source] of [
    ["1.2.3", {}, /unchanged tracked files/u, "0".repeat(40)],
    ["1.2.3", { BUNDLE_SCENARIO: "wrong-version" }, /selected version/u],
    ["1.2.3", { BUNDLE_SCENARIO: "symlink" }, /regular files and directories/u],
    ["1.2.3", { BUNDLE_SCENARIO: "bad-signature" }, /signature failed verification/u],
    ["1.2.3", { BUNDLE_SCENARIO: "zip-error" }, /could not create the Maven Central ZIP/u],
  ]) {
    const result = run(version, env, source);
    assert.equal(result.status, 1, `${env.BUNDLE_SCENARIO ?? "different-source"}: ${result.stderr}`);
    assert.match(result.stderr, diagnostic);
    assert.equal(result.stdout, "");
    assert.deepEqual(readdirSync(output), ["reallyme-maven-central-1.2.3.zip"]);
    assert.equal(readFileSync(original, "utf8"), "previous archive");
  }
});

test("Maven bundle without arguments selects the package version and current checkout from another directory", { skip: process.platform === "win32" }, (t) => {
  const { run, output } = fixture(t);
  const result = run();
  const archive = join(output, "reallyme-maven-central-1.2.3.zip");
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.stdout.trim(), archive);
  assert.deepEqual(readdirSync(output), ["reallyme-maven-central-1.2.3.zip"]);
});

test("Maven bundle accepts a default version override and preserves explicit argument precedence", { skip: process.platform === "win32" }, (t) => {
  const { run, output } = fixture(t);
  const selected = run(undefined, { MAVEN_RELEASE_VERSION: "1.2.4" });
  assert.equal(selected.status, 0, selected.stderr);
  assert.equal(selected.stdout.trim(), join(output, "reallyme-maven-central-1.2.4.zip"));
  const explicit = run("1.2.5", { MAVEN_RELEASE_VERSION: "9.9.9" });
  assert.equal(explicit.status, 0, explicit.stderr);
  assert.equal(explicit.stdout.trim(), join(output, "reallyme-maven-central-1.2.5.zip"));
});

test("Maven bundle defaults reject invalid versions, unavailable commits, and dirty tracked files before signing", { skip: process.platform === "win32" }, (t) => {
  const { run, output, calls, manifest } = fixture(t);
  for (const [contents, overrides, diagnostic] of [
    ["", {}, /stable release version/u],
    ['version = "1.2.3"\nversion = "1.2.4"\n', {}, /stable release version/u],
    ['version = "1.2.3"\n', { MAVEN_RELEASE_VERSION: "1.2.4-rc.1" }, /stable release version/u],
    ['version = "1.2.3"\n', { BUNDLE_SCENARIO: "missing-head" }, /full source SHA/u],
    ['version = "1.2.3"\n', { BUNDLE_SCENARIO: "dirty" }, /unchanged tracked files/u],
  ]) {
    writeFileSync(manifest, contents);
    const result = run(undefined, overrides);
    assert.equal(result.status, 1, result.stderr);
    assert.match(result.stderr, diagnostic);
    assert.equal(result.stdout, "");
    assert.deepEqual(readdirSync(output), []);
    assert.equal(readFileSync(calls, "utf8"), "");
  }
});

test("Maven bundle uses the work directory fallback while retaining explicit output precedence", { skip: process.platform === "win32" }, (t) => {
  const { run, root, output } = fixture(t);
  const work = join(root, "bundle work");
  const selected = run(undefined, { MAVEN_CENTRAL_OUTPUT_DIR: "", MAVEN_CENTRAL_WORK_DIR: work });
  assert.equal(selected.status, 0, selected.stderr);
  assert.equal(selected.stdout.trim(), join(work, "out/reallyme-maven-central-1.2.3.zip"));
  const explicit = run(undefined, { MAVEN_CENTRAL_WORK_DIR: work });
  assert.equal(explicit.status, 0, explicit.stderr);
  assert.equal(explicit.stdout.trim(), join(output, "reallyme-maven-central-1.2.3.zip"));
  assert.deepEqual(readdirSync(join(work, "out")), ["reallyme-maven-central-1.2.3.zip"]);
});

function publicationFixture(t) {
  const root = realpathSync(mkdtempSync(join(tmpdir(), "codec-maven-publication-")));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const version = "0.4.4";
  const sha = "a".repeat(40);
  const packages = join(root, "packages");
  const destination = join(root, "destination");
  mkdirSync(packages);
  mkdirSync(destination);
  return { root, version, sha, packages, destination, layout: publicationLayout(version, sha) };
}

test("Maven staging copies the exact unsigned JVM and Android packages", { skip: process.platform === "win32" }, (t) => {
  const { version, sha, packages, destination, layout } = publicationFixture(t);
  for (const { name, files } of layout) {
    for (const { source, target } of files) {
      const input = join(packages, name, source);
      mkdirSync(join(input, ".."), { recursive: true });
      writeFileSync(input, target);
      writeFileSync(`${input}.sha256`, "local repository checksum");
    }
  }
  stageMavenPublications(packages, version, sha, destination);

  for (const { files } of layout) {
    for (const { target } of files) {
      assert.equal(readFileSync(join(destination, target), "utf8"), target);
      assert.equal(existsSync(`${join(destination, target)}.sha256`), false);
    }
  }
  assert.deepEqual(readdirSync(join(destination, "me/really/codec/0.4.4")).sort(), [
    "codec-0.4.4-javadoc.jar", "codec-0.4.4-sources.jar", "codec-0.4.4.jar", "codec-0.4.4.module", "codec-0.4.4.pom",
  ]);
  assert.deepEqual(readdirSync(join(destination, "me/really/codec-android/0.4.4")).sort(), [
    "codec-android-0.4.4-sources.jar", "codec-android-0.4.4.aar", "codec-android-0.4.4.module", "codec-android-0.4.4.pom",
  ]);
  assert.throws(() => stageMavenPublications(packages, version, sha, destination), { code: "EEXIST" });
});

test("Maven staging rejects a symlink in the authenticated publication tree", { skip: process.platform === "win32" }, (t) => {
  const { root, version, sha, packages, destination, layout: [publication] } = publicationFixture(t);
  const outside = join(root, "outside");
  const archive = publication.files[0];
  const expectedPath = join(packages, publication.name, archive.source);
  writeFileSync(outside, "unrelated artifact");
  mkdirSync(join(expectedPath, ".."), { recursive: true });
  symlinkSync(outside, expectedPath);
  assert.throws(() => stageMavenPublications(packages, version, sha, destination), MavenPublicationError);
  assert.deepEqual(readdirSync(destination), []);
});

for (const valid of [true, false]) {
  test(`retained Maven download ${valid ? "extracts verified bytes" : "rejects a different digest before extraction"}`,
    { skip: process.platform === "win32" }, (t) => {
      const { root, packages, layout: [, publication] } = publicationFixture(t);
      const artifactFile = publication.files[0].source;
      const source = join(root, "fixture-source");
      const archive = join(root, "fixture.zip");
      const bin = join(root, "bin");
      mkdirSync(join(source, artifactFile, ".."), { recursive: true });
      mkdirSync(bin);
      writeFileSync(join(source, artifactFile), "tested AAR bytes");
      writeFileSync(join(source, "sha256.json"), "manifest outside publication");
      const zip = spawnSync("zip", ["-q", "-r", archive, "."], { cwd: source, encoding: "utf8" });
      assert.equal(zip.status, 0, zip.stderr);
      writeFileSync(join(bin, "gh"), [
        "#!/bin/sh",
        '[ "$5" = repos/reallyme/codec/actions/artifacts/1234/zip ] || exit 71',
        'cat "$FIXTURE_ARCHIVE"',
        "",
      ].join("\n"), { mode: 0o755 });
      const digest = valid ? createHash("sha256").update(readFileSync(archive)).digest("hex") : "0".repeat(64);
      const request = { ...publication, files: publication.files.slice(0, 1), artifact: { id: 1234, digest: `sha256:${digest}` } };
      const script = new URL("./stage_maven_central_bundle.mjs", import.meta.url).href;
      const result = spawnSync(process.execPath, ["--input-type=module", "-e", `
        import { downloadPublication } from ${JSON.stringify(script)};
        try { await downloadPublication(${JSON.stringify(request)}, ${JSON.stringify(packages)}); }
        catch (error) { console.error(error.message); process.exitCode = 1; }
      `], {
        encoding: "utf8",
        env: { ...process.env, PATH: `${bin}${delimiter}${process.env.PATH}`, FIXTURE_ARCHIVE: archive },
        timeout: 10_000,
      });
      assert.equal(result.status, valid ? 0 : 1, result.stderr);
      const extracted = join(packages, publication.name);
      if (valid) {
        assert.equal(readFileSync(join(extracted, artifactFile), "utf8"), "tested AAR bytes");
        assert.equal(existsSync(join(extracted, "sha256.json")), false);
      } else {
        assert.match(result.stderr, /artifact digest does not match preflight/u);
        assert.equal(existsSync(extracted), false);
      }
    });
}
