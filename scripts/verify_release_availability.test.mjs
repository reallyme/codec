// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { strict as assert } from "node:assert";
import test from "node:test";

import {
  ReleaseAvailabilityError,
  verifyReleaseAvailability,
} from "./verify_release_availability.mjs";

const request = {
  version: "0.3.1",
  repository: "reallyme/codec",
  token: "test-token",
  crateNames: ["reallyme-codec", "reallyme-codec-base64"],
};

test("an unused version passes every registry and tag check", async () => {
  const urls = [];
  await verifyReleaseAvailability({
    ...request,
    fetchImpl: async (url, options) => {
      urls.push([url, options.method]);
      return { status: 404 };
    },
  });
  assert.equal(urls.length, 7);
  assert.equal(urls.filter(([, method]) => method === "HEAD").length, 2);
  assert(urls.some(([url]) => url.includes("%40reallyme%2Fcodec/0.3.1")));
});

for (const [registry, urlPart, reason] of [
  ["GitHub tag", "/git/ref/tags/", "github-release-version-in-use"],
  ["GitHub release", "/releases/tags/", "github-release-version-in-use"],
  ["crates.io", "/api/v1/crates/", "crates-release-version-in-use"],
  ["npm", "registry.npmjs.org", "npm-release-version-in-use"],
  ["Maven Central", "repo.maven.apache.org", "maven-release-version-in-use"],
]) {
  test(`a used ${registry} version fails closed`, async () => {
    await assert.rejects(
      verifyReleaseAvailability({
        ...request,
        fetchImpl: async (url) => ({ status: url.includes(urlPart) ? 200 : 404 }),
      }),
      (error) => error instanceof ReleaseAvailabilityError && error.code === reason,
    );
  });
}

test("registry errors and missing authentication fail closed", async () => {
  await assert.rejects(
    verifyReleaseAvailability({ ...request, token: "" }),
    (error) => error instanceof ReleaseAvailabilityError && error.code === "github-authentication-unavailable",
  );
  await assert.rejects(
    verifyReleaseAvailability({ ...request, fetchImpl: async () => ({ status: 429 }) }),
    (error) => error instanceof ReleaseAvailabilityError && error.code === "registry-response-invalid",
  );
});
