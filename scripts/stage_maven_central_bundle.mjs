// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
// SPDX-License-Identifier: MIT OR Apache-2.0

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { closeSync, createReadStream, existsSync, lstatSync, mkdirSync, mkdtempSync, openSync, readdirSync, realpathSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { isAbsolute, join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { MavenPublicationError, publicationLayout, stageMavenPublications } from "./stage_maven_publications.mjs";

const repositoryRoot = fileURLToPath(new URL("../", import.meta.url));
const repository = "reallyme/codec";
const pinRunner = join(repositoryRoot, "scripts/run_pinned_release_readiness.mjs");
const readinessRepository = "https://github.com/reallyme/release-readiness.git";
const requestTimeoutMs = 120_000;
const linuxNativeFiles = ["linux-x86_64", "linux-aarch64"].map(
  (target) => `me/really/codec/native/${target}/libreallyme_codec_ffi.so`,
);
const jvmNativeFiles = [
  ...linuxNativeFiles,
  "me/really/codec/native/macos-x86_64/libreallyme_codec_ffi.dylib",
  "me/really/codec/native/macos-aarch64/libreallyme_codec_ffi.dylib",
  "me/really/codec/native/windows-x86_64/reallyme_codec_ffi.dll",
  "me/really/codec/native/native-manifest.json",
];
const androidNativeFiles = [
  ...["arm64-v8a", "armeabi-v7a", "x86_64", "x86"].map((abi) => `jni/${abi}/libreallyme_codec_ffi.so`),
  "assets/reallyme-codec/native-manifest.json",
];

function fail(message) {
  throw new MavenPublicationError(message);
}

function run(command, args, message, options = {}) {
  const result = spawnSync(command, args, {
    cwd: repositoryRoot,
    encoding: "utf8",
    maxBuffer: 16_777_216,
    timeout: requestTimeoutMs,
    ...options,
  });
  if (result.status !== 0 || result.error !== undefined) fail(message);
  return result.stdout;
}

function loadPinnedPackage(scratch) {
  const installed = join(repositoryRoot, ".release-readiness");
  let packageRoot = process.env.REALLYME_RELEASE_READINESS_ROOT;
  if (packageRoot === undefined) {
    packageRoot = existsSync(installed) ? installed : join(scratch, "release-readiness");
    if (packageRoot !== installed) {
      const commit = run(process.execPath, [pinRunner, "--print-commit"], "could not identify the pinned policy package").trim();
      if (!/^[0-9a-f]{40}$/u.test(commit)) fail("invalid pinned policy package commit");
      run("git", ["init", "--quiet", packageRoot], "could not initialize the policy package checkout");
      run("git", ["fetch", "--quiet", "--no-tags", "--depth=1", readinessRepository, commit],
        "could not fetch the pinned policy package", { cwd: packageRoot });
      run("git", ["checkout", "--quiet", "--detach", commit], "could not check out the pinned policy package", { cwd: packageRoot });
    }
  }
  packageRoot = realpathSync(packageRoot);
  run(process.execPath, [pinRunner, "--check-pins"], "could not authenticate the pinned policy package", {
    env: { ...process.env, REALLYME_RELEASE_READINESS_ROOT: packageRoot },
  });
  return packageRoot;
}

function readArtifact(id) {
  const metadata = run("gh", ["api", "--hostname", "github.com", `repos/${repository}/actions/artifacts/${id}`],
    "could not read the retained preflight artifact metadata");
  try {
    return JSON.parse(metadata);
  } catch {
    fail("GitHub returned invalid preflight artifact metadata");
  }
}

async function readPreflight(packageRoot, layout, version, sourceSha) {
  // The shared verifier supplies structured artifact IDs for the local adapter.
  const load = (path) => import(pathToFileURL(join(packageRoot, path)).href);
  const { createReleaseReadinessContext } = await load("core.mjs");
  const { assertPublicRepoPolicy } = await load("scripts/public-repo-policy.mjs");
  const {
    assertOnMain, assertPublicPreflightRun, assertPublicReleaseInvocation,
    assertPublicReleaseVersion, findPackageArtifacts,
  } = await load("scripts/public-release.mjs");
  const context = createReleaseReadinessContext({ scriptUrl: import.meta.url, failurePrefix: "Maven staging failed" });
  const trackedPaths = run("git", ["ls-files", "-z"], "could not read the tracked release inputs");
  const trackedFiles = new Set(trackedPaths.split("\0").filter(Boolean));
  const policy = assertPublicRepoPolicy(context, trackedFiles, { RELEASE_READINESS_PUBLIC_REPO: "true" });
  assertPublicReleaseInvocation(context, repository, sourceSha, "maven", "publish", policy.release);
  assertPublicReleaseVersion(context, trackedFiles, policy.release, version, `v${version}`);
  assertOnMain(context, repository, sourceSha);
  const verified = assertPublicPreflightRun(context, repository, policy.release, "maven", version, sourceSha);
  const artifacts = verified.packageArtifactIds.map(readArtifact);
  const selectedIds = findPackageArtifacts(artifacts, { id: verified.preflightRunId, head_branch: "main" },
    sourceSha, layout.map(({ name }) => name));
  if (selectedIds === null || selectedIds.length !== verified.packageArtifactIds.length ||
      selectedIds.some((id) => !verified.packageArtifactIds.includes(id))) {
    fail("retained JVM and Android artifacts must come from the same verified preflight");
  }
  console.error(`Using Maven package preflight ${verified.preflightRunId} for ${version} at ${sourceSha}.`);
  return layout.map((publication, index) => ({ ...publication, artifact: artifacts.find(({ id }) => id === selectedIds[index]) }));
}

function requireArchiveEntries(archive, expected) {
  const listing = run("jar", ["tf", archive], "could not list a Maven archive");
  const entries = listing.split(/\r?\n/u);
  for (const entry of expected) {
    if (entries.filter((name) => name === entry).length !== 1) fail("Maven archive has a missing or duplicate entry");
  }
}

export async function downloadPublication(publication, packages) {
  const { name, artifact, files } = publication;
  const archive = join(packages, `${name}.zip`);
  const descriptor = openSync(archive, "wx", 0o600);
  try {
    run("gh", ["api", "--hostname", "github.com", "--allow-escape-sequences",
      `repos/${repository}/actions/artifacts/${artifact.id}/zip`], "could not download a retained Maven artifact", {
      stdio: ["ignore", descriptor, "ignore"],
    });
  } finally {
    closeSync(descriptor);
  }
  const hash = createHash("sha256");
  for await (const chunk of createReadStream(archive)) hash.update(chunk);
  if (`sha256:${hash.digest("hex")}` !== artifact.digest) fail("downloaded Maven artifact digest does not match preflight");

  // Extract only the exact versioned publication entries. Repository metadata
  // and the preflight wrapper files are outside the signed Maven bundle.
  const entries = files.map(({ source }) => source);
  requireArchiveEntries(archive, entries);
  const output = join(packages, name);
  mkdirSync(output);
  run("jar", ["xf", archive, ...entries], "could not extract the verified Maven publication", { cwd: output });
}

function verifyNativeArchives(destination, version, scratch) {
  const jvm = join(destination, `me/really/codec/${version}/codec-${version}.jar`);
  const android = join(destination, `me/really/codec-android/${version}/codec-android-${version}.aar`);
  requireArchiveEntries(jvm, jvmNativeFiles);
  requireArchiveEntries(android, androidNativeFiles);
  if (process.platform === "linux") {
    const resources = join(scratch, "linux-native");
    mkdirSync(resources);
    run("jar", ["xf", jvm, ...linuxNativeFiles], "could not extract the Linux JNI resources", { cwd: resources });
    run("bash", [join(repositoryRoot, "scripts/verify_linux_glibc_floor.sh"), resources],
      "bundled Linux JNI libraries exceed the glibc floor or could not be checked");
  }
}

async function stage() {
  const [, , version, sourceSha, destination, ...extra] = process.argv;
  const layout = publicationLayout(version, sourceSha);
  if (destination === undefined || extra.length !== 0 || !isAbsolute(destination) ||
      !lstatSync(destination).isDirectory() || readdirSync(destination).length !== 0) {
    fail("expected an empty, absolute staging directory");
  }
  const scratch = mkdtempSync(join(tmpdir(), "codec-maven-preflight-"));
  const cleanup = () => rmSync(scratch, { recursive: true, force: true });
  // Shared policy failures exit directly, including before any download starts.
  process.once("exit", cleanup);
  try {
    const packageRoot = loadPinnedPackage(scratch);
    const publications = await readPreflight(packageRoot, layout, version, sourceSha);
    const packages = join(scratch, "packages");
    mkdirSync(packages);
    for (const publication of publications) await downloadPublication(publication, packages);
    stageMavenPublications(packages, version, sourceSha, destination);
    verifyNativeArchives(destination, version, scratch);
  } finally {
    process.removeListener("exit", cleanup);
    cleanup();
  }
}

const isMain = process.argv[1] !== undefined && fileURLToPath(import.meta.url) === process.argv[1];
if (isMain) {
  try {
    await stage();
  } catch (error) {
    const message = error instanceof MavenPublicationError ? error.message : "could not stage the verified Maven publications";
    console.error(`Maven staging failed: ${message}`);
    process.exitCode = 1;
  }
}
