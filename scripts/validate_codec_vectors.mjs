// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

const manifestPath = resolve(process.cwd(), "vectors/codec-vectors.json");
const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
const deterministic = manifest?.deterministicCbor;
const deterministicMinimumCounts = Object.freeze({
  positive: 30,
  negative: 15,
  equivalentInputOrders: 1,
  resourceRejections: 5,
  interoperability: 4,
});

const fail = (message) => {
  throw new Error(`codec vector validation failed: ${message}`);
};

const requireString = (value, path) => {
  if (typeof value !== "string" || value.length === 0) {
    fail(`${path} must be a non-empty string`);
  }
  return value;
};

const requireSha256 = (value, path) => {
  const digest = requireString(value, path);
  if (!/^[0-9a-f]{64}$/u.test(digest)) {
    fail(`${path} must be a lowercase SHA-256 digest`);
  }
  return digest;
};

if (manifest?.schemaVersion !== 2 || deterministic === null || typeof deterministic !== "object") {
  fail("schemaVersion 2 and deterministicCbor are required");
}

const fixtureClasses = deterministic.fixtureClasses;
if (fixtureClasses?.positive !== "golden" ||
    fixtureClasses?.negative !== "rejection-fixture" ||
    fixtureClasses?.resourceRejections !== "construction-recipe" ||
    fixtureClasses?.interoperability !== "interop-fixture") {
  fail("deterministicCbor.fixtureClasses does not declare the supported fixture classes");
}

const interoperability = deterministic.interoperability;
if (!Array.isArray(interoperability) || interoperability.length === 0) {
  fail("deterministicCbor.interoperability must be a non-empty array");
}
for (const [section, minimumCount] of Object.entries(deterministicMinimumCounts)) {
  const fixtures = deterministic[section];
  if (!Array.isArray(fixtures) || fixtures.length < minimumCount) {
    fail(`deterministicCbor.${section} must contain at least ${minimumCount} fixtures`);
  }
}

const nonCanonicalCborReasons = new Set([
  "duplicate-key",
  "non-minimal-integer",
  "non-minimal-length",
  "map-key-order",
  "trailing-data",
]);
for (const [index, fixture] of deterministic.negative.entries()) {
  const reason = requireString(fixture?.reason, `deterministicCbor.negative[${index}].reason`);
  const expectedClass = nonCanonicalCborReasons.has(reason)
    ? "non-canonical"
    : "invalid-input";
  if (fixture.errorClass !== expectedClass) {
    fail(`deterministicCbor.negative[${index}].errorClass must be ${expectedClass}`);
  }
}

const names = new Set();
for (const [index, fixture] of interoperability.entries()) {
  const path = `deterministicCbor.interoperability[${index}]`;
  const name = requireString(fixture?.name, `${path}.name`);
  if (names.has(name)) {
    fail(`duplicate fixture name ${name}`);
  }
  names.add(name);

  const fixtureKind = requireString(fixture?.fixtureKind, `${path}.fixtureKind`);
  if (fixtureKind !== "synthetic" && fixtureKind !== "captured") {
    fail(`${path}.fixtureKind must be synthetic or captured`);
  }
  requireString(fixture?.sourceRepo, `${path}.sourceRepo`);
  const sourceCommit = requireString(fixture?.sourceCommit, `${path}.sourceCommit`);
  if (fixtureKind === "synthetic" && sourceCommit !== "content-hash-pinned") {
    fail(`${path}.synthetic fixtures must use content-hash-pinned sourceCommit`);
  }
  if (fixtureKind === "captured" && sourceCommit === "content-hash-pinned") {
    fail(`${path}.captured fixtures must name an upstream commit or release`);
  }
  requireString(fixture?.source, `${path}.source`);
  requireString(fixture?.explanation, `${path}.explanation`);

  if (!Array.isArray(fixture?.sourceFiles) || fixture.sourceFiles.length === 0) {
    fail(`${path}.sourceFiles must be non-empty`);
  }
  for (const [sourceIndex, sourceFile] of fixture.sourceFiles.entries()) {
    const sourcePath = `${path}.sourceFiles[${sourceIndex}]`;
    const relativePath = requireString(sourceFile?.path, `${sourcePath}.path`);
    if (relativePath.startsWith("/") || relativePath.split("/").includes("..")) {
      fail(`${sourcePath}.path must be repository-relative`);
    }
    requireSha256(sourceFile?.sha256, `${sourcePath}.sha256`);
  }

  const hex = requireString(fixture?.hex, `${path}.hex`);
  if (!/^(?:[0-9a-f]{2})+$/u.test(hex)) {
    fail(`${path}.hex must contain lowercase complete bytes`);
  }
  if (!Number.isSafeInteger(fixture?.byteLength) || fixture.byteLength !== hex.length / 2) {
    fail(`${path}.byteLength must equal the encoded byte length`);
  }
  const actualDigest = createHash("sha256").update(Buffer.from(hex, "hex")).digest("hex");
  if (actualDigest !== requireSha256(fixture?.sha256, `${path}.sha256`)) {
    fail(`${path}.sha256 does not match ${path}.hex`);
  }
  if (!Number.isSafeInteger(fixture?.entryCount) || fixture.entryCount < 0) {
    fail(`${path}.entryCount must be a non-negative safe integer`);
  }
}

