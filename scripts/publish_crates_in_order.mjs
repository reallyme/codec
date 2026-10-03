// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

const repositoryRoot = fileURLToPath(new URL("..", import.meta.url));
const MODE_INSPECT = "inspect";
const MODE_PUBLISH = "publish";
const args = process.argv.slice(2);
const mode = args[0] ?? MODE_INSPECT;
const PUBLISH_RETRY_ATTEMPTS = 12;
const allowDirty = args.includes("--allow-dirty");
const unknownArgs = args.slice(1).filter((arg) => arg !== "--allow-dirty");

if ((mode !== MODE_INSPECT && mode !== MODE_PUBLISH) || unknownArgs.length !== 0) {
  console.error(
    `usage: node scripts/publish_crates_in_order.mjs ${MODE_INSPECT}|${MODE_PUBLISH} [--allow-dirty]`,
  );
  process.exit(2);
}

if (allowDirty && mode !== MODE_INSPECT) {
  console.error("--allow-dirty is only supported for local package inspection");
  process.exit(2);
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: repositoryRoot,
    encoding: "utf8",
    stdio: options.capture ? "pipe" : "inherit",
  });
  if (result.error) {
    throw result.error;
  }
  return result;
}

function sleepMs(delayMs) {
  Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, delayMs);
}

function retryAfterMs(output) {
  const match = /try again after ([^\n.]+ GMT)/i.exec(output);
  if (!match) {
    return null;
  }

  const retryAt = Date.parse(match[1]);
  if (!Number.isFinite(retryAt)) {
    return null;
  }

  const delayMs = retryAt - Date.now() + 10000;
  return Math.max(delayMs, 10000);
}

const metadataResult = run("cargo", ["metadata", "--format-version", "1", "--no-deps", "--locked"], {
  capture: true,
});

if (metadataResult.status !== 0) {
  process.stderr.write(metadataResult.stderr);
  process.exit(metadataResult.status ?? 1);
}

const metadata = JSON.parse(metadataResult.stdout);

function isPublishablePackage(pkg) {
  return !(Array.isArray(pkg.publish) && pkg.publish.length === 0);
}

const publishable = new Map();
for (const pkg of metadata.packages) {
  if (isPublishablePackage(pkg)) {
    publishable.set(pkg.name, pkg);
  }
}

function isWorkspacePathDependency(dep) {
  return dep.source === null && typeof dep.path === "string" && publishable.has(dep.name);
}

// A published crate must not silently resolve a different release of an
// internal dependency after the next lockstep version becomes available.
function matchesReleasedVersion(req, version) {
  return req === `=${version}`;
}

function checkPathDependencyVersions() {
  const failures = [];
  for (const pkg of publishable.values()) {
    for (const dep of pkg.dependencies) {
      if (!isWorkspacePathDependency(dep)) {
        continue;
      }

      const target = publishable.get(dep.name);
      if (!matchesReleasedVersion(dep.req, target.version)) {
        failures.push(
          `${pkg.name} depends on ${dep.name} with ${dep.req}; local version is ${target.version}`,
        );
      }
    }
  }

  if (failures.length !== 0) {
    console.error("publishable workspace path dependency versions are stale:");
    for (const failure of failures) {
      console.error(`- ${failure}`);
    }
    process.exit(1);
  }
}

const visiting = new Set();
const visited = new Set();
const ordered = [];

function visit(pkg) {
  if (visited.has(pkg.name)) {
    return;
  }
  if (visiting.has(pkg.name)) {
    console.error(`workspace publish dependency cycle at ${pkg.name}`);
    process.exit(1);
  }

  visiting.add(pkg.name);
  for (const dep of pkg.dependencies) {
    const depName = dep.package ?? dep.name;
    if (dep.source === null && publishable.has(depName)) {
      visit(publishable.get(depName));
    }
  }
  visiting.delete(pkg.name);
  visited.add(pkg.name);
  ordered.push(pkg);
}

