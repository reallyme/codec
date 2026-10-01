#!/usr/bin/env node
// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const repositoryRoot = fileURLToPath(new URL("..", import.meta.url));
const VERSION_PATTERN = /^(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)$/u;
const CRATE_PATTERN = /^[a-z][a-z0-9-]*$/u;
const EXPECTED_REPOSITORY = "reallyme/codec";
const REQUEST_TIMEOUT_MS = 20_000;

export class ReleaseAvailabilityError extends Error {
  constructor(code) {
    super(code);
    this.name = "ReleaseAvailabilityError";
    this.code = code;
  }
}

const fail = (code) => {
  throw new ReleaseAvailabilityError(code);
};

const isOccupied = async (fetchImpl, url, headers = undefined, method = "GET") => {
  let response;
  try {
    response = await fetchImpl(url, {
      method,
      headers,
      signal: AbortSignal.timeout(REQUEST_TIMEOUT_MS),
    });
  } catch {
    fail("registry-query-failed");
  }
  if (response.status === 404) {
    return false;
  }
  if (response.status === 200) {
    return true;
  }
  fail("registry-response-invalid");
};

export const verifyReleaseAvailability = async ({
  version,
  repository,
  token,
  crateNames,
  fetchImpl = fetch,
}) => {
  if (typeof version !== "string" || !VERSION_PATTERN.test(version)) {
    fail("invalid-release-version");
  }
  if (repository !== EXPECTED_REPOSITORY || typeof token !== "string" || token.length === 0) {
    fail("github-authentication-unavailable");
  }
  if (!Array.isArray(crateNames) || crateNames.length === 0 ||
      new Set(crateNames).size !== crateNames.length ||
      crateNames.some((name) => typeof name !== "string" || !CRATE_PATTERN.test(name))) {
    fail("invalid-publishable-crate-list");
  }

  const githubHeaders = {
    Accept: "application/vnd.github+json",
    Authorization: `Bearer ${token}`,
    "User-Agent": "reallyme-codec-release-preflight",
  };
  const tag = encodeURIComponent(`v${version}`);
  for (const url of [
    `https://api.github.com/repos/${repository}/git/ref/tags/${tag}`,
    `https://api.github.com/repos/${repository}/releases/tags/${tag}`,
  ]) {
    if (await isOccupied(fetchImpl, url, githubHeaders)) {
      fail("github-release-version-in-use");
    }
  }

  for (const crate of crateNames) {
    const url = `https://crates.io/api/v1/crates/${encodeURIComponent(crate)}/${version}`;
    if (await isOccupied(fetchImpl, url, { "User-Agent": "reallyme-codec-release-preflight" })) {
      fail("crates-release-version-in-use");
    }
  }

  const npmPackage = encodeURIComponent("@reallyme/codec");
  if (await isOccupied(fetchImpl, `https://registry.npmjs.org/${npmPackage}/${version}`)) {
    fail("npm-release-version-in-use");
  }

  for (const artifact of ["codec", "codec-android"]) {
    const url = `https://repo.maven.apache.org/maven2/me/really/${artifact}/${version}/${artifact}-${version}.pom`;
    if (await isOccupied(fetchImpl, url, undefined, "HEAD")) {
      fail("maven-release-version-in-use");
    }
  }
};

const isMain = process.argv[1] !== undefined && fileURLToPath(import.meta.url) === process.argv[1];
if (isMain) {
  try {
    const version = process.argv[2];
    const encoded = execFileSync(
      "cargo",
      ["metadata", "--format-version", "1", "--no-deps", "--locked"],
      { cwd: repositoryRoot, encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] },
    );
    const metadata = JSON.parse(encoded);
    const members = new Set(metadata.workspace_members);
    const publishable = metadata.packages.filter(
      (pkg) => members.has(pkg.id) && !(Array.isArray(pkg.publish) && pkg.publish.length === 0),
    );
    if (publishable.some((pkg) => pkg.version !== version)) {
      fail("workspace-version-mismatch");
    }
    await verifyReleaseAvailability({
      version,
      repository: process.env.GITHUB_REPOSITORY,
      token: process.env.GH_TOKEN,
      crateNames: publishable.map((pkg) => pkg.name),
    });
    console.log("release version is unused across GitHub and package registries");
  } catch (error) {
    const code = error instanceof ReleaseAvailabilityError ? error.code : "release-availability-check-failed";
    console.error(`release availability check failed: ${code}`);
    process.exitCode = 1;
  }
}