const pemBundle = manifest.pemBundleDecode;
if (pemBundle?.fixtureClass !== "golden-operation" || !Array.isArray(pemBundle.cases)) {
  fail("pemBundleDecode must declare golden-operation cases");
}
const pemNames = new Set();
for (const [index, fixture] of pemBundle.cases.entries()) {
  const path = `pemBundleDecode.cases[${index}]`;
  const name = requireString(fixture?.name, `${path}.name`);
  if (pemNames.has(name)) fail(`duplicate PEM bundle fixture ${name}`);
  pemNames.add(name);
  for (const field of ["requestHex", "responseHex"]) {
    if (!/^(?:[0-9a-f]{2})+$/u.test(requireString(fixture[field], `${path}.${field}`))) {
      fail(`${path}.${field} must contain lowercase complete bytes`);
    }
  }
  if (Object.keys(fixture.request ?? {}).join() !== "pemBundleDecode" ||
      typeof fixture.request.pemBundleDecode?.pem !== "string") {
    fail(`${path}.request must be a generated PEM bundle request`);
  }
  const response = fixture.response;
  if (Object.keys(response ?? {}).join() === "result") {
    const blocks = response.result?.pemBundleDecode?.blocks;
    if (Object.keys(response.result).join() !== "pemBundleDecode" || !Array.isArray(blocks) ||
        blocks.some((block) => typeof block.label !== "string" || typeof block.der !== "string")) {
      fail(`${path}.response must contain PEM bundle blocks`);
    }
  } else if (Object.keys(response ?? {}).join() === "error") {
    const error = response.error;
    const detail = error?.pem ?? error?.baseEncoding;
    if (error?.origin !== "CODEC_ERROR_ORIGIN_CALLER" ||
        !/^CODEC_ERROR_REASON_(?:PEM_[A-Z_]+|BASE_INPUT_TOO_LARGE)$/u.test(detail?.reason)) {
      fail(`${path}.response must carry a typed PEM failure and caller origin`);
    }
  } else {
    fail(`${path}.response must have exactly one generated outcome`);
  }
}
// Require contrasting scope/profile pairs as well as allocation-budget failures.
// All lanes compare the exact same binary envelopes, not locally built outputs.
for (const name of [
  "default-strict-all-mixed-labels", "explicit-strict-all-crlf", "strict-cr-only",
  "first-stops-before-malformed-tail", "all-rejects-malformed-tail",
  "strict-rejects-inline-armor", "compatible-accepts-inline-armor",
  "compatible-preserves-label-case-and-headers", "strict-rejects-form-feed-prefix",
  "strict-rejects-empty-input", "compatible-accepts-empty-input",
  "block-limit-exceeded", "exact-block-and-der-limits", "der-limit-exceeded",
  "input-limit-exceeded", "invalid-scope", "invalid-syntax-profile",
]) {
  if (!pemNames.has(name)) fail(`missing PEM bundle fixture ${name}`);
}

process.stdout.write(`validated ${interoperability.length} deterministic-CBOR provenance fixtures and ${pemNames.size} PEM bundle fixtures\n`);