for (const pkg of publishable.values()) {
  visit(pkg);
}

console.log(`Publish order (${ordered.length} crates):`);
for (const pkg of ordered) {
  console.log(`- ${pkg.name} ${pkg.version}`);
}

checkPathDependencyVersions();

const orderedIndexByName = new Map();
ordered.forEach((pkg, index) => {
  orderedIndexByName.set(pkg.name, index);
});

function unresolvedRegistryPackages(output) {
  const missing = [];
  const noMatchPattern = /no matching package named `([^`]+)` found/g;
  for (let match = noMatchPattern.exec(output); match !== null; match = noMatchPattern.exec(output)) {
    missing.push(match[1]);
  }

  const versionSelectPattern = /failed to select a version for the requirement `([^`\s]+) =/g;
  for (
    let match = versionSelectPattern.exec(output);
    match !== null;
    match = versionSelectPattern.exec(output)
  ) {
    missing.push(match[1]);
  }

  return [...new Set(missing)];
}

function isEarlierWorkspaceDependency(pkg, depName) {
  const pkgIndex = orderedIndexByName.get(pkg.name);
  const depIndex = orderedIndexByName.get(depName);
  return depIndex !== undefined && pkgIndex !== undefined && depIndex < pkgIndex;
}

function awaitsWorkspaceDependency(pkg, output) {
  const missing = unresolvedRegistryPackages(output);
  return missing.length !== 0 && missing.every((depName) => isEarlierWorkspaceDependency(pkg, depName));
}

function inspectPackage(pkg) {
  const listArgs = ["package", "-p", pkg.name, "--list"];
  if (allowDirty) {
    listArgs.push("--allow-dirty");
  }
  const listResult = run("cargo", listArgs);
  if (listResult.status !== 0) {
    process.exit(listResult.status ?? 1);
  }

  const dryRunArgs = ["publish", "-p", pkg.name, "--dry-run", "--locked"];
  if (allowDirty) {
    dryRunArgs.push("--allow-dirty");
  }
  const dryRunResult = run("cargo", dryRunArgs, { capture: true });
  process.stdout.write(dryRunResult.stdout);
  process.stderr.write(dryRunResult.stderr);
  if (dryRunResult.status === 0) {
    return;
  }

  const combined = `${dryRunResult.stdout}\n${dryRunResult.stderr}`;
  const missing = unresolvedRegistryPackages(combined);
  if (
    missing.length !== 0 &&
    missing.every((depName) => isEarlierWorkspaceDependency(pkg, depName))
  ) {
    console.log(
      `${pkg.name} dry-run reached unpublished ordered workspace dependencies: ${missing.join(", ")}`,
    );
    return;
  }

  process.exit(dryRunResult.status ?? 1);
}

function packageChecksum(pkg) {
  // No build script or proc macro runs while a publishing token is present.
  for (let attempt = 1; attempt <= PUBLISH_RETRY_ATTEMPTS; attempt += 1) {
    const result = run("cargo", ["package", "-p", pkg.name, "--locked", "--no-verify"], {
      capture: true,
    });
    process.stdout.write(result.stdout);
    process.stderr.write(result.stderr);
    if (result.status === 0) {
      const archive = join(metadata.target_directory, "package", `${pkg.name}-${pkg.version}.crate`);
      return createHash("sha256").update(readFileSync(archive)).digest("hex");
    }
    const combined = `${result.stdout}\n${result.stderr}`;
    if (!awaitsWorkspaceDependency(pkg, combined) || attempt === PUBLISH_RETRY_ATTEMPTS) {
      process.exit(result.status ?? 1);
    }
    const delayMs = attempt * 15_000;
    console.log(`crates.io index has not observed an ordered dependency; retrying package ${pkg.name} in ${delayMs / 1000}s...`);
    sleepMs(delayMs);
  }
  throw new Error(`package-retry-state-invalid: ${pkg.name}`);
}

async function publishedChecksum(pkg) {
  const response = await fetch(
    `https://crates.io/api/v1/crates/${encodeURIComponent(pkg.name)}/${encodeURIComponent(pkg.version)}`,
    { headers: { "User-Agent": "reallyme-codec-release/0.3.1" }, signal: AbortSignal.timeout(20000) },
  );
  if (response.status === 404) {
    return null;
  }
  if (!response.ok) {
    throw new Error(`registry-version-query-failed: ${response.status}`);
  }
  const body = await response.json();
  const checksum = body?.version?.checksum;
  if (typeof checksum !== "string" || !/^[0-9a-f]{64}$/u.test(checksum)) {
    throw new Error("registry-version-checksum-missing");
  }
  return checksum;
}

async function previouslyPublishedMatches(pkg, localChecksum) {
  const registryChecksum = await publishedChecksum(pkg);
  if (registryChecksum === null) {
    return false;
  }
  if (registryChecksum !== localChecksum) {
    throw new Error(`published-crate-checksum-mismatch: ${pkg.name} ${pkg.version}`);
  }
  console.log(`${pkg.name} ${pkg.version} already has the verified release checksum`);
  return true;
}

async function waitForPublishedChecksum(pkg, localChecksum) {
  // Cargo can time out after crates.io accepted the upload. The version API
  // may also lag the upload response, so allow bounded propagation before
  // deciding that the release state cannot be verified.
  const verificationAttempts = 12;
  const verificationDelayMs = 15_000;
  for (let attempt = 1; attempt <= verificationAttempts; attempt += 1) {
    if (await previouslyPublishedMatches(pkg, localChecksum)) {
      return;
    }
    if (attempt < verificationAttempts) {
      sleepMs(verificationDelayMs);
    }
  }
  throw new Error(`published-crate-not-verifiable: ${pkg.name} ${pkg.version}`);
}

async function publishPackage(pkg) {
  const localChecksum = packageChecksum(pkg);
  if (await previouslyPublishedMatches(pkg, localChecksum)) {
    return;
  }
  // Package verification already ran in the uncredentialed preflight job.
  const args = ["publish", "-p", pkg.name, "--locked", "--no-verify"];

  for (let attempt = 1; attempt <= PUBLISH_RETRY_ATTEMPTS; attempt += 1) {
    const result = run("cargo", args, { capture: true });
    process.stdout.write(result.stdout);
    process.stderr.write(result.stderr);

    if (result.status === 0) {
      return;
    }

    const combined = `${result.stdout}\n${result.stderr}`;
    const lowerCombined = combined.toLowerCase();
    if (
      lowerCombined.includes("already uploaded") ||
      lowerCombined.includes("already exists") ||
      (lowerCombined.includes("timed out") && lowerCombined.includes("publish"))
    ) {
      await waitForPublishedChecksum(pkg, localChecksum);
      return;
    }

    const rateLimitDelayMs = retryAfterMs(combined);
    if (lowerCombined.includes("too many requests") && rateLimitDelayMs !== null) {
      console.log(
        `crates.io rate-limited new crate uploads; retrying ${pkg.name} in ${Math.ceil(rateLimitDelayMs / 1000)}s...`,
      );
      sleepMs(rateLimitDelayMs);
      continue;
    }

    if (!awaitsWorkspaceDependency(pkg, combined) || attempt === PUBLISH_RETRY_ATTEMPTS) {
      process.exit(result.status ?? 1);
    }

    const delayMs = attempt * 15000;
    console.log(
      `crates.io index has not observed a freshly published dependency yet; retrying ${pkg.name} in ${delayMs / 1000}s...`,
    );
    sleepMs(delayMs);
  }

  // Rate-limit responses can consume every attempt without entering the
  // ordinary failure branch. Never report a partially published release as done.
  console.error(`${pkg.name} publish retry limit exhausted`);
  process.exit(1);
}

for (const pkg of ordered) {
  if (mode === MODE_INSPECT) {
    inspectPackage(pkg);
    continue;
  }

  await publishPackage(pkg);
}
