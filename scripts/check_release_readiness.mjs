#!/usr/bin/env node
// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const supportedArguments = new Set([
  "--generated-freshness",
  "--release-packages",
]);
const suppliedArguments = new Set();
for (const argument of process.argv.slice(2)) {
  if (!supportedArguments.has(argument)) {
    console.error(`release readiness check failed: unsupported argument ${argument}`); process.exit(1);
  }
  if (suppliedArguments.has(argument)) {
    console.error(`release readiness check failed: argument ${argument} was specified more than once`); process.exit(1);
  }
  suppliedArguments.add(argument);
}


// Direct invocation delegates to the pinned package runner. Its authenticated
// core and source-policy proof channel must govern every successful mode.
const runnerPath = fileURLToPath(new URL("./run_pinned_release_readiness.mjs", import.meta.url));
if (process.env.RELEASE_READINESS_CORE_URL === undefined) {
  const delegated = spawnSync(process.execPath, [runnerPath, ...process.argv.slice(2)], {
    cwd: fileURLToPath(new URL("../", import.meta.url)), stdio: "inherit",
  });
  process.exit(Number.isInteger(delegated.status) ? delegated.status : 1);
}
const pinCheck = spawnSync(process.execPath, [runnerPath, "--check-pins"], {
  stdio: ["ignore", "ignore", "inherit"],
});
if (pinCheck.error !== undefined || pinCheck.status !== 0) {
  console.error("release readiness check failed: readiness pins could not be verified");
  process.exit(1);
}

// Every local module this checker imports or spawns is pinned here. This file
// is itself pinned by the runner, so changing any policy module requires
// updating this table, which in turn trips the checker pin for review.
const POLICY_MODULE_SHA256 = new Map([
  ["packages/ts/scripts/lint-typescript.mjs", "ac3fbad064d2655773cc5c8476eda63e5e1cff0e53cceb6d9cf0faca923df89d"],
  ["scripts/check_swift_source_style.mjs", "24a53cb703ea5763ace64a7c6e43a96ec721235cfe6efec6b0bec134b43d0455"],
  ["scripts/release-readiness/sdk-source-policy.mjs", "9527f3842536ed78e8d4358c4a802f7ecf1669c6537d87d67a4d6ccb6e28011f"],
  ["scripts/generate_codec_sdk_contract.mjs", "0f0c49b1f3f8185c6599aa4123b8b432224e146760561a2d45d5e37d9dae9cd0"],
  ["scripts/codec_proto_schema.mjs", "4ee81a8a3c896d70a871bfb4a0724c007d709891a88580c3eed777bb80ddcbc9"],
  ["scripts/release-readiness/native-proto-policy.mjs", "f7713ec21913a67d1b4b7c32890f0bcb1981a5c0d0900649dacb923e70a3142f"],
  ["scripts/generate_codec_proto.mjs", "6bd8d27443bac3540d89bb7c0e95253028d7f8584efbb91797451e343c4ed043"],
  ["scripts/generate_codec_proto_support.mjs", "6ec4f0c7727003fd2d36e537a51f27a0f20bf5cbdf194de912857d94c0ba462a"],
  ["scripts/harden_codec_sdk_proto.mjs", "60636ba0bc757c45ec951ca13d79fedba29620f605a1c1d4e6b6d0c8c3c0d0ae"],
  ["scripts/release-readiness/source-policy.mjs", "1bc1368be565bfb972ef68a7b3a532c5b1969f40ef9893cd660fb1c244a4202e"],
  ["scripts/verify_release_version.mjs", "bc858e3f4bd98b808b5557b7c50755b4a0d3595393842d7cd01728f853d86215"],
  ["scripts/codec_proto_sensitivity.mjs", "e74e8459712b1091f4dc413fe32d9c6a4b160d90d110d22f9cb52601d5c3ca77"],
  ["scripts/validate_codec_vectors.mjs", "2441249555b41a4dddc20096ae56521a165058284f933bf418b074dd76420fb9"],
  ["scripts/generate_codec_limits.mjs", "9f566d26749206dffcabf2795b4ce5cd7f20d51ce6d863e0527b91b64d143256"],
]);
for (const [path, expected] of POLICY_MODULE_SHA256) {
  let actual;
  try {
    actual = createHash("sha256")
      .update(readFileSync(new URL(`../${path}`, import.meta.url)))
      .digest("hex");
  } catch {
    actual = undefined;
  }
  if (actual !== expected) {
    console.error(
      `release readiness check failed: ${path} does not match its reviewed pin; ` +
        "after reviewing the change, update POLICY_MODULE_SHA256 and refresh the checker pin",
    );
    process.exit(1);
  }
}

const { createReleaseReadinessContext, scrubCommentsForAssertion } =
  await import(process.env.RELEASE_READINESS_CORE_URL);
const { ReleaseVersionError, verifyCheckedOutReleaseVersion } =
  await import("./verify_release_version.mjs");
const {
  codecProtoProviderOutputMessages,
  codecProtoScalarFieldClassifications,
  codecProtoSensitiveNonTextFieldClassifications,
  codecProtoSensitiveOwnerMessages,
} = await import("./codec_proto_sensitivity.mjs");

const codecProtoSensitiveMessageNames = [...new Set([
  ...codecProtoScalarFieldClassifications
    .filter((classification) => classification.sensitivity === "sensitive")
    .map((classification) => classification.message),
  ...codecProtoSensitiveNonTextFieldClassifications
    .map((classification) => classification.message),
  ...codecProtoSensitiveOwnerMessages,
])];
const readinessContext = createReleaseReadinessContext({
  scriptUrl: import.meta.url,
  requireTrackedFiles: true,
});
const {
  readText,
  readJson,
  listFiles,
  fail,
  requireTracked,
  assertContains,
  assertNotContains,
  assertLockPackageVersion,
  assertNodeWorkflowJobsPinNode,
  assertProtoContract,
  assertReallyMeReleasePackagePolicy,
  assertRustSourcePolicy,
  assertCargoMetadataPolicy,
  assertSpdxHeaders,
  assertWorkflowActionsPinned,
  assertWorkflowUsesStepPinnedTo,
  extractWorkflowSteps,
  assertCargoFuzzWorkflowPolicy,
  assertWorkflowPermissionsPolicy,
} = readinessContext;

// This policy module executes inside the release gate. Requiring it explicitly
// prevents an untracked worktree file from influencing release readiness.
for (const path of POLICY_MODULE_SHA256.keys()) {
  requireTracked(path);
}
// Script regression suites run once per host in the repository policy job,
// whose glob is asserted below; running them here again would only repeat it.
try {
  execFileSync(process.execPath, ["scripts/validate_codec_vectors.mjs"], {
    stdio: "pipe",
  });
} catch {
  fail("scripts/validate_codec_vectors.mjs failed");
}
const {
  findManualProtoPresenceFields,
  sourceBlockFromNeedle,
  stripSourceComments,
  stripSourceStringsAndComments,
} = await import("./release-readiness/source-policy.mjs");

const assertSubstringsInOrder = (path, substrings) => {
  const contents = scrubCommentsForAssertion(path, readText(path));
  let previousIndex = -1;
  for (const substring of substrings) {
    const currentIndex = contents.indexOf(substring, previousIndex + 1);
    if (currentIndex < 0) {
      fail(`${path} is missing ordered release step: ${substring}`);
    }
    if (currentIndex <= previousIndex) {
      fail(`${path} has release steps in an unsafe order near: ${substring}`);
    }
    previousIndex = currentIndex;
  }
};

const assertMinOccurrences = (path, needle, expectedMin) => {
  const source = scrubCommentsForAssertion(path, readText(path));
  const count = source.split(needle).length - 1;
  if (count < expectedMin) {
    fail(`${path} contains ${needle} ${count} time(s), expected at least ${expectedMin}`);
  }
};

const rustProductionSource = (path) => {
  const source = readText(path);
  const testStart = source.indexOf("\n#[cfg(test)]");
  return testStart === -1 ? source : source.slice(0, testStart);
};

const blockFromNeedle = ({
  path,
  source,
  startNeedle,
  nextNeedle,
  label,
}) => {
  const block = sourceBlockFromNeedle({ source, startNeedle, nextNeedle });
  if (block === undefined) {
    fail(`${path} is missing ${label ?? startNeedle}`);
  }
  return block;
};

const assertBlockContains = ({
  path,
  block,
  needle,
  label,
}) => {
  if (!block.includes(needle)) {
    fail(`${path} ${label} is missing ${needle}`);
  }
  const quoteIndex = [needle.indexOf('"'), needle.indexOf("'"), needle.indexOf("`")]
    .filter((index) => index >= 0)
    .reduce((minimum, index) => Math.min(minimum, index), needle.length);
  const executableNeedle = needle.slice(0, quoteIndex).trimEnd();
  if (
    executableNeedle.length > 0 &&
    !stripSourceStringsAndComments(block).includes(executableNeedle)
  ) {
    fail(`${path} ${label} contains ${needle} only outside executable source`);
  }
};

const assertBlockNotContains = ({
  path,
  block,
  needle,
  label,
}) => {
  const executableBlock = stripSourceStringsAndComments(block);
  const quoted = needle.includes('"') || needle.includes("'") || needle.includes("`");
  if ((quoted && block.includes(needle)) || (!quoted && executableBlock.includes(needle))) {
    fail(`${path} ${label} must not contain ${needle}`);
  }
};

const assertRustCodecArm = ({
  path,
  arm,
  requiredNeedles,
  forbiddenNeedles = [],
}) => {
  const source = stripSourceComments(rustProductionSource(path));
  const block = blockFromNeedle({
    path,
    source,
    startNeedle: arm,
    nextNeedle: "\n        CODEC_",
    label: arm,
  });
  for (const needle of requiredNeedles) {
    assertBlockContains({ path, block, needle, label: arm });
  }
  for (const needle of forbiddenNeedles) {
    assertBlockNotContains({ path, block, needle, label: arm });
  }
};

const assertRustFunction = ({
  path,
  functionNeedle,
  requiredNeedles,
  forbiddenNeedles = [],
}) => {
  const source = stripSourceComments(rustProductionSource(path));
  const block = blockFromNeedle({
    path,
    source,
    startNeedle: functionNeedle,
    nextNeedle: "\nfn ",
    label: functionNeedle,
  });
  for (const needle of requiredNeedles) {
    assertBlockContains({ path, block, needle, label: functionNeedle });
  }
  for (const needle of forbiddenNeedles) {
    assertBlockNotContains({ path, block, needle, label: functionNeedle });
  }
};

const assertTypescriptExport = ({
  path,
  exportNeedle,
  requiredNeedles,
  forbiddenNeedles = [],
}) => {
  const source = stripSourceComments(readText(path));
  const block = blockFromNeedle({
    path,
    source,
    startNeedle: exportNeedle,
    nextNeedle: "\nexport const ",
    label: exportNeedle,
  });
  for (const needle of requiredNeedles) {
    assertBlockContains({ path, block, needle, label: exportNeedle });
  }
  for (const needle of forbiddenNeedles) {
    assertBlockNotContains({ path, block, needle, label: exportNeedle });
  }
};

const sortedUniqueMatches = (source, pattern) =>
  [...new Set([...source.matchAll(pattern)].map((match) => match[1]))].sort();

const assertSetEquals = ({ label, actual, expected }) => {
  const missing = expected.filter((value) => !actual.includes(value));
  const extra = actual.filter((value) => !expected.includes(value));
  if (missing.length > 0 || extra.length > 0) {
    fail(`${label} mismatch; missing=[${missing.join(", ")}] extra=[${extra.join(", ")}]`);
  }
};

const assertTypescriptProtoFacadeCompleteness = ({ facadePath, generatedPath }) => {
  const facadeSource = readText(facadePath);
  const generatedSource = readText(generatedPath);
  const valueBlock = facadeSource.match(/export\s*\{([\s\S]*?)\}\s*from/u);
  if (valueBlock === null) fail(`${facadePath} is missing its generated value export block`);
  const typeBlock = facadeSource.match(/export type\s*\{([\s\S]*?)\}\s*from/u);
  if (typeBlock === null) {
    fail(`${facadePath} is missing its generated type export block`);
  }

  assertSetEquals({
    label: `${facadePath} generated schema exports`,
    actual: sortedUniqueMatches(valueBlock[1], /\b(Codec[A-Za-z0-9_]+Schema)\b/gu),
    expected: sortedUniqueMatches(generatedSource, /export const (Codec[A-Za-z0-9_]+Schema)\b/gu),
  });
  assertSetEquals({
    label: `${facadePath} generated type exports`,
    actual: sortedUniqueMatches(typeBlock[1], /\b(Codec[A-Za-z0-9_]+)\b/gu),
    expected: sortedUniqueMatches(generatedSource, /export type (Codec[A-Za-z0-9_]+)\b/gu),
  });
};

assertReallyMeReleasePackagePolicy({ version: "0.7.0" });
assertRustSourcePolicy({ generatedPrefixes: ["crates/proto/src/generated"] });
assertCargoMetadataPolicy({
  metadataArgs: ["metadata", "--locked", "--format-version", "1", "--no-deps"],
  packages: [{
    name: "reallyme-codec-proto",
    dependencies: [{
      name: "reallyme-buffa", rename: "buffa", source: "registry",
      requirement: "=0.1.1", features: ["json", "zeroize"],
    }],
  }],
});
// Every governed tracked source must name the same copyright holder. Only
// generator output that cannot carry a header is excluded, with a typed reason.
assertSpdxHeaders({
  copyright: "SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited",
  license: "SPDX-License-Identifier: MIT OR Apache-2.0",
  exclusions: [
    { path: "crates/proto/src/generated/buffa", reason: "generated" },
    { path: "gen/kotlin", reason: "generated" },
  ],
  requireExclusionsMatched: true,
  requireExclusionReasons: true,
});
// Composite actions can hide additional third-party dependencies from the
// top-level workflow scan. Reject them until the checker recursively validates
// every local action dependency with the same full-SHA policy.
assertWorkflowActionsPinned({ allowLocalActions: false });
assertCargoFuzzWorkflowPolicy({
  gitSource: {
    url: "https://github.com/rust-fuzz/cargo-fuzz.git",
    revision: "984c861c8dfea28055254c5f1d2659ab2cd63f76",
  },
});

const codecPackageVersion = "0.4.0";
const codecProtoPackageVersion = "0.4.0";
assertContains("Package.swift", `let ffiArtifactVersion = "${codecPackageVersion}"`);
// The probes are separate Cargo workspaces. Root workspace metadata cannot
// detect a release bump that leaves their own lockfile package entries stale.
for (const [probePath, probeName] of [
  ["probes/jcs-feature-unification", "reallyme-jcs-feature-unification-probe"],
  ["probes/der-feature-unification", "reallyme-der-feature-unification-probe"],
]) {
  assertContains(
    `${probePath}/Cargo.toml`,
    `name = "${probeName}"\nversion = "${codecPackageVersion}"`,
  );
  assertLockPackageVersion(readText(`${probePath}/Cargo.lock`), probeName, codecPackageVersion);
}
try {
  verifyCheckedOutReleaseVersion(codecPackageVersion);
} catch (error) {
  if (error instanceof ReleaseVersionError) {
    fail(`release package version policy failed: ${error.code}`);
  }
  fail("release package version policy could not be checked");
}
const releasePackagesMode = suppliedArguments.has("--release-packages");
const generatedFreshnessMode = suppliedArguments.has("--generated-freshness");
const codecRustLeafCrates = [
  "crates/base64/Cargo.toml",
  "crates/base64url/Cargo.toml",
  "crates/cbor/Cargo.toml",
  "crates/hex/Cargo.toml",
  "crates/jcs/Cargo.toml",
  "crates/multibase/Cargo.toml",
  "crates/multicodec/Cargo.toml",
  "crates/multikey/Cargo.toml",
  "crates/pem/Cargo.toml",
];

assertNodeWorkflowJobsPinNode();

const rootCargo = readText("Cargo.toml");
for (const member of [
  '"crates/codec"',
  '"crates/ffi"',
  '"crates/wasm"',
  '"crates/proto"',
]) {
  assertContains("Cargo.toml", member);
}
if (rootCargo.includes("crates/crypto") || rootCargo.includes("crates/proto/crypto")) {
  fail("root Cargo.toml still references removed crypto crates");
}
assertContains("Cargo.toml", 'repository = "https://github.com/reallyme/codec"');
assertContains("Cargo.toml", 'buffa = { package = "reallyme-buffa", version = "=0.1.1", features = ["json", "zeroize"] }');
assertContains("README.md", "ReallyMe Codec is one cross-language codec operation contract for identity data.");
assertContains("README.md", "released in lockstep with `reallyme-codec`");
assertContains("README.md", "SDK consumers should usually start with the umbrella package");
assertContains("crates/codec/README.md", "ReallyMe Codec is one cross-language codec contract for identity data.");
assertContains("crates/codec/README.md", "released in lockstep");
assertContains("docs/rust-publishing.md", "`reallyme-codec` is the recommended public Rust entry point.");
assertContains("docs/rust-publishing.md", "They are released in lockstep");
for (const rustLeafCrate of codecRustLeafCrates) {
  const manifest = readText(rustLeafCrate);
  if (!manifest.includes(`version = "${codecPackageVersion}"`)) {
    fail(`${rustLeafCrate} is not versioned ${codecPackageVersion}`);
  }
  if (!manifest.includes("publish = true")) {
    fail(`${rustLeafCrate} must remain a publishable lockstep support crate`);
  }
  if (!manifest.includes('"/LICENSE"') || !manifest.includes('"/NOTICE"')) {
    fail(`${rustLeafCrate} must include LICENSE and NOTICE in its published package`);
  }
  const crateDirectory = rustLeafCrate.slice(0, rustLeafCrate.lastIndexOf("/"));
  readText(`${crateDirectory}/LICENSE`);
  readText(`${crateDirectory}/NOTICE`);
}

for (const rustPath of [
  ...listFiles("crates/codec").filter((path) => path.endsWith(".rs")),
  "crates/proto/src/lib.rs",
]) {
  const source = readText(rustPath);
  const publicEnum = /(?:^|\n)((?:#\[[^\n]+\]\n)*)pub enum\s+([A-Za-z0-9_]+)/gu;
  for (const match of source.matchAll(publicEnum)) {
    if (!match[1].includes("#[non_exhaustive]")) {
      fail(`${rustPath} public enum ${match[2]} must be #[non_exhaustive]`);
    }
  }
}

const codecCargo = readText("crates/codec/Cargo.toml");
if (!codecCargo.includes(`version = "${codecPackageVersion}"`)) {
  fail(`crates/codec/Cargo.toml is not versioned ${codecPackageVersion}`);
}
assertContains(
  "crates/codec/Cargo.toml",
  'include = ["/src/**/*.rs", "/Cargo.toml", "/README.md", "/LICENSE", "/NOTICE"]',
);
assertContains("crates/codec/README.md", `reallyme-codec = "${codecPackageVersion}"`);

assertContains("crates/ffi/Cargo.toml", 'name = "reallyme-codec-ffi"');
assertContains("Cargo.toml", "publish = false");
assertContains("crates/ffi/Cargo.toml", "publish.workspace = true");
assertContains("crates/ffi/src/codec.rs", "rm_codec_process_operation");
assertContains("crates/ffi/src/codec.rs", "rm_codec_process_operation_json");
assertContains("crates/ffi/src/codec.rs", "rm_codec_abi_version");
assertContains("crates/ffi/src/codec.rs", "rm_codec_max_operation_response_bytes");
assertContains("crates/ffi/src/codec.rs", "rm_codec_max_ffi_input_bytes");
assertContains("crates/ffi/src/codec.rs", "rm_codec_max_ffi_output_bytes");
assertContains("crates/ffi/src/codec.rs", "CODEC_ABI_VERSION");
assertContains("crates/ffi/src/codec.rs", "CODEC_ABI_VERSION: u32 = codec_proto::CODEC_PROVIDER_ABI_VERSION");
assertContains("crates/proto/src/provider_limits.rs", "CODEC_PROVIDER_ABI_VERSION: u32 = 6");
assertContains("crates/codec/src/operation_contract/run_contract.rs", "include!(\"dispatch.rs\")");
assertContains("crates/codec/src/operation_contract/dispatch.rs", "pub fn process_operation_response");
assertContains(
  "crates/codec/src/operation_contract/tests/dispatch_multiformat.rs",
  "binary_and_proto_json_dispatch_match",
);
assertContains("crates/wasm/src/proto_output.rs", "js_name = processOperation");
assertContains("crates/ffi/src/kotlin_codec.rs", "processOperationNative");
assertContains("crates/ffi/src/codec.rs", "MAX_CODEC_FFI_INPUT_BYTES");
assertNotContains("crates/ffi/src/codec.rs", "struct FixedJsonWriter");
assertNotContains("crates/ffi/src/codec.rs", "fn json_bytes");
assertNotContains("crates/ffi/src/status.rs", "CODEC_PROTO_ERROR");
assertNotContains("vectors/codec-vectors.json", "dagCborTaggedJson");
assertNotContains("vectors/codec-vectors.json", "dagCborCanonicalTaggedJson");
for (const structuredJsonNeedle of [
  "codec_spec_proto(&spec).map_err(|_| CODEC_INTERNAL_ERROR)?",
  "multicodec_table_result_proto(&table).map_err(|_| CODEC_INTERNAL_ERROR)?",
  "multikey_parse_result_proto(parsed).map_err(|_| CODEC_INTERNAL_ERROR)?",
  "dag_cbor_verify_cid_result_proto(verification)",
  "pem_decode_result_proto(decoded).map_err(|_| CODEC_INTERNAL_ERROR)?",
]) {
  assertNotContains("crates/ffi/src/codec.rs", structuredJsonNeedle);
}
for (const forbiddenStructuredJsonNeedle of [
  "serde_json::json!({\n                \"name\"",
  "serde_json::json!({\n                \"codecName\"",
  "serde_json::json!({\n                \"expectedCid\"",
  "serde_json::json!({\n                \"der\"",
  "struct MultikeyParseJson",
  "struct DagCborVerifyCidJson",
  "struct PemDecodeJson",
]) {
  assertNotContains("crates/ffi/src/codec.rs", forbiddenStructuredJsonNeedle);
}
for (const removedDagCborJsonNeedle of [
  "enum TaggedCborValue",
  "fn tagged_to_cbor(",
  "fn cbor_to_tagged(",
  "json_bytes(cbor_to_tagged(value)?)",
]) {
  assertNotContains("crates/ffi/src/codec.rs", removedDagCborJsonNeedle);
}
for (const removedWasmDagCborJsonNeedle of [
  "enum TaggedCborValue",
  "fn tagged_to_cbor(",
  "fn cbor_to_tagged(",
  "serde_json::from_str(&value_json).map_err(|_| invalid_input())?",
  "serde_json::to_string(&tagged).map_err(|_| provider_failure())",
  "js_name = dagCborEncode",
  "js_name = dagCborDecode",
]) {
  assertNotContains("crates/wasm/src/cbor.rs", removedWasmDagCborJsonNeedle);
}
for (const typedDagCborNeedle of [
  "CodecDagCborEncodeRequest dag_cbor_encode = 3001;",
  "CodecDagCborDecodeRequest dag_cbor_decode = 3002;",
  "CodecDagCborEncodeResult dag_cbor_encode = 3001;",
  "CodecDagCborDecodeResult dag_cbor_decode = 3002;",
]) {
  assertContains("crates/proto/proto/reallyme/codec/v1/codec.proto", typedDagCborNeedle);
}
for (const rustTypedDagCborNeedle of [
  "CodecOperation::DagCborEncode(request)",
  "CodecOperation::DagCborDecode(request)",
]) {
  assertContains("crates/codec/src/operation_contract/dispatch.rs", rustTypedDagCborNeedle);
}
for (const rustTypedDagCborNeedle of [
  "encode_dag_cbor_value(&value)",
  "decode_dag_cbor_value(encoded)",
]) {
  assertContains("crates/codec/src/operation_contract/execute_documents.rs", rustTypedDagCborNeedle);
}
assertTypescriptExport({
  path: "packages/ts/src/processCbor.ts",
  exportNeedle: "export const dagCborEncode = (value: ReallyMeCborValue): Uint8Array =>",
  requiredNeedles: [
    "processGeneratedOperationRequest(",
    'case: "dagCborEncode"',
    'operationResult.result.case !== "dagCborEncode"',
    "clearGeneratedOperationResult(operationResult)",
    "result.encoded.fill(0)",
    "wipeProtoDeterministicCborValueBytes(protoValue)",
    "wipeDagCborValueBytes(normalized)",
  ],
  forbiddenNeedles: [
    "processGeneratedProtoRequest(",
    "protoPayloadOrThrow(",
    "CodecDagCborEncodeResultSchema",
    "base64urlDecode(value.value)",
  ],
});
assertTypescriptExport({
  path: "packages/ts/src/processCbor.ts",
  exportNeedle: "export const dagCborDecode = (bytes: Uint8Array): ReallyMeCborValue =>",
  requiredNeedles: [
    "processGeneratedOperationRequest(",
    'case: "dagCborDecode"',
    'operationResult.result.case !== "dagCborDecode"',
    "clearGeneratedOperationResult(operationResult)",
    "wipeProtoDeterministicCborValueBytes(result.value)",
    "requestBytes.fill(0)",
  ],
  forbiddenNeedles: [
    "processGeneratedProtoRequest(",
    "protoPayloadOrThrow(",
    "CodecDagCborDecodeResultSchema",
    "base64urlEncode(value.value)",
  ],
});
assertContains("packages/ts/src/cborValues.ts", 'Readonly<{ type: "bytes"; value: Uint8Array }>');
assertContains("packages/ts/src/convertDagCbor.ts", "value: snapshotProviderBytes(");
assertContains("packages/ts/src/convertDagCbor.ts", "const entries: CodecDeterministicCborMapEntry[] = []");
assertContains("packages/ts/src/convertDagCbor.ts", "const values: ReallyMeCborValue[] = []");
assertContains("packages/ts/src/convertDagCbor.ts", "const bytes = snapshotDeterministicCborBytes(");
assertContains("packages/ts/test/reallyme-codec.test.mjs", 'value: { type: "bytes", value: bytes(0, 1, 2) }');
for (const removedTsDagCborJsonNeedle of [
  "function cborValueForJson",
  "const cborValueForJson",
  "function readCborValue",
  "const readCborValue",
  "stringifyBoundaryJson(cborValueForJson",
  "requireReallyMeCodecWasmProvider().dagCborEncode",
  "requireReallyMeCodecWasmProvider().dagCborDecode",
]) {
  for (const module of ["cbor", "processCbor", "convertDagCbor", "validateDagCbor"]) {
    assertNotContains(`packages/ts/src/${module}.ts`, removedTsDagCborJsonNeedle);
  }
}
assertContains("packages/swift/Sources/ReallyMeCodec/ProcessCbor.swift", ".dagCborEncode(request)");
assertContains("packages/swift/Sources/ReallyMeCodec/ProcessCbor.swift", ".dagCborDecode(request)");
assertContains("packages/swift/Sources/ReallyMeCodec/ProcessCbor.swift", "processGeneratedOperation(request: request)");
assertContains("packages/swift/Sources/ReallyMeCodec/ProcessCbor.swift", "case .dagCborEncode(var result)?");
assertContains("packages/swift/Sources/ReallyMeCodec/ProcessCbor.swift", "case .dagCborDecode(var result)?");
assertNotContains("packages/swift/Sources/ReallyMeCodec/DeterministicCbor.swift", "deterministicCborPayload(request:");
assertNotContains("packages/swift/Sources/ReallyMeCodec/StructuredResults.swift", "serializedBytes: resultBytes");
assertNotContains("packages/swift/Sources/ReallyMeCodec/StructuredResults.swift", "from resultBytes");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ConvertCborProto.kt", ".setDagCborEncode(");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ConvertCborProto.kt", ".setDagCborDecode(");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "processOperation(dagCborEncodeRequest(value))");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "CodecOperationResult.ResultCase.DAG_CBOR_ENCODE");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "CodecOperationResult.ResultCase.DAG_CBOR_DECODE");
assertNotContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "processProtoPayload(dagCborEncodeRequest");
assertNotContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "processProtoPayload(dagCborDecodeRequest");
assertNotContains("packages/kotlin/src/main/kotlin/me/really/codec/StructuredResults.kt", "resultBytes: ByteArray");
assertNotContains("packages/kotlin/src/main/kotlin/me/really/codec/StructuredResults.kt", "parseFrom(resultBytes)");
assertContains("crates/ffi/src/codec/tests.rs", "retired_scalar_structured_json_ids_fail_closed");
assertContains("crates/ffi/src/codec.rs", "CODEC_PEM_ENCODE");
assertNotContains("crates/ffi/src/codec.rs", "fn typed_dag_cbor_scalar_payload");
assertNotContains("crates/ffi/src/codec.rs", "CodecDagCborEncodeRequest");
assertNotContains("crates/ffi/src/codec.rs", "CodecDagCborDecodeRequest");
assertContains("crates/ffi/src/codec.rs", "CODEC_ABI_VERSION: u32 = codec_proto::CODEC_PROVIDER_ABI_VERSION");
assertContains("crates/proto/src/provider_limits.rs", "CODEC_PROVIDER_ABI_VERSION: u32 = 6");
assertNotContains("packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift", "dagCborEncode(taggedJson");
assertNotContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "dagCborEncode(taggedJson");
assertNotContains("packages/ts/src/cbor.ts", "bytesBase64url");
assertNotContains("packages/ts/src/cbor.ts", "string(value: string)");
assertContains("crates/jcs/src/canonicalize.rs", "canonicalize_json_text");
assertContains("crates/jcs/src/parse_json.rs", "serde_json::Deserializer::from_str(input)");
assertContains("crates/ffi/src/codec.rs", "initialize_output_length(output_ptr");
assertContains("crates/ffi/src/codec.rs", "write_i32(result_out, 0)");
assertNotContains("crates/ffi/src/codec.rs", "serde_json::to_vec(&cbor_to_tagged");
assertContains("crates/ffi/src/lib.rs", '#[cfg(not(panic = "unwind"))]');
assertContains("crates/ffi/src/lib.rs", "compile_error!");
assertContains("crates/ffi/src/guard.rs", "with_redacted_panic_hook");
assertContains("crates/ffi/src/guard.rs", "INSIDE_NATIVE_BOUNDARY");
assertContains("crates/ffi/src/kotlin_codec.rs", "with_redacted_panic_hook");
assertContains("crates/pem/src/encode.rs", "String::with_capacity(output_length)");
assertContains("crates/pem/src/decode.rs", "String::with_capacity(body_capacity)");
assertNotContains("crates/pem/src/decode.rs", '.replace("\\r\\n"');
assertContains("crates/codec/src/jcs.rs", "canonicalize_json_text");
assertContains("crates/codec/src/jcs.rs", "canonicalize_trusted_json_value");
assertNotContains("crates/codec/src/jcs.rs", "canonicalize_json, JcsError");
assertNotContains("crates/jcs/src/lib.rs", "canonicalize_json;");
assertNotContains("crates/jcs/src/canonicalize.rs", "pub fn canonicalize_json(");
assertNotContains("crates/jcs/tests/jcs_tests.rs", "deprecated_value_alias");
assertContains("crates/jcs/src/canonicalize.rs", "validate_interoperable_float_integer");
assertContains("crates/jcs/src/canonicalize.rs", "MIN_INTEROPERABLE_INTEGER_F64");
assertContains(
  "crates/jcs/tests/jcs_tests.rs",
  "integer_valued_binary64_numbers_outside_interoperable_range_are_rejected",
);
assertContains("crates/jcs/tests/jcs_tests.rs", '"1e19"');
assertContains("crates/jcs/tests/jcs_tests.rs", '"9007199254740992.0"');
assertContains("crates/ffi/src/codec.rs", "validate_boundary_input_lengths");
assertContains(
  "crates/ffi/src/codec/operation.rs",
  "resource_limit_operation_response()",
);
assertContains(
  "crates/ffi/src/kotlin_codec.rs",
  "validate_managed_input_lengths",
);
assertContains("crates/ffi/src/kotlin_codec/map_error.rs", "aggregate.checked_add(length)");
assertContains("crates/wasm/Cargo.toml", 'name = "reallyme-codec-wasm"');
assertContains("crates/wasm/src/boundary.rs", "MAX_WASM_INPUT_BYTES");
assertContains("crates/wasm/src/boundary.rs", "checked_add");
assertContains("crates/wasm/src/boundary.rs", "validate_js_inputs");
assertContains("crates/wasm/src/boundary.rs", "utf8_byte_len_for_js_string");
assertContains("crates/wasm/src/boundary.rs", "value.iter().peekable()");
assertNotContains("crates/wasm/src/boundary.rs", "MAX_WASM_STRING_CODE_UNITS");
assertContains("crates/wasm/src/boundary.rs", "zeroizing_string");
assertContains("crates/wasm/src/boundary.rs", "zeroizing_bytes_with_maximum");
assertContains("crates/wasm/src/boundary.rs", "checked_subarray(value,");
assertContains("crates/wasm/src/boundary.rs", "snapshot.fill");
for (const wasmSourcePath of [
  "crates/wasm/src/base_encoding.rs",
  "crates/wasm/src/cbor.rs",
  "crates/wasm/src/jcs.rs",
  "crates/wasm/src/multiformat.rs",
  "crates/wasm/src/proto_output.rs",
]) {
  const wasmSource = readText(wasmSourcePath);
  if (wasmSource.includes(".to_vec()")) {
    fail(`${wasmSourcePath} copies caller-owned JavaScript storage without a fixed snapshot`);
  }
  for (const exportBlock of wasmSource.split("#[wasm_bindgen").slice(1)) {
    const signatureEnd = exportBlock.indexOf("{");
    const signature = exportBlock.slice(0, signatureEnd);
    if (signature.includes("&str") || signature.includes("Option<String>")) {
      fail(`${wasmSourcePath} exports a string that wasm-bindgen copies before boundary validation`);
    }
  }
  for (const forbiddenJsResultShaper of [
    "Object::new(",
    "Array::new(",
    "Reflect::set(",
  ]) {
    if (rustProductionSource(wasmSourcePath).includes(forbiddenJsResultShaper)) {
      fail(`${wasmSourcePath} restores adapter-local JavaScript result shaping with ${forbiddenJsResultShaper}`);
    }
  }
}
for (const boundaryPath of [
  "crates/ffi/Cargo.toml",
  "crates/wasm/Cargo.toml",
  "crates/ffi/src/codec.rs",
  "crates/wasm/src/base_encoding.rs",
  "crates/wasm/src/cbor.rs",
  "crates/wasm/src/jcs.rs",
  "crates/wasm/src/multiformat.rs",
]) {
  for (const primitiveName of [
    "codec_base64",
    "codec_base64url",
    "codec_cbor",
    "codec_hex",
    "codec_jcs",
    "codec_multibase",
    "codec_multicodec",
    "codec_multikey",
  ]) {
    assertNotContains(boundaryPath, primitiveName);
  }
}
assertContains("crates/wasm/src/proto_output.rs", "process_operation");
assertContains("crates/wasm/src/proto_output.rs", "process_operation_json");
for (const staleWasmProtoExport of [
  "multicodec_prefix_for_name_proto",
  "multicodec_lookup_prefix_proto",
  "multicodec_table_proto",
  "multikey_parse_proto",
  "dag_cbor_verify_cid_proto",
  "pem_decode_proto",
]) {
  assertNotContains(
    "crates/wasm/src/proto_output.rs",
    staleWasmProtoExport,
  );
}
assertNotContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt",
  "PemDecodeOptionsJsonParser",
);
assertNotContains(
  "packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift",
  "private func pemDecodeOptions(",
);
assertContains(
  "packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift",
  "public func processOperation(_ request: [UInt8])",
);
assertContains(
  "packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift",
  "public func processOperationJson(_ requestJson: [UInt8])",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt",
  "public fun processOperation(request: ByteArray)",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt",
  "public fun processOperationJson(requestJson: ByteArray)",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt",
  "public fun decodePem(",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodecNative.kt",
  "processOperationNative",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/CallCodec.kt",
  "internal fun <T> processOperation(",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/CallCodec.kt",
  "UnsafeByteOperations.unsafeWrap(responseBytes).newCodedInput()",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/CallCodec.kt",
  "input.enableAliasing(true)",
);
for (const resultCase of [
  "MULTICODEC_PREFIX_FOR_NAME",
  "MULTICODEC_LOOKUP_PREFIX",
  "MULTICODEC_TABLE",
  "MULTIKEY_PARSE",
  "DAG_CBOR_VERIFY_CID",
  "PEM_DECODE",
]) {
  assertContains(
    "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt",
    `CodecOperationResult.ResultCase.${resultCase}`,
  );
}
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/CallCodec.kt",
  "responseBytes.fill(0)",
);
assertContains(
  "packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift",
  "public func decodePem(",
);
assertNotContains("crates/multikey/src/error.rs", "&'static str");
assertContains(
  "scripts/build_swift_xcframework.sh",
  "rm_codec_process_operation_json",
);
assertContains("scripts/build_swift_xcframework.sh", "rm_codec_process_operation");
assertContains(".github/workflows/code-checks.yml", "cargo-nextest@0.9.151");
assertContains(".github/workflows/code-checks.yml", "cargo-deny@0.20.2");
assertContains(".github/workflows/code-checks.yml", "cargo-audit@0.23.0");
assertContains(".github/workflows/code-checks.yml", "scripts/audit_committed_lockfiles.sh");
assertContains(
  ".github/workflows/code-checks.yml",
  'node --test --test-concurrency=4 "scripts/**/*.test.mjs" "packages/ts/scripts/*.test.mjs"',
);
assertContains(".github/workflows/code-checks.yml", "bash scripts/install_actionlint.sh");
assertContains(".github/workflows/code-checks.yml", "run: actionlint");
const repositoryToolingSteps = extractWorkflowSteps(".github/workflows/code-checks.yml")
  .filter((step) => step.job === "repository-policy");
for (const stepName of ["Install ShellCheck", "Install actionlint", "Lint workflows"]) {
  const steps = repositoryToolingSteps.filter((step) => step.name === stepName);
  if (steps.length !== 1 || steps[0].condition !== "matrix.os == 'ubuntu-24.04'") {
    fail(`${stepName} must run once in the Linux repository-policy lane`);
  }
}
assertContains(
  ".github/workflows/code-checks.yml",
  "python3 -m unittest discover -s scripts -p '*_test.py'",
);
assertContains(
  ".github/workflows/code-checks.yml",
  "RUSTFLAGS=-Dwarnings cargo check --workspace --all-features --locked",
);
assertNotContains(
  ".github/workflows/code-checks.yml",
  "run: cargo check --workspace --all-features --locked",
);
assertContains(".github/workflows/code-checks.yml", "--check-pins");
assertContains(
  ".github/workflows/code-checks.yml",
  "node scripts/run_pinned_release_readiness.mjs --check-pins --verify-upstream",
);
assertContains(".github/workflows/protobuf-ci.yml", "--check-pins");
assertContains(
  ".github/workflows/code-checks.yml",
  "node scripts/run_pinned_release_readiness.mjs",
);
assertContains(
  "scripts/run_pinned_release_readiness.mjs",
  'const RELEASE_READINESS_COMMIT = "18a6d91d99837e80f92e41fe04effb0b043d754e"',
);
assertContains(
  "scripts/run_pinned_release_readiness.mjs",
  '"7614b0cbf9e36a4653c2fa9008bee0d1e22542de0e4a08a10e640c836a5c93db"',
);
assertContains(
  "scripts/run_pinned_release_readiness.mjs",
  "LOCAL_CHECKER_SHA256",
);
assertContains(
  "scripts/run_pinned_release_readiness.mjs",
  "MAX_CHECKER_BYTES = 524_288",
);
assertContains(
  "scripts/run_pinned_release_readiness.mjs",
  "local checker does not match the reviewed repository policy pin",
);
assertContains("scripts/run_pinned_release_readiness.mjs", "fileURLToPath(officialRunnerUrl)");
assertContains("scripts/run_pinned_release_readiness.mjs", "RELEASE_READINESS_RUNNER_SHA256");
assertContains(
  "scripts/release-readiness/source-policy.test.mjs",
  "TypeScript block comments do not nest or hide following code",
);
assertContains(
  "scripts/release-readiness/source-policy.test.mjs",
  "regex literals with quotes do not blank following executable code",
);
const gradleWrapperSteps = extractWorkflowSteps(".github/workflows/code-checks.yml")
  .filter((step) => step.name === "Validate Gradle wrapper");
if (
  gradleWrapperSteps.length !== 1 ||
  !/^gradle\/actions\/wrapper-validation@[0-9a-f]{40}$/u.test(gradleWrapperSteps[0].uses ?? "")
) {
  fail("Code Checks must use the Gradle wrapper validator at a full commit SHA");
}

const codecProtoCargo = readText("crates/proto/Cargo.toml");
if (!codecProtoCargo.includes(`version = "${codecProtoPackageVersion}"`)) {
  fail(`crates/proto/Cargo.toml is not versioned ${codecProtoPackageVersion}`);
}
assertContains("crates/proto/Cargo.toml", 'name = "reallyme-codec-proto"');
assertContains(
  "crates/proto/Cargo.toml",
  '"/proto/**/*.proto"',
);
assertContains(
  "crates/proto/Cargo.toml",
  '"/tests/**/*.rs"',
);
assertContains(
  "crates/proto/README.md",
  `reallyme-codec-proto = { version = "${codecProtoPackageVersion}", features = ["generated"] }`,
);
assertProtoContract("crates/proto/proto/reallyme/codec/v1/codec.proto");
if (findManualProtoPresenceFields(readText("crates/proto/proto/reallyme/codec/v1/codec.proto")).length !== 0) {
  fail("codec.proto must use optional/message/oneof presence instead of manual presence booleans");
}
const { assertNativeOperationBoundary } = await import("./release-readiness/native-proto-policy.mjs");
assertNativeOperationBoundary(readinessContext, {
  protoPath: "crates/proto/proto/reallyme/codec/v1/codec.proto",
  operationRequest: "CodecOperationRequest",
  operationResponse: "CodecOperationResponse",
  protoReadme: "crates/proto/README.md",
  protoCargo: "crates/proto/Cargo.toml",
  wirePath: "crates/codec/src/operation_contract/dispatch.rs",
  codecPath: "crates/proto/src/wire.rs",
  binaryResponseNeedle: "DecodeOptions::new()",
  allowServices: false,
  sdkAdapters: [
    {
      path: "crates/wasm/src/proto_output.rs",
      processOperationNeedle: "pub fn process_operation(",
      processOperationJsonNeedle: "pub fn process_operation_json(",
      binaryResponseNeedle: "process_operation_response_request(request.as_slice())",
      requiredNeedles: [
        "process_operation_response_request(request.as_slice())",
        "process_operation_response_json_request(request_json.as_slice())",
      ],
    },
    {
      path: "packages/ts/src/operationContract.ts",
      processOperationNeedle: "export const processOperation =",
      processOperationJsonNeedle: "export const processOperationJson =",
    },
    {
      path: "packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift",
      processOperationNeedle: "public func processOperation(_ request: [UInt8])",
      processOperationJsonNeedle:
        "public func processOperationJson(_ requestJson: [UInt8])",
    },
    {
      path: "packages/swift/Sources/ReallyMeCodec/CallCodecWithRustCAbi.swift",
      processOperationNeedle: "func processOperation(request: [UInt8])",
      processOperationJsonNeedle:
        "func processOperationJson(request: [UInt8])",
      binaryResponseNeedle: "processOperationResponse(",
      requiredNeedles: [
        "private typealias CodecSizeLimitFunction = @convention(c) () -> UInt",
        "import ReallyMeCodecFFI",
        "private let maxFfiInputLength: Int",
        "private let maxFfiOutputLength: Int",
        "private let maxOperationResponseLength: Int",
        "rm_codec_max_ffi_input_bytes()",
        "rm_codec_max_ffi_output_bytes()",
        "rm_codec_max_operation_response_bytes()",
        '"rm_codec_max_ffi_input_bytes"',
        '"rm_codec_max_ffi_output_bytes"',
        '"rm_codec_max_operation_response_bytes"',
        "static func requireValidOperationResponseLimit<Limit: BinaryInteger>(",
        "Int(exactly: limit)",
        "guard firstStatus == ReallyMeCodecRustCAbiStatus.bufferTooSmall else",
        "producedLength <= maxOperationResponseLength",
        "guard producedLength == output.count else",
        '"rm_codec_abi_version"',
      ],
    },
    {
      path: "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt",
      processOperationNeedle: "public fun processOperation(request: ByteArray)",
      processOperationJsonNeedle:
        "public fun processOperationJson(requestJson: ByteArray)",
      binaryResponsePath: "packages/kotlin/src/main/kotlin/me/really/codec/CallCodec.kt",
      binaryResponseNeedle: "CodecOperationResponse.parseFrom(input)",
    },
  ],
});
assertContains(
  "packages/swift/Tests/ReallyMeCodecTests/ProviderSafetyTests.swift",
  "UInt(Int.max) + 1",
);

const codecProtoSchemaPath = "crates/proto/proto/reallyme/codec/v1/codec.proto";
const codecContractDispatchPath = "crates/codec/src/operation_contract/dispatch.rs";
const codecContractExecuteMultiformatPath = "crates/codec/src/operation_contract/execute_multiformat.rs";
const codecContractExecuteDocumentsPath = "crates/codec/src/operation_contract/execute_documents.rs";
const codecContractDecodeDocumentsPath = "crates/codec/src/operation_contract/decode_documents.rs";
const codecContractCopyLimitsPath = "crates/codec/src/operation_contract/copy_limits.rs";
const codecContractShapePath = "crates/codec/src/operation_contract/shape_results.rs";
const codecContractMapErrorsPath = "crates/codec/src/operation_contract/map_errors.rs";
const codecFfiPath = "crates/ffi/src/codec.rs";
const codecWasmProviderPath = "packages/ts/src/wasmProvider.ts";

const codecOperationResultSchemaBlock = blockFromNeedle({
  path: codecProtoSchemaPath,
  source: readText(codecProtoSchemaPath),
  startNeedle: "message CodecOperationResult {",
  nextNeedle: "\nmessage CodecOperationResponse {",
  label: "fully discriminated operation result schema",
});
assertBlockContains({
  path: codecProtoSchemaPath,
  block: codecOperationResultSchemaBlock,
  needle: "reserved 1 to 999;",
  label: "fully discriminated operation result schema",
});

const generatedStructuredOperations = [
  {
    name: "multicodec prefix-for-name",
    protoOperationField:
      "CodecMulticodecPrefixForNameRequest multicodec_prefix_for_name = 1000;",
    protoRequestMessage: "message CodecMulticodecPrefixForNameRequest",
    protoResultMessage: "message CodecMulticodecSpec",
    protoVariant: "CodecOperation::MulticodecPrefixForName(request) => {",
    rustPath: codecContractExecuteMultiformatPath,
    contractFunction: "fn process_multicodec_prefix_for_name(",
    contractNeedles: [
      "prefix_for_name(name).map_err(multicodec_boundary_error)",
      "codec_spec_proto(&spec)",
    ],
    tsPath: "packages/ts/src/multiformat.ts",
    tsExport: "export const multicodecPrefixForName = (",
    tsNeedles: [
      "processGeneratedOperationRequest(",
      'result.result.case !== "multicodecPrefixForName"',
      "readMulticodecMetadata(result.result.value)",
    ],
    forbiddenProviderCall: "requireReallyMeCodecWasmProvider().multicodecPrefixForName",
  },
  {
    name: "multicodec lookup-prefix",
    protoOperationField:
      "CodecMulticodecLookupPrefixRequest multicodec_lookup_prefix = 1001;",
    protoRequestMessage: "message CodecMulticodecLookupPrefixRequest",
    protoResultMessage: "message CodecMulticodecLookupResult",
    protoVariant: "CodecOperation::MulticodecLookupPrefix(request) => {",
    rustPath: codecContractExecuteMultiformatPath,
    contractFunction: "fn process_multicodec_lookup_prefix(",
    contractNeedles: [
      "lookup_prefix(value).map_err(multicodec_boundary_error)",
      "multicodec_lookup_result_proto(&found)",
    ],
    tsPath: "packages/ts/src/multiformat.ts",
    tsExport: "export const multicodecLookupPrefix = (",
    tsNeedles: [
      "processGeneratedOperationRequest(",
      "bytesSnapshot.fill(0)",
      'operationResult.result.case !== "multicodecLookupPrefix"',
      "requireNoProviderUnknownFields(result)",
    ],
    forbiddenProviderCall: "requireReallyMeCodecWasmProvider().multicodecLookupPrefix",
  },
  {
    name: "multicodec table",
    protoOperationField: "CodecMulticodecTableRequest multicodec_table = 1002;",
    protoRequestMessage: "message CodecMulticodecTableRequest",
    protoResultMessage: "message CodecMulticodecTableResult",
    protoVariant: "CodecOperation::MulticodecTable(_) => {",
    rustPath: codecContractExecuteMultiformatPath,
    contractFunction: "fn process_multicodec_table(",
    contractNeedles: [
      "supported_table().map_err(multicodec_boundary_error)",
      "multicodec_table_result_proto(&table)",
    ],
    tsPath: "packages/ts/src/multiformat.ts",
    tsExport: "export const multicodecTable = (): ReallyMeMulticodecTable => {",
    tsNeedles: [
      "processGeneratedOperationRequest(",
      'operationResult.result.case !== "multicodecTable"',
    ],
    forbiddenProviderCall: "requireReallyMeCodecWasmProvider().multicodecTable",
  },
  {
    name: "multikey parse",
    protoOperationField: "CodecMultikeyParseRequest multikey_parse = 2000;",
    protoRequestMessage: "message CodecMultikeyParseRequest",
    protoResultMessage: "message CodecMultikeyParseResult",
    protoVariant: "CodecOperation::MultikeyParse(request) => {",
    rustPath: codecContractExecuteMultiformatPath,
    contractFunction: "fn process_multikey_parse(",
    contractNeedles: [
      "parse_multikey(multikey).map_err(multikey_boundary_error)",
      "multikey_parse_result_proto(parsed)",
    ],
    tsPath: "packages/ts/src/multiformat.ts",
    tsExport: "export const multikeyParse = (multikey: string): ReallyMeParsedMultikey =>",
    tsNeedles: [
      "processGeneratedOperationRequest(",
      'operationResult.result.case !== "multikeyParse"',
      "result.publicKey.fill(0)",
    ],
    forbiddenProviderCall: "requireReallyMeCodecWasmProvider().multikeyParse",
  },
  {
    name: "DAG-CBOR CID verification",
    protoOperationField:
      "CodecDagCborVerifyCidRequest dag_cbor_verify_cid = 3000;",
    protoRequestMessage: "message CodecDagCborVerifyCidRequest",
    protoResultMessage: "message CodecDagCborVerifyCidResult",
    protoVariant: "CodecOperation::DagCborVerifyCid(request) => {",
    rustPath: codecContractExecuteMultiformatPath,
    contractFunction: "fn process_dag_cbor_verify_cid(",
    contractNeedles: [
      "verify_dag_cbor_cid(cid, payload).map_err(dag_cbor_boundary_error)",
      "dag_cbor_verify_cid_result_proto(verification)",
    ],
    tsPath: "packages/ts/src/processCbor.ts",
    tsExport:
      "export const dagCborVerifyCidDetails = (\n  cid: string,\n  bytes: Uint8Array,",
    tsNeedles: [
      "processGeneratedOperationRequest(",
      "payloadSnapshot.fill(0)",
      'operationResult.result.case !== "dagCborVerifyCid"',
    ],
    forbiddenProviderCall: "requireReallyMeCodecWasmProvider().dagCborVerifyCid",
  },
  {
    name: "PEM decode",
    protoOperationField: "CodecPemDecodeRequest pem_decode = 4000;",
    protoRequestMessage: "message CodecPemDecodeRequest",
    protoResultMessage: "message CodecPemDecodeResult",
    protoVariant: "CodecOperation::PemDecode(request) => {",
    rustPath: codecContractExecuteDocumentsPath,
    contractFunction: "fn process_pem_decode(",
    contractNeedles: [
      "decode_pem(",
      ".map_err(pem_boundary_error)?",
      ".map_err(|_| internal_wire_error())?",
      "pem_decode_result_proto(decoded)",
    ],
    tsPath: "packages/ts/src/pem.ts",
    tsExport: "export const decodePem = (",
    tsNeedles: [
      "processGeneratedOperationRequest(",
      'case: "pemDecode"',
      "pemSnapshot.fill(0)",
      'operationResult.result.case !== "pemDecode"',
      "readPemDocument(operationResult.result.value)",
    ],
    forbiddenProviderCall: "requireReallyMeCodecWasmProvider().pemDecode",
  },
  {
    name: "deterministic CBOR encode",
    protoOperationField:
      "CodecDeterministicCborEncodeRequest deterministic_cbor_encode = 5000;",
    protoRequestMessage: "message CodecDeterministicCborEncodeRequest",
    protoResultMessage: "message CodecDeterministicCborEncodeResult",
    protoVariant: "CodecOperation::DeterministicCborEncode(request) => {",
    rustPath: codecContractExecuteDocumentsPath,
    contractFunction: "fn process_deterministic_cbor_encode(",
    contractFunctionDeclaration:
      "fn process_deterministic_cbor_encode<P: buffa::ProtoBox<CodecDeterministicCborValue>>(",
    contractNeedles: [
      "validate_deterministic_value(proto_value, 0, &mut limits)?",
      "encode_deterministic_cbor_value(&value).map_err(deterministic_cbor_wire_error)?",
    ],
    tsPath: "packages/ts/src/processCbor.ts",
    tsExport: "export const deterministicCborEncode = (value: unknown): Uint8Array =>",
    tsNeedles: [
      "case: \"deterministicCborEncode\"",
      "processGeneratedOperationRequest(",
      'operationResult.result.case !== "deterministicCborEncode"',
      "result.encoded.fill(0)",
      "wipeProtoDeterministicCborValueBytes(protoValue)",
    ],
  },
  {
    name: "deterministic CBOR decode",
    protoOperationField:
      "CodecDeterministicCborDecodeRequest deterministic_cbor_decode = 5001;",
    protoRequestMessage: "message CodecDeterministicCborDecodeRequest",
    protoResultMessage: "message CodecDeterministicCborDecodeResult",
    protoVariant: "CodecOperation::DeterministicCborDecode(request) => {",
    rustPath: codecContractDecodeDocumentsPath,
    contractFunction: "fn process_deterministic_cbor_decode(",
    contractNeedles: [
      "decode_deterministic_cbor_value(encoded).map_err(deterministic_cbor_wire_error)?",
      "CodecDeterministicCborDecodeResult",
    ],
    tsPath: "packages/ts/src/processCbor.ts",
    tsExport:
      "export const deterministicCborDecode = (\n  bytes: Uint8Array,",
    tsNeedles: [
      "case: \"deterministicCborDecode\"",
      "processGeneratedOperationRequest(",
      'operationResult.result.case !== "deterministicCborDecode"',
      "requestBytes.fill(0)",
      "wipeProtoDeterministicCborValueBytes(result.value)",
    ],
  },
];

// The legacy envelope stays source-compatible. Every expanded operation must
// have a generated result at the same wire tag and exactly one concrete handler.
const expandedSchema = readText(codecProtoSchemaPath);
const operationFields = (name) => {
  const body = expandedSchema.match(new RegExp(`message ${name} \\{([\\s\\S]*?)\n\\}`, "u"))?.[1];
  if (body === undefined) fail(`missing generated envelope ${name}`);
  return [...body.matchAll(/^\s+(\w+) (\w+) = (\d+);/gmu)].map((match) => ({ type: match[1], name: match[2], tag: match[3] }));
};
const expandedRequests = operationFields("CodecApiOperationRequest");
const expandedResults = operationFields("CodecApiOperationResult");
assertSetEquals({
  label: "provider lifecycle request/result wire tags",
  actual: operationFields("CodecProviderOperationRequest").map((field) => field.tag).sort(),
  expected: operationFields("CodecProviderOperationResult").map((field) => field.tag).sort(),
});
assertContains(codecProtoSchemaPath, "CodecWasmReinstantiateRequest reinstantiate_wasm = 5;");
assertContains(codecProtoSchemaPath, "CodecEmptyResult reinstantiated_wasm = 5;");
const expandedHandlers = ["dispatch_api", "execute_scalars", "execute_general_cbor", "execute_der", "execute_tlv"]
  .map((name) => {
    const source = stripSourceComments(readText(`crates/codec/src/operation_contract/${name}.rs`));
    // Routing patterns delegate ownership; only the leaf arm implements an
    // operation. Empty-request leaves still count as their own implementation.
    return name === "dispatch_api" ? source.replace(/\bOperation::\w+\(_\)/gu, "") : source;
  }).join("\n");
const declaredHandlers = [...expandedHandlers.matchAll(/\bOperation::(\w+)\((?:request|_)\)(?=\s*(?:=>|\|))/gu)].map((match) => match[1]);
const declaredResults = [...expandedHandlers.matchAll(/\b(?:OperationResult|ApiResult)::(\w+)\(/gu)].map((match) => match[1]);
assertSetEquals({ label: "expanded operation result wire tags", actual: expandedResults.map((field) => `${field.name}:${field.tag}`).sort(), expected: expandedRequests.map((field) => `${field.name}:${field.tag}`).sort() });
const operationVariant = (name) => name.replace(/(^|_)([a-z])/gu, (_match, _separator, letter) => letter.toUpperCase());
for (const field of expandedRequests) {
  const variant = operationVariant(field.name);
  if (declaredHandlers.filter((name) => name === variant).length !== 1 || !declaredResults.includes(variant)) {
    fail(`expanded operation ${field.name} must have one semantic handler and its generated result`);
  }
}
const operationVectorsPath = "vectors/codec-operation-vectors.json";
const operationVectors = readJson(operationVectorsPath);
if (operationVectors.schemaVersion !== 1 || !Array.isArray(operationVectors.cases) ||
    operationVectors.cases.length !== expandedRequests.length) {
  fail("every expanded schema operation requires a conformance vector");
}
const expectedVectorOperations = expandedRequests.map((field) =>
  `${field.name.replace(/_([a-z])/gu, (_match, letter) => letter.toUpperCase())}:${field.tag}`,
).sort();
const actualVectorOperations = [];
for (const vector of operationVectors.cases) {
  if (typeof vector.operation !== "string" || !Number.isSafeInteger(vector.fieldNumber) ||
      typeof vector.requestJson !== "string" || typeof vector.requestHex !== "string" ||
      !/^(?:[0-9a-f]{2})+$/u.test(vector.requestHex) ||
      vector.checks === null || typeof vector.checks !== "object" || Array.isArray(vector.checks)) {
    fail("operation conformance vectors require typed names, wire bytes, JSON requests, and checks");
  }
  let request;
  try { request = JSON.parse(vector.requestJson); } catch { fail("invalid conformance request JSON"); }
  if (request === null || typeof request !== "object" || Array.isArray(request) ||
      Object.keys(request).length !== 1 || !Object.hasOwn(request, vector.operation)) {
    fail("operation conformance vector must select its declared operation");
  }
  actualVectorOperations.push(`${vector.operation}:${vector.fieldNumber}`);
}
assertSetEquals({ label: "operation conformance vectors", actual: actualVectorOperations.sort(), expected: expectedVectorOperations });
for (const path of [
  "crates/codec/tests/complete_contract_tests.rs",
  "packages/ts/test/complete-contract.test.mjs",
  "packages/swift/Tests/ReallyMeCodecTests/CompleteContractTests.swift",
  "packages/kotlin/src/test/kotlin/me/really/codec/CompleteContractTest.kt",
]) {
  assertContains(path, "codec-operation-vectors.json");
}
for (const field of operationFields("CodecOperationRequest")) {
  if (!expandedRequests.some((candidate) => candidate.name === field.name && candidate.tag === field.tag && candidate.type === field.type)) {
    fail(`expanded envelope changed legacy operation ${field.name}`);
  }
}
for (const field of operationFields("CodecOperationResult")) {
  if (!expandedResults.some((candidate) => candidate.name === field.name && candidate.tag === field.tag && candidate.type === field.type)) {
    fail(`expanded envelope changed legacy result ${field.name}`);
  }
}
for (const [path, needle] of [
  ["crates/codec/src/operation_contract/dispatch.rs", "decode_protobuf::<ApiRequest>(request_bytes)"],
  ["crates/codec/src/operation_contract/dispatch.rs", "decode_json::<ApiRequest>(request_json)"],
  ["crates/codec/src/operation_contract/dispatch_api.rs", "super::process_operation_request(request)?"],
  ["crates/cbor/src/general/error.rs", "CodecGeneralCborErrorReason as ErrorReason"],
  ["crates/codec/src/der/error.rs", "CodecDerErrorReason as DerErrorReason"],
  ["crates/codec/src/operation_contract/execute_scalar_native.rs", "CodecBytesRequestView"],
  ["crates/adapter/src/scalar_ops.rs", "pub use codec_runtime::operation_contract::scalar_ops::{"],
  ["crates/codec/src/operation_contract/execute_scalars.rs", "scalar_ops as scalar"],
  ["packages/ts/scripts/build-wasm.mjs", "The generated WASM initializer declaration layout changed."],
]) assertContains(path, needle);

const scalarDispatchSource = stripSourceStringsAndComments(readText("crates/codec/src/operation_contract/execute_scalars.rs"));
if (/\bcodec_\w+::[a-z]\w*\s*\(/u.test(scalarDispatchSource)) {
  fail("scalar dispatch must call the shared semantic implementation instead of primitive functions");
}
const legacyScalarSource = stripSourceStringsAndComments(readText("crates/adapter/src/scalar_ops.rs"));
if (/\b(?:fn|struct|enum)\b/u.test(legacyScalarSource)) {
  fail("the legacy scalar adapter must only re-export the shared generated-contract implementation");
}

// Framing stays separate from the schema-directed DER reader. Shared goldens
// cover complete headers, absent values, borrowed extents, and typed failures.
for (const path of [
  "crates/codec/tests/tlv_contract_tests.rs",
  "packages/ts/test/tlv-contract.test.mjs",
  "packages/swift/Tests/ReallyMeCodecTests/TlvContractTests.swift",
  "packages/kotlin/src/test/kotlin/me/really/codec/TlvContractTest.kt",
]) assertContains(path, "tlv-framing-vectors.json");
assertContains("crates/codec/src/operation_contract/execute_tlv.rs", "tlv::read_header");
assertContains("crates/codec/src/operation_contract/execute_tlv.rs", "tlv::frame_prefix");
assertContains("crates/codec/src/operation_contract/execute_tlv.rs", "tlv::frame_exact");
assertContains("crates/codec/src/operation_contract/execute_tlv.rs", "tlv::read_identifier");
assertContains("crates/codec/src/operation_contract/execute_tlv.rs", "tlv::read_length");
assertContains("crates/codec/src/tlv/error.rs", "CodecTlvErrorReason as ErrorReason");
assertContains("crates/proto/src/error.rs", "pub fn from_tlv_reason");
const tlvVectors = readJson("vectors/tlv-framing-vectors.json");
if (tlvVectors.schemaVersion !== 1 || !Array.isArray(tlvVectors.cases) || tlvVectors.cases.length === 0) {
  fail("TLV framing requires independent transport goldens");
}
const tlvVectorNames = new Set();
for (const vector of tlvVectors.cases) {
  const key = `${vector.name}:${vector.operation}`;
  if (typeof vector.name !== "string" || vector.name.length === 0 || tlvVectorNames.has(key) ||
      !["tlvReadHeader", "tlvFramePrefix", "tlvFrameExact", "tlvReadIdentifier", "tlvReadLength"].includes(vector.operation) ||
      typeof vector.requestJson !== "string" || typeof vector.requestHex !== "string" ||
      typeof vector.responseHex !== "string" ||
      !/^(?:[0-9a-f]{2})+$/u.test(vector.requestHex) || !/^(?:[0-9a-f]{2})+$/u.test(vector.responseHex)) {
    fail("TLV framing goldens require unique names, generated operations, and exact wire bytes");
  }
  tlvVectorNames.add(key);
}

const remainingStructuredSemanticOperations = [
  {
    moduleName: "multikey",
    path: "crates/codec/src/operation_contract/core/multikey.rs",
    functionNeedle: "pub fn parse_multikey(",
    fileNeedles: ["pub struct ParsedMultikey", "pub enum MultikeyOperationError"],
    requiredNeedles: [
      "parse_primitive_multikey(multikey).map_err(multikey_operation_error)?",
      "MultikeyOperationError",
    ],
  },
  {
    moduleName: "dag_cbor",
    path: "crates/codec/src/operation_contract/core/dag_cbor.rs",
    functionNeedle: "pub fn verify_dag_cbor_cid(",
    fileNeedles: [
      "pub struct DagCborCidVerification",
      "pub enum DagCborOperationError",
    ],
    requiredNeedles: [
      "if payload.len() > MAX_DAG_CBOR_INPUT_LEN",
      "verify_primitive_dag_cbor_cid(cid, payload)",
    ],
  },
  {
    moduleName: "pem",
    path: "crates/codec/src/operation_contract/core/pem.rs",
    functionNeedle: "pub fn decode_pem(",
    fileNeedles: ["pub struct DecodedPem", "der: Zeroizing<Vec<u8>>"],
    requiredNeedles: [
      "decode_primitive_pem(input, policy).map_err(pem_operation_error)?",
    ],
  },
];

for (const operation of remainingStructuredSemanticOperations) {
  assertContains(
    "crates/codec/src/operation_contract/core/mod.rs",
    `mod ${operation.moduleName};`,
  );
  assertNotContains("crates/codec/src/lib.rs", "pub mod contract_core;");
  assertContains(operation.path, "#[non_exhaustive]");
  for (const needle of operation.fileNeedles) {
    assertContains(operation.path, needle);
  }
  assertRustFunction({
    path: operation.path,
    functionNeedle: operation.functionNeedle,
    requiredNeedles: operation.requiredNeedles,
    forbiddenNeedles: [
      "codec_proto",
      "serde_json",
      "wasm_bindgen",
      "jni::",
      "unwrap(",
      "expect(",
      "panic!(",
    ],
  });
}

for (const ffiRegressionTest of [
  "retired_scalar_structured_json_ids_fail_closed",
]) {
  assertContains("crates/ffi/src/codec/tests.rs", ffiRegressionTest);
}

for (const ownershipNeedle of [
  "verification.into_parts()",
  "decoded.into_der()",
  "try_copy_result_bytes",
  "try_copy_result_string",
]) {
  assertContains(codecContractShapePath, ownershipNeedle);
}
assertContains(
  "crates/codec/src/operation_contract/tests/multikey_documents_pem.rs",
  "dag_cbor_generated_result_takes_semantic_string_ownership",
);
assertContains(
  "crates/codec/src/operation_contract/tests/multikey_documents_pem.rs",
  "pem_generated_result_takes_semantic_der_ownership",
);
assertNotContains(codecContractShapePath, "der: decoded.der().to_vec()");
assertRustFunction({
  path: codecContractShapePath,
  functionNeedle: "fn dag_cbor_verify_cid_result_proto(",
  requiredNeedles: ["verification.into_parts()", "expected_cid", "actual_cid"],
  forbiddenNeedles: ["to_owned()", "to_string()", "clone()"],
});
assertRustFunction({
  path: codecContractShapePath,
  functionNeedle: "fn pem_decode_result_proto(",
  requiredNeedles: [
    "try_copy_result_string(decoded.label().as_str())?",
    "decoded.into_der()",
  ],
  forbiddenNeedles: ["decoded.der().to_vec()", "decoded.der().to_owned()"],
});
assertRustFunction({
  path: codecContractShapePath,
  functionNeedle: "fn try_copy_result_bytes(",
  requiredNeedles: [
    "try_reserve_exact(value.len())",
    "map_err(|_| internal_wire_error())?",
    "extend_from_slice(value)",
  ],
  forbiddenNeedles: ["value.to_vec()", "value.to_owned()"],
});
assertRustFunction({
  path: codecContractShapePath,
  functionNeedle: "fn try_copy_result_string(",
  requiredNeedles: [
    "try_reserve_exact(value.len())",
    "map_err(|_| internal_wire_error())?",
    "push_str(value)",
  ],
  forbiddenNeedles: ["value.to_string()", "value.to_owned()"],
});

const structuredAdapterProductionSource = stripSourceComments(
  [
    rustProductionSource(codecContractExecuteMultiformatPath),
    rustProductionSource(codecContractExecuteDocumentsPath),
  ].join("\n"),
);
for (const adapterPrimitiveNeedle of [
  "codec_multikey::parse_multikey",
  "codec_cbor::verify_dag_cbor_cid",
  "codec_pem::decode_pem",
]) {
  if (structuredAdapterProductionSource.includes(adapterPrimitiveNeedle)) {
    fail(
      `operation contract execution fragments must not contain ${adapterPrimitiveNeedle}`,
    );
  }
}

const contractDispatchSource = stripSourceComments(
  rustProductionSource(codecContractDispatchPath),
);
for (const operation of generatedStructuredOperations) {
  const contractCallNeedle = operation.contractFunction.replace(/^fn /u, "");
  const contractFunctionNeedle =
    operation.contractFunctionDeclaration ?? operation.contractFunction;
  const operationRustPath = operation.rustPath;

  assertContains(codecProtoSchemaPath, operation.protoOperationField);
  assertContains(codecProtoSchemaPath, operation.protoRequestMessage);
  assertContains(codecProtoSchemaPath, operation.protoResultMessage);

  const variantBlock = blockFromNeedle({
    path: codecContractDispatchPath,
    source: contractDispatchSource,
    startNeedle: operation.protoVariant,
    nextNeedle: "\n        CodecOperation::",
    label: `${operation.name} operation variant`,
  });
  assertBlockContains({
    path: codecContractDispatchPath,
    block: variantBlock,
    needle: contractCallNeedle,
    label: `${operation.name} operation variant`,
  });


  assertRustFunction({
    path: operationRustPath,
    functionNeedle: contractFunctionNeedle,
    requiredNeedles: operation.contractNeedles,
    forbiddenNeedles: ["serde_json::json!", "json!"],
  });

  if (operation.ffiArm !== undefined) {
    assertRustCodecArm({
      path: codecFfiPath,
      arm: operation.ffiArm,
      requiredNeedles: operation.ffiNeedles,
      forbiddenNeedles: ["serde_json::json!", "json!"],
    });
  }

  assertTypescriptExport({
    path: operation.tsPath,
    exportNeedle: operation.tsExport,
    requiredNeedles: operation.tsNeedles,
    forbiddenNeedles: [
      operation.forbiddenProviderCall,
      "errorCodeForCodecError(",
      "CodecErrorReason.",
    ].filter((needle) => needle !== undefined),
  });
}

for (const sdkPath of [
  "packages/ts/src/multiformat.ts",
  "packages/ts/src/cbor.ts",
  "packages/ts/src/pem.ts",
]) {
  assertNotContains(sdkPath, "errorCodeForCodecError(");
  assertNotContains(sdkPath, "reason >= ");
  assertNotContains(sdkPath, "reason <=");
}
assertContains("packages/ts/src/classifyOperationError.ts", "const errorCodeForCodecErrorMessage =");
for (const reasonRange of [
  "reason < 100 || reason > 199",
  "reason < 200 || reason > 299",
  "reason < 300 || reason > 399",
  "reason < 400 || reason > 499",
]) {
  assertContains("packages/ts/src/classifyOperationError.ts", reasonRange);
}
for (const typedSdkResultReaderPath of [
  "packages/swift/Sources/ReallyMeCodec/StructuredResults.swift",
  "packages/swift/Sources/ReallyMeCodec/DeterministicCbor.swift",
  "packages/kotlin/src/main/kotlin/me/really/codec/StructuredResults.kt",
  "packages/kotlin/src/main/kotlin/me/really/codec/DeterministicCbor.kt",
]) {
  assertNotContains(typedSdkResultReaderPath, "inputErrorOrProviderFailure");
  assertNotContains(typedSdkResultReaderPath, "reason.rawValue");
  assertNotContains(typedSdkResultReaderPath, "reason.number");
  assertNotContains(typedSdkResultReaderPath, "errorCodeForCodecError(");
}
assertContains(
  "packages/swift/Sources/ReallyMeCodec/CallCodecWithRustCAbi.swift",
  "static func errorForCodecError(",
);
assertContains(
  "packages/swift/Sources/ReallyMeCodec/CallCodecWithRustCAbi.swift",
  "Self.isNonCanonicalReason(error.reason)",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/CallCodec.kt",
  "private fun exceptionForCodecError(codecError: CodecError)",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/CallCodec.kt",
  "reason.number in expectedRange",
);

for (const deletedOperationProtoApi of [
  "multicodecPrefixForNameProto",
  "multicodecPrefixForNameProtoResult",
  "multicodecLookupPrefixProto",
  "multicodecLookupPrefixProtoResult",
  "multicodecTableProto",
  "multicodecTableProtoResult",
  "multikeyParseProto",
  "multikeyParseProtoResult",
  "dagCborVerifyCidProto",
  "dagCborVerifyCidProtoResult",
  "decodePemProto",
  "decodePemProtoResult",
]) {
  for (const sdkSourcePath of [
    "packages/ts/src/index.ts",
    "packages/ts/src/multiformat.ts",
    "packages/ts/src/cbor.ts",
    "packages/ts/src/pem.ts",
    "packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift",
    "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt",
  ]) {
    assertNotContains(sdkSourcePath, deletedOperationProtoApi);
  }
}
for (const deletedProtoPayloadShim of [
  "protoPayloadOrThrow",
  "processGeneratedProtoRequest",
  "boundaryResourceLimitResult",
  "decodeResultEnvelope",
  "ReallyMeCodecProtoResult",
  "ReallyMeCodecProtoStatus",
  "processProtoPayload",
  "processProtoResultNative",
  "decodeProtoResultEnvelope",
  "errorForCodecErrorPayload",
  "exceptionForCodecErrorPayload",
]) {
  for (const sdkSourcePath of [
    "packages/ts/src/operationContract.ts",
    "packages/ts/src/readOutput.ts",
    "packages/ts/src/index.ts",
    "packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift",
    "packages/swift/Sources/ReallyMeCodec/CallCodecWithRustCAbi.swift",
    "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt",
    "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodecNative.kt",
    "crates/ffi/src/kotlin_codec.rs",
  ]) {
    assertNotContains(sdkSourcePath, deletedProtoPayloadShim);
  }
}

// Semantic resource ceilings are generated from one reviewed table for Rust
// and every SDK. A source edit without regenerating all lanes fails readiness.
requireTracked("scripts/codec_limits.json");
requireTracked("scripts/generate_codec_limits.mjs");
try {
  execFileSync(process.execPath, ["scripts/generate_codec_limits.mjs", "--check"], {
    stdio: "pipe",
  });
} catch {
  fail("codec resource limits differ from scripts/codec_limits.json");
}
assertContains(
  "packages/ts/src/deterministicCborBoundary.ts",
  "MAX_DETERMINISTIC_CBOR_PROTO_RECURSION_DEPTH = 198",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/CodecWireLimits.kt",
  "MAX_DETERMINISTIC_CBOR_PROTO_MESSAGE_DEPTH: Int = 197",
);
assertContains(
  "packages/swift/Sources/ReallyMeCodec/ValidateDeterministicCbor.swift",
  "maxDeterministicCborProtoMessageDepth = 198",
);
for (const swiftTransportNeedle of [
  "maxCodecProtoStructuralBytesPerDeterministicCborNode = 128",
  "maxCodecProtoFixedDeterministicCborOperationBytes = 4_096",
  "maxDeterministicCborProtoMessageBytes =",
  "serialized.count <= maxDeterministicCborProtoMessageBytes",
]) {
  assertContains(
    swiftTransportNeedle.startsWith("serialized.count")
      ? "packages/swift/Sources/ReallyMeCodec/ProcessCbor.swift"
      : "packages/swift/Sources/ReallyMeCodec/ValidateDeterministicCbor.swift",
    swiftTransportNeedle,
  );
}
for (const path of ["ValidateDeterministicCbor", "ValidateProviderCbor"]) {
  assertContains(`packages/swift/Sources/ReallyMeCodec/${path}.swift`, "boundedDeterministicCborUtf8Length(");
  assertNotContains(`packages/swift/Sources/ReallyMeCodec/${path}.swift`, ".utf8.count");
}
assertContains(
  "packages/swift/Sources/ReallyMeCodec/ValidateProviderCbor.swift",
  "rejectDuplicateProviderDeterministicCborMapKeys(",
);
assertContains(
  "packages/swift/Sources/ReallyMeCodec/ValidateProviderCbor.swift",
  "compareDeterministicCborUtf8(",
);
assertContains(
  "packages/swift/Sources/ReallyMeCodec/DeterministicCbor.swift",
  "public enum ReallyMeDeterministicCbor",
);
assertContains(
  "packages/swift/Sources/ReallyMeCodec/DeterministicCbor.swift",
  "public enum ReallyMeDagCbor",
);
assertContains(
  "packages/swift/Sources/ReallyMeCodec/DataErgonomics.swift",
  "func base64urlEncode(_ data: Data) throws(ReallyMeCodecError) -> String",
);
assertContains(
  "packages/swift/Sources/ReallyMeCodec/DataErgonomics.swift",
  "func deterministicCborEncodeData(",
);
assertContains(
  "packages/swift/Sources/ReallyMeCodec/DeterministicCbor.swift",
  "public static func bytes(_ value: Data)",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ValidateProviderCbor.kt",
  "rejectDuplicateProviderDeterministicCborMapKeys(",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/DeterministicCbor.kt",
  "public object ReallyMeDeterministicCbor",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/DeterministicCbor.kt",
  "public object ReallyMeDagCbor",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/DeterministicCbor.kt",
  "entries: List<Pair<String, ReallyMeDeterministicCborValue>>",
);
assertContains("packages/ts/src/constructCbor.ts", "export const ReallyMeDeterministicCbor =");
assertContains("packages/ts/src/constructCbor.ts", "export const ReallyMeDagCbor =");
assertContains("packages/ts/src/constructCbor.ts", "bytes(value: Uint8Array)");
assertContains("packages/ts/src/constructCbor.ts", "negative(value: number | bigint)");
assertContains("packages/ts/src/constructCbor.ts", "text(value: string)");
for (const sdkVectorTest of [
  "packages/swift/Tests/ReallyMeCodecTests/DeterministicCborVectorTests.swift",
  "packages/kotlin/src/test/kotlin/me/really/codec/DeterministicCborVectorTest.kt",
]) {
  for (const vectorSection of [
    "positive",
    "negative",
    "equivalentInputOrders",
    "resourceRejections",
    "interoperability",
    "idkit-ios-synthetic-passport-claims-v1",
  ]) {
    assertContains(sdkVectorTest, vectorSection);
  }
}
assertContains(
  "packages/swift/Tests/ReallyMeCodecTests/DeterministicCborVectorTests.swift",
  "maximumCborBytes",
);
assertContains(
  "packages/kotlin/src/test/kotlin/me/really/codec/DeterministicCborVectorTest.kt",
  "MAXIMUM_CBOR_BYTES",
);
for (const semanticPath of [
  "crates/cbor/src/encode_deterministic_cbor.rs",
  "crates/cbor/src/decode_deterministic_cbor.rs",
]) {
  assertContains(semanticPath, "try_vec_with_capacity");
  assertNotContains(semanticPath, "Vec::with_capacity(");
}
assertContains(
  "crates/cbor/src/deterministic/error.rs",
  "AllocationFailure",
);
for (const [conversionPath, conversionNeedle] of [
  [codecContractCopyLimitsPath, "try_deterministic_vec"],
  [codecContractCopyLimitsPath, "try_copy_deterministic_bytes"],
  [codecContractCopyLimitsPath, "try_copy_deterministic_text"],
  ["crates/codec/src/operation_contract/map_errors.rs", "DeterministicCborError::AllocationFailure"],
  [
    "crates/codec/src/operation_contract/tests/deterministic_cbor.rs",
    "deterministic_cbor_semantic_maximum_is_reachable_through_protobuf_lanes",
  ],
]) {
  assertContains(conversionPath, conversionNeedle);
}
assertContains("crates/codec/src/operation_contract/run_contract.rs", 'include!("copy_limits.rs")');

assertContains("buf.gen.yaml", "out: crates/proto/src/generated/buffa");
assertContains("buf.gen.yaml", "out: packages/ts/src/proto/generated");
assertContains("buf.gen.yaml", "buf.build/bufbuild/es:v2.17.0");
assertContains("buf.gen.yaml", "buf.build/apple/swift:v1.39.0");
assertContains("buf.gen.yaml", "buf.build/protocolbuffers/java:v36.2");
assertContains("buf.gen.yaml", "buf.build/protocolbuffers/kotlin:v36.2");
assertContains("crates/proto/src/generated/buffa/mod.rs", "pub mod codec");
assertContains("crates/proto/src/error.rs", "pub struct CodecWireError");
assertContains("crates/proto/src/error.rs", "pub fn try_new");
assertContains("crates/proto/src/limits.rs", "MAX_CODEC_PROTO_MESSAGE_BYTES");
assertContains("crates/proto/src/limits.rs", "MAX_CODEC_PROTO_ERROR_ENVELOPE_BYTES");
for (const transportDerivationNeedle of [
  "CODEC_PROTO_DETERMINISTIC_CBOR_TEXT_BYTES",
  "CODEC_PROTO_DETERMINISTIC_CBOR_BYTE_STRING_BYTES",
  "CODEC_PROTO_DETERMINISTIC_CBOR_NODES",
  "CODEC_PROTO_MAX_STRUCTURAL_BYTES_PER_CBOR_NODE",
  "max_codec_proto_cbor_message_bytes_const()",
  "max_codec_proto_cbor_json_bytes_const()",
  "max_codec_proto_tlv_json_bytes_const()",
  "MAX_CODEC_TLV_INPUT_BYTES.checked_add(CODEC_PROTO_MAX_FIXED_OPERATION_BYTES)",
]) {
  assertContains("crates/proto/src/limits.rs", transportDerivationNeedle);
}
for (const typescriptTransportNeedle of [
  "MAX_CODEC_PROTO_STRUCTURAL_BYTES_PER_NODE",
  "MAX_CODEC_PROTO_FIXED_OPERATION_BYTES",
  "MAX_CODEC_TLV_INPUT_BYTES",
  "MAX_CODEC_TLV_JSON_PAYLOAD_BYTES",
  "MAX_CODEC_PROTO_MESSAGE_BYTES =",
  "MAX_CODEC_PROTO_JSON_BYTES =",
]) {
  assertContains("packages/ts/src/boundary.ts", typescriptTransportNeedle);
}
for (const kotlinTransportNeedle of [
  "MAX_CODEC_PROTO_MESSAGE_BYTES: Int =",
]) {
  assertContains(
    "packages/kotlin/src/main/kotlin/me/really/codec/CodecWireLimits.kt",
    kotlinTransportNeedle,
  );
}
assertContains(
  "crates/ffi/src/kotlin_codec.rs",
  "if request.len(env)? > max_request_len",
);
assertContains(
  "packages/ts/src/readOutput.ts",
  "value.buffer === input.buffer",
);
assertContains(
  "packages/ts/test/reallyme-codec.test.mjs",
  "protobuf envelope validation rejects shared and invalid provider storage",
);
assertContains(
  "crates/proto/src/wire.rs",
  "pub fn encode_protobuf<M: Message>(message: &M) -> CodecWireResult<Zeroizing<Vec<u8>>>",
);
assertContains(
  "crates/proto/src/wire.rs",
  ".try_encode_to_vec()",
);
assertContains("crates/proto/tests/generated_tests/error_wire.rs", "bounded_protobuf_decode_rejects_oversized_messages");
assertContains("crates/proto/tests/generated_tests/error_wire.rs", "json_decode_rejects_inputs_that_expand_past_binary_cap");
assertContains(".github/workflows/protobuf-ci.yml", "scripts/run_pinned_release_readiness.mjs");
assertContains(".github/workflows/protobuf-ci.yml", "cargo install reallyme-protoc-gen-buffa-packaging");
assertContains("scripts/generate_codec_proto.mjs", '["buf", ["lint"]]');
assertContains(".github/workflows/protobuf-ci.yml", "buf breaking --against \".git#tag=${release_tag}\"");
assertContains(".github/workflows/protobuf-ci.yml", "buf breaking --against '.git#branch=origin/main'");
assertContains(
  "crates/proto/proto/reallyme/codec/v1/codec.proto",
  "repeated CodecDeterministicCborMapEntry entries = 1;",
);
assertContains(
  ".github/workflows/protobuf-ci.yml",
  "node scripts/run_pinned_release_readiness.mjs --generated-freshness",
);
assertWorkflowUsesStepPinnedTo(
  ".github/workflows/protobuf-ci.yml",
  "Install buf",
  "bufbuild/buf-action",
);
assertContains(
  ".github/workflows/protobuf-ci.yml",
  "checksum: f71b009ebd8530b12fd85b060c6c245e15d6e5110ccc8da48d8b6fa71e59330b",
);
assertContains(".github/workflows/protobuf-ci.yml", "github_token: ${{ github.token }}");
assertContains(".github/workflows/protobuf-ci.yml", "setup_only: true");
if (readText("buf.gen.yaml").includes("reallyme.crypto.v1")) {
  fail("buf.gen.yaml still generates crypto protos");
}

const tsCodecPackage = readJson("packages/ts/package.json");
if (tsCodecPackage.name !== "@reallyme/codec") {
  fail("packages/ts/package.json must publish @reallyme/codec");
}
if (tsCodecPackage.version !== codecPackageVersion) {
  fail(`packages/ts/package.json is not versioned ${codecPackageVersion}`);
}
if (tsCodecPackage.private === true) {
  fail("packages/ts/package.json is private and cannot be published to npm");
}
assertContains("packages/ts/README.md", "@reallyme/codec/wasm/reallyme_codec_wasm.js");
assertContains("packages/ts/package.json", '"@bufbuild/protobuf": "2.17.0"');
assertContains("packages/ts/package.json", '"fast-check": "4.10.2"');
assertContains("packages/ts/package.json", '"typescript": "7.0.2"');
assertContains("packages/ts/package.json", '"NOTICE"');
readText("packages/ts/NOTICE");
assertTypescriptProtoFacadeCompleteness({
  facadePath: "packages/ts/src/proto.ts",
  generatedPath: "packages/ts/src/proto/generated/reallyme/codec/v1/codec_pb.ts",
});

const kotlinCodecBuild = readText("packages/kotlin/build.gradle.kts");
if (!kotlinCodecBuild.includes(`version = "${codecPackageVersion}"`)) {
  fail(`packages/kotlin/build.gradle.kts is not versioned ${codecPackageVersion}`);
}
assertContains("packages/kotlin/build.gradle.kts", 'kotlin("jvm") version "2.4.21"');
assertContains("packages/kotlin/build.gradle.kts", 'artifactId = "codec"');
assertContains("packages/kotlin/build.gradle.kts", "Java, Kotlin, JVM, and Android");
assertContains("packages/kotlin/build.gradle.kts", "com.google.protobuf:protobuf-javalite:4.36.2");
assertContains("packages/kotlin/build.gradle.kts", "com.google.protobuf:protobuf-kotlin-lite:4.36.2");
assertContains("packages/kotlin/build.gradle.kts", 'val bouncyCastleSecurityVersion = "1.87"');
assertContains("packages/kotlin/build.gradle.kts", 'if (name == kotlinBouncyCastleConfiguration)');
for (const artifact of ["bcpg", "bcpkix", "bcprov", "bcutil"]) {
  assertContains(
    "packages/kotlin/gradle.lockfile",
    `org.bouncycastle:${artifact}-jdk18on:1.87=kotlinBouncyCastleConfiguration`,
  );
}
assertContains("packages/kotlin/build.gradle.kts", "https://github.com/reallyme/codec");
assertContains("packages/kotlin/build.gradle.kts", "reallyme.codec.nativeResourcesDir");
assertContains("packages/kotlin/build.gradle.kts", "reallyme.codec.requireFullNativeResources");
assertContains("packages/kotlin/build.gradle.kts", "verifyBundledNativeResources");
assertContains("packages/kotlin/build.gradle.kts", "verifyHostBundledNativeResource");
assertContains("packages/kotlin/build.gradle.kts", "verifyJarContainsNativeResources");
assertContains("packages/kotlin/build.gradle.kts", "ZipFile(jarTask.get().archiveFile.get().asFile)");
assertContains("packages/kotlin/build.gradle.kts", "expectedNativeDigestMetadata");
assertContains("packages/kotlin/build.gradle.kts", "ReallyMe codec native digest does not match");
assertContains("packages/kotlin/build.gradle.kts", "JVM JAR native digest does not match");
assertContains("packages/kotlin/build.gradle.kts", "stageHostNativeResource");
assertContains("packages/kotlin/build.gradle.kts", "native-manifest.json");
assertContains("packages/kotlin/build.gradle.kts", "PublishToMavenRepository");
assertContains("packages/kotlin/build.gradle.kts", "dependencyLocking {");
assertContains("packages/kotlin/build.gradle.kts", "lockAllConfigurations()");
assertContains(
  "packages/kotlin/gradle/wrapper/gradle-wrapper.properties",
  "distributionSha256Sum=d8a18a7a841dd5df825d35a685fad6f2d7f12ac0c280e57b67c27b4a520ba228",
);
assertContains(
  "packages/kotlin/gradle/wrapper/gradle-wrapper.properties",
  "distributionUrl=https\\://services.gradle.org/distributions/gradle-9.9.1-bin.zip",
);
assertContains(
  "packages/kotlin/gradle.properties",
  "org.gradle.dependency.verification=strict",
);
assertContains("packages/kotlin/gradle.lockfile", "com.google.protobuf:protobuf-javalite");
assertContains(
  "packages/kotlin/gradle/verification-metadata.xml",
  "<verify-metadata>true</verify-metadata>",
);
assertContains(
  "packages/kotlin/gradle/verification-metadata.xml",
  "<verify-signatures>false</verify-signatures>",
);
assertContains("docs/dependency-updates.md", "verifying every trusted full fingerprint");
assertContains("scripts/write_native_manifest.mjs", "sha256");
assertContains("scripts/write_native_manifest.mjs", 'writeFileSync(`${path}.sha256`');
assertContains("scripts/write_native_manifest.mjs", "GITHUB_SHA does not match the checked-out source SHA");
assertContains("packages/kotlin-android/build.gradle.kts", "fun checkedOutCommitSha()");
assertContains("packages/kotlin-android/build.gradle.kts", "GITHUB_SHA does not match the checked-out source SHA");
const workflowFiles = listFiles(".github/workflows");
for (const path of [
  ".github/workflows/code-checks.yml",
  ".github/workflows/codeql.yml",
  ".github/workflows/dependency-security.yml",
  ".github/workflows/fuzz.yml",
  ".github/workflows/protobuf-ci.yml",
]) {
  // PR updates supersede older PR checks; push checks remain distinct evidence
  // for the exact source SHA used by release attestation.
  assertContains(
    path,
    "group: ${{ github.workflow }}-${{ github.event_name }}-${{ github.event.pull_request.number || github.sha }}",
  );
  assertContains(path, "cancel-in-progress: ${{ github.event_name == 'pull_request' }}");
}
for (const removedWorkflow of [
  ".github/workflows/package-release.yml",
  ".github/workflows/release-preflight.yml",
]) {
  if (workflowFiles.includes(removedWorkflow)) {
    fail(`${removedWorkflow} must not be restored; package release lanes are intentionally separate`);
  }
}
const packagePreflightWorkflows = Object.freeze([
  ".github/workflows/crates-package-preflight.yml",
  ".github/workflows/swift-package-preflight.yml",
  ".github/workflows/kotlin-android-package-preflight.yml",
  ".github/workflows/npm-package-preflight.yml",
]);
const packageReleaseWorkflows = Object.freeze([
  ".github/workflows/crates-release.yml",
  ".github/workflows/swift-package-release.yml",
  ".github/workflows/kotlin-android-package-release.yml",
  ".github/workflows/npm-package-release.yml",
]);
for (const workflowPath of packagePreflightWorkflows) {
  assertContains(workflowPath, "Resolve release SHA");
  assertContains(workflowPath, 'default: ""');
  assertContains(workflowPath, "default: 0.4.0");
}
for (const workflowPath of packageReleaseWorkflows) {
  assertContains(workflowPath, "Verify reviewed release SHA");
  assertContains(workflowPath, "Resolve current release SHA");
  assertNotContains(workflowPath, "RELEASE_SHA_INPUT");
  assertNotContains(workflowPath, "inputs.publish");
}
assertContains(".github/workflows/kotlin-android-package-preflight.yml", "Write native checksum manifest");
assertContains(".github/workflows/kotlin-android-package-preflight.yml", "Test host native loader");
assertContains("scripts/maven_central_bundle_local.sh", "verify_linux_glibc_floor.sh");
assertWorkflowPermissionsPolicy({
  path: ".github/workflows/swift-package-release.yml",
  workflow: { contents: "read" },
  jobs: {
    "verify-release-sha": { actions: "read", contents: "read" },
    "swift-verify": { actions: "read", contents: "read" },
    "swift-release": { actions: "read", contents: "write" },
  },
});
assertWorkflowPermissionsPolicy({
  path: ".github/workflows/kotlin-android-package-release.yml",
  workflow: { contents: "read" },
  jobs: {
    "verify-release-sha": { actions: "read", contents: "read" },
    "maven-package": { actions: "read", contents: "read" },
    "android-aar": { actions: "read", contents: "read" },
  },
});
assertWorkflowPermissionsPolicy({
  path: ".github/workflows/npm-package-release.yml",
  workflow: { contents: "read" },
  jobs: {
    "verify-release-sha": { actions: "read", contents: "read" },
    "npm-package": { actions: "read", contents: "read" },
    "npm-publish": { actions: "read", contents: "read", "id-token": "write" },
  },
});
assertWorkflowPermissionsPolicy({
  path: ".github/workflows/crates-release.yml",
  workflow: { contents: "read" },
  jobs: {
    "verify-release-sha": { actions: "read", contents: "read" },
    "dry-run": { contents: "read" },
    publish: { actions: "read", contents: "read" },
  },
});
const readOnlyWorkflowPaths = [
  ".github/workflows/android-runtime-gate.yml",
  ".github/workflows/code-checks.yml",
  ".github/workflows/crates-package-preflight.yml",
  ".github/workflows/fuzz.yml",
  ".github/workflows/kotlin-android-package-preflight.yml",
  ".github/workflows/npm-package-preflight.yml",
  ".github/workflows/protobuf-ci.yml",
  ".github/workflows/swift-package-preflight.yml",
];
for (const path of readOnlyWorkflowPaths) {
  assertWorkflowPermissionsPolicy({ path, workflow: { contents: "read" } });
}
assertWorkflowPermissionsPolicy({
  path: ".github/workflows/dependency-security.yml",
  workflow: { actions: "read", contents: "read", "security-events": "write" },
});
assertWorkflowPermissionsPolicy({
  path: ".github/workflows/codeql.yml",
  workflow: { contents: "read" },
  jobs: {
    analyze: { actions: "read", contents: "read", "security-events": "write" },
  },
});
assertSetEquals({
  label: "workflow permissions policy coverage",
  actual: [
    ...readOnlyWorkflowPaths,
    ".github/workflows/dependency-security.yml",
    ".github/workflows/codeql.yml",
    ...packageReleaseWorkflows,
  ],
  expected: workflowFiles,
});

assertContains(".github/workflows/swift-package-release.yml", "environment: github-release");
for (const path of workflowFiles) {
  if (packageReleaseWorkflows.includes(path) ||
      path === ".github/workflows/kotlin-android-package-preflight.yml" ||
      path === ".github/workflows/swift-package-preflight.yml") {
    continue;
  }
  const source = readText(path);
  const steps = source.split(/^      - name: /mu).slice(1);
  for (const step of steps) {
    if (/^        uses: actions\/checkout@/mu.test(step) &&
        !/^          persist-credentials: false$/mu.test(step)) {
      fail(`${path} checkout must not retain repository credentials`);
    }
  }
}
for (const path of [
  ".github/workflows/crates-package-preflight.yml",
  ".github/workflows/npm-package-preflight.yml",
]) {
  assertContains(path, "Verify release version across package manifests");
  assertContains(path, "node scripts/verify_release_version.mjs");
}
for (const path of packageReleaseWorkflows) {
  assertContains(path, "node scripts/verify_release_attestation.mjs");
}
assertContains(".github/workflows/swift-package-release.yml", "gh release create");
assertContains(".github/workflows/swift-package-release.yml", "git push origin");
assertContains(".github/workflows/swift-package-release.yml", "Bind release manifest to verified Swift artifact");
assertContains(".github/workflows/npm-package-release.yml", "npm publish");
assertContains(".github/workflows/npm-package-release.yml", "npm pack --ignore-scripts");
assertContains(".github/workflows/npm-package-release.yml", "--provenance --access public --ignore-scripts");
assertContains(".github/workflows/npm-package-release.yml", "EXPECTED_TARBALL_SHA256");
assertContains("Cargo.toml", 'wasm-bindgen = "=0.2.130"');
assertContains(".github/workflows/protobuf-ci.yml", "wasm-bindgen-cli@0.2.130");
assertContains(".github/workflows/npm-package-preflight.yml", "wasm-bindgen-cli@0.2.130");
assertContains(".github/workflows/npm-package-release.yml", "wasm-bindgen-cli@0.2.130");
assertContains(".github/workflows/npm-package-release.yml", "NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}");
assertContains(".github/workflows/crates-release.yml", "secrets.CARGO_REGISTRY_TOKEN");
assertContains(".github/workflows/crates-package-preflight.yml", "verify_release_availability.mjs");
assertContains("scripts/verify_release_attestation.mjs", '"crates-package-preflight.yml"');
assertContains("scripts/verify_release_version.mjs", "workspace-path-requirement-mismatch");
assertContains("crates/ffi/src/codec.rs", "rm_codec_package_version_major");
assertContains("crates/ffi/src/codec.rs", "rm_codec_package_version_minor");
assertContains("crates/ffi/src/codec.rs", "rm_codec_package_version_patch");
assertContains("packages/swift/Sources/ReallyMeCodec/CallCodecWithRustCAbi.swift", "requireCompatiblePackageVersion");
assertContains(".github/workflows/kotlin-android-package-release.yml", "if: steps.maven_remote.outputs.configured == 'true'");
assertContains(".github/workflows/kotlin-android-package-release.yml", "configured=false");
assertContains("scripts/publish_crates_in_order.mjs", "published-crate-checksum-mismatch");
assertContains("scripts/maven_central_bundle_local.sh", "release checkout must have a clean working tree");
assertContains("scripts/maven_central_bundle_local.sh", "validate_successful_workflow_run");
assertContains("scripts/maven_central_bundle_local.sh", "android-runtime-gate.yml");
assertContains(".github/workflows/crates-release.yml", "needs: [verify-release-sha, dry-run]");
for (const workflowPath of [".github/workflows/fuzz.yml", ...packagePreflightWorkflows, ...packageReleaseWorkflows]) {
  assertNotContains(workflowPath, "actions/upload-artifact@330a01c490aca151604b8cf639adc76d48f6c5d4");
  assertNotContains(workflowPath, "actions/download-artifact@634f93cb2916e3fdff6788551b99b062d0335ce0");
}
assertContains(".github/workflows/dependency-security.yml", "google/osv-scanner-action/");
assertContains(".github/workflows/dependency-security.yml", "python3 scripts/gradle_runtime_lockfiles.py --check");
assertContains(".github/workflows/dependency-security.yml", "packages/kotlin/runtime/gradle.lockfile");
assertContains(".github/workflows/dependency-security.yml", "packages/kotlin-android/runtime/gradle.lockfile");
assertContains(".github/workflows/dependency-security.yml", "name: Gate published runtime dependencies");
assertContains(".github/workflows/dependency-security.yml", "name: Report Gradle build tooling advisories");
assertContains(".github/workflows/dependency-security.yml", "fail-on-vuln: true");
assertContains(".github/workflows/dependency-security.yml", "fail-on-vuln: false");
assertNotContains(".github/workflows/dependency-security.yml", "python3 -m unittest");
assertContains(".github/workflows/codeql.yml", "queries: security-and-quality");
assertContains(".github/workflows/codeql.yml", "- language: java-kotlin");
assertContains(".github/workflows/codeql.yml", "build-mode: manual");
assertContains(".github/workflows/codeql.yml", "compileKotlin compileJava");
assertContains(".github/workflows/codeql.yml", "compileReleaseKotlin compileReleaseJavaWithJavac");
assertNotContains(".github/workflows/codeql.yml", "- language: swift");
assertNotContains(".github/workflows/codeql.yml", "matrix.language == 'swift'");
assertContains(".github/workflows/protobuf-ci.yml", "runs-on: macos-26");
assertContains(".github/workflows/protobuf-ci.yml", 'test "$(uname -m)" = arm64');
assertContains("scripts/release-readiness/sdk-source-policy.mjs", '"test", "--arch", "arm64"');
assertContains(".github/workflows/protobuf-ci.yml", 'REALLYME_CODEC_SWIFTPM_RUNTIME_FFI: "1"');
assertContains(".github/workflows/code-checks.yml", "npm audit --prefix packages/ts --omit=dev --audit-level=high");
assertContains(".github/dependabot.yml", "package-ecosystem: cargo");
assertContains(".github/dependabot.yml", "package-ecosystem: gradle");
assertContains("scripts/verify_release_attestation.mjs", "DEPENDENCY_SECURITY_WORKFLOW");
assertMinOccurrences(".github/workflows/fuzz.yml", "toolchain: nightly-2026-07-01", 2);
assertNotContains(".github/workflows/fuzz.yml", "cargo +nightly fuzz");
assertContains(".github/workflows/fuzz.yml", "- operation_contract");
assertContains("fuzz/Cargo.toml", 'name = "operation_contract"');
assertContains("fuzz/README.md", "`operation_contract`");
assertContains("fuzz/tests/targets/operation_contract.rs", "process_operation_response(data)");
assertContains("fuzz/tests/targets/operation_contract.rs", "process_operation_response_json(data)");
assertContains(".github/workflows/fuzz.yml", "- deterministic_cbor");
assertContains("fuzz/Cargo.toml", 'name = "deterministic_cbor"');
assertContains("fuzz/README.md", "`deterministic_cbor`");
assertContains(
  "crates/cbor/tests/deterministic_codec_tests.rs",
  "fn bounded_arbitrary_trees_preserve_semantic_properties()",
);
assertContains(
  "crates/cbor/tests/deterministic_codec_tests.rs",
  "fn simultaneous_operations_are_deterministic_and_cleanup_is_independent()",
);
assertContains(
  "fuzz/tests/targets/deterministic_cbor.rs",
  "decode_deterministic_cbor(data)",
);
assertContains(
  "fuzz/tests/targets/deterministic_cbor.rs",
  "encode_deterministic_cbor(&value)",
);
assertContains(
  "fuzz/tests/targets/deterministic_cbor.rs",
  "decode_deterministic_cbor(&encoded)",
);
assertContains(
  "fuzz/tests/targets/deterministic_cbor.rs",
  "assert_eq!(encoded_again, encoded)",
);
assertContains("fuzz/Cargo.toml", 'name = "jcs_text"');
assertContains("fuzz/tests/targets/jcs_text.rs", "canonicalize_json_text(json)");
assertContains("fuzz/README.md", "`jcs_text`");
assertContains(".github/workflows/fuzz.yml", "- jcs_text");
assertContains("scripts/test_native_sanitizers.sh", "nightly-2026-07-01");
assertContains("scripts/test_native_sanitizers.sh", "-Zsanitizer=address");
assertContains("scripts/test_native_sanitizers.sh", "-Zub-checks=yes -Zextra-const-ub-checks=yes");
assertContains(".github/workflows/code-checks.yml", "Test native sanitizer lanes");
assertContains(".github/workflows/kotlin-android-package-release.yml", "Test and package Maven artifact");
assertContains(".github/workflows/kotlin-android-package-release.yml", "Build Android AAR");
assertContains(".github/workflows/kotlin-android-package-preflight.yml", "requireFullNativeResources=true");
assertContains("packages/kotlin/settings.gradle.kts", 'rootProject.name = "reallyme-codec"');
assertContains("packages/kotlin/README.md", "me.really:codec:0.4.0");
assertContains("packages/kotlin/README.md", "ships Rust JNI libraries as platform resources");
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodecRustNativeProvider.kt",
  "/me/really/codec/native",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodecRustNativeProvider.kt",
  'ANDROID_LIBRARY_NAME: String = "reallyme_codec_ffi"',
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodecRustNativeProvider.kt",
  "System.loadLibrary(ANDROID_LIBRARY_NAME)",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodecRustNativeProvider.kt",
  "createPrivateExtractionDirectory",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ExtractNativeLibrary.kt",
  "MessageDigest.isEqual",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodecRustNativeProvider.kt",
  "isSecurePosixTempMode",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodecRustNativeProvider.kt",
  "isTrustedPosixTempOwner",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ExtractNativeLibrary.kt",
  "AclFileAttributeView",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ExtractNativeLibrary.kt",
  "restrictAclToOwner",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ExtractNativeLibrary.kt",
  "makeExtractedLibraryReadOnly",
);
assertNotContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodecRustNativeProvider.kt",
  "File.createTempFile",
);
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "@JvmStatic");
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/WithCodecBytes.kt",
  "UnsafeByteOperations.unsafeWrap(bytes)",
);
assertNotContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt",
  "ByteString.copyFrom(bytes)",
);
assertContains("packages/kotlin/build.gradle.kts", "fun nonBlank(value: String?): String?");
assertContains("packages/kotlin/build.gradle.kts", "verifyRemoteMavenPublishingConfigured");
assertContains("packages/kotlin/build.gradle.kts", "remote Maven publishing is not configured");
assertContains("packages/kotlin/build.gradle.kts", 'parsed.scheme != "https"');
assertNotContains("packages/kotlin/build.gradle.kts", "reallyme.maven.requireRemote");
assertNotContains(".github/workflows/kotlin-android-package-release.yml", "-Preallyme.maven.requireRemote=true");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "public fun tryParseCid(cid: String): String?");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "public fun dagCborCodecCode(): Int");
assertContains("crates/ffi/src/codec.rs", "MAX_CODEC_FFI_OUTPUT_BYTES");
assertContains("crates/ffi/src/kotlin_codec.rs", "probed_output_capacity");
assertContains("crates/ffi/src/kotlin_codec.rs", "produced_len != output.len()");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/WithCodecBytes.kt", "CodingErrorAction.REPORT");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/WithCodecBytes.kt", "bytes.fill(0)");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/WithCodecBytes.kt", "Character.isSurrogate(character)");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/CallCodec.kt", "CODEC_ERROR_REASON_CANONICAL_INTERNAL");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "withTextBytes");
assertContains(
  "packages/kotlin/src/test/kotlin/me/really/codec/NativeProviderSafetyTest.kt",
  "nativeDigestMetadataAndTempPermissionsFailClosed",
);
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/NativeProviderSafetyTest.kt", 'ReallyMeCodec.canonicalizeJson("\\uD800")');
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", 'assertNull(codec.tryParseCid("not-a-cid"))');
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", "assertEquals(0x71, codec.dagCborCodecCode())");
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", 'codec.base58btcDecode("")');
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", "codec.base58btcEncode(oversizedBase58Input)");
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", 'codec.multicodecPrefixForName("not-a-codec")');
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", "codec.multicodecLookupPrefix(byteArrayOf(0, 0, 7))");
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", 'codec.dagCborVerifyCid("", encoded)');
assertContains("crates/multibase/src/base58btc.rs", "bytes.len() > MAX_BASE58BTC_DECODED_LEN");
assertContains("crates/multibase/tests/base58btc_tests.rs", "rejects_inputs_above_encode_cap_before_base58_conversion");
assertContains(
  "packages/kotlin/src/test/java/me/really/codec/ReallyMeCodecJavaTest.java",
  "ReallyMeCodec.base64urlEncode",
);
assertContains(
  "packages/kotlin/src/test/java/me/really/codec/ReallyMeCodecJavaTest.java",
  'assertNull(ReallyMeCodec.tryParseCid("not-a-cid"))',
);
assertContains(
  "packages/kotlin/src/test/java/me/really/codec/ReallyMeCodecJavaTest.java",
  "ReallyMeCodec.base58btcEncode(oversizedBase58Input)",
);
assertContains(
  "packages/kotlin/src/test/java/me/really/codec/ReallyMeCodecJavaTest.java",
  "javaCallersCanProcessSharedProtoVector",
);
// Keep the fixture set anchored outside the manifest itself. A checksum field
// beside its own bytes cannot detect a coordinated golden-vector edit.
const EXPECTED_CODEC_VECTORS_SHA256 =
  "36e558e2cc8cd1ec50171ff3fb5478ebcde03d6eacd5df7192b13e05d70905d8";
const codecVectorDigest = createHash("sha256")
  .update(readText("vectors/codec-vectors.json"), "utf8")
  .digest("hex");
if (codecVectorDigest !== EXPECTED_CODEC_VECTORS_SHA256) {
  fail("vectors/codec-vectors.json differs from the reviewed fixture set");
}
const codecVectorManifest = readJson("vectors/codec-vectors.json");
if (codecVectorManifest.schemaVersion !== 2) {
  fail("vectors/codec-vectors.json must use schemaVersion 2");
}
for (const key of [
  "base64MissingPadding",
  "base64NonCanonicalTrailingBits",
  "base64urlPadded",
  "base64urlNonCanonicalTrailingBits",
  "unsupportedMultibase",
  "nonCanonicalBase64urlMultikey",
  "p256Multikey",
  "dagCborNonCanonicalIntegerHex",
  "dagCborDuplicateKeyHex",
  "dagCborOutOfOrderKeyHex",
  "jcsDuplicateMemberJson",
  "jcsNonInteroperableIntegerJson",
  "jcsLoneSurrogateJson",
]) {
  if (typeof codecVectorManifest.vectors?.[key] !== "string") {
    fail(`vectors/codec-vectors.json is missing rejection vector ${key}`);
  }
}
for (const provenance of ["official", "trusted-upstream", "reallyme-pinned"]) {
  if (!codecVectorManifest.sources.some((source) => source.provenance === provenance)) {
    fail(`vectors/codec-vectors.json is missing ${provenance} provenance`);
  }
}
const deterministicCborVectors = codecVectorManifest.deterministicCbor;
const deterministicCborMinimumCounts = Object.freeze({
  positive: 30,
  negative: 15,
  equivalentInputOrders: 1,
  resourceRejections: 5,
  interoperability: 4,
});
for (const [section, fixtureClass] of Object.entries({
  positive: "golden",
  negative: "rejection-fixture",
  equivalentInputOrders: "golden",
  resourceRejections: "construction-recipe",
  interoperability: "interop-fixture",
})) {
  if (deterministicCborVectors?.fixtureClasses?.[section] !== fixtureClass) {
    fail(`vectors/codec-vectors.json must label ${section} as ${fixtureClass}`);
  }
}
for (const [section, minimumCount] of Object.entries(deterministicCborMinimumCounts)) {
  const fixtures = deterministicCborVectors?.[section];
  if (!Array.isArray(fixtures) || fixtures.length < minimumCount) {
    fail(`vectors/codec-vectors.json must include at least ${minimumCount} ${section} fixtures`);
  }
}
assertContains(
  "scripts/validate_codec_vectors.mjs",
  "deterministicMinimumCounts",
);
assertContains(
  "scripts/validate_codec_vectors.mjs",
  "fixtures.length < minimumCount",
);
for (const fixture of deterministicCborVectors?.interoperability ?? []) {
  if (fixture.fixtureKind !== "synthetic") {
    fail(`interoperability fixture ${fixture.name} must declare fixtureKind`);
  }
  if (fixture.sourceRepo !== "reallyme/idkit-ios") {
    fail(`interoperability fixture ${fixture.name} must pin sourceRepo`);
  }
  if (fixture.sourceCommit !== "content-hash-pinned") {
    fail(`interoperability fixture ${fixture.name} must pin sourceCommit`);
  }
  if (
    typeof fixture.source !== "string" ||
    fixture.source.length === 0 ||
    typeof fixture.explanation !== "string" ||
    fixture.explanation.length === 0
  ) {
    fail(`interoperability fixture ${fixture.name} must explain provenance`);
  }
  if (!Array.isArray(fixture.sourceFiles) || fixture.sourceFiles.length === 0) {
    fail(`interoperability fixture ${fixture.name} must pin source files`);
  }
  for (const sourceFile of fixture.sourceFiles) {
    if (
      typeof sourceFile.path !== "string" ||
      sourceFile.path.length === 0 ||
      !/^[0-9a-f]{64}$/u.test(sourceFile.sha256)
    ) {
      fail(`interoperability fixture ${fixture.name} has invalid source file provenance`);
    }
  }
}
assertContains("vectors/README.md", "official");
assertContains("vectors/README.md", "trusted-upstream");
assertContains("vectors/README.md", "reallyme-pinned");
assertContains("vectors/README.md", "golden");
assertContains("vectors/README.md", "rejection-fixture");
assertContains("vectors/README.md", "construction-recipe");
assertContains("vectors/README.md", "interop-fixture");
assertContains("crates/codec/tests/vector_suite/core_methods.rs", "shared_vector_suite_covers_core_codec_methods");
assertContains("crates/codec/tests/vector_suite/core_methods.rs", "shared_vector_suite_rejects_non_canonical_inputs");
assertContains("packages/ts/test/reallyme-codec.test.mjs", "shared codec vector suite covers TypeScript public methods");
assertContains("packages/ts/test/reallyme-codec.test.mjs", "shared codec vector suite rejects non-canonical inputs in TypeScript");
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", "sharedVectorSuiteCoversKotlinPublicMethods");
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", "sharedVectorSuiteRejectsNonCanonicalInputs");
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", "codec.processOperationJson");
assertContains("packages/kotlin/src/test/java/me/really/codec/ReallyMeCodecJavaTest.java", "ReallyMeCodec.processOperationJson");

assertContains("Package.swift", 'name: "reallyme-codec"');
if (!readText("Package.swift").startsWith("// swift-tools-version: 6.3\n")) {
  fail("Package.swift must declare Swift tools version 6.3");
}
assertContains("Package.swift", 'name: "ReallyMeCodec"');
assertNotContains("Package.swift", 'name: "ReallyMeCodecProto"');
assertContains("Package.swift", 'path: "packages/swift/Sources/ReallyMeCodec"');
if (readText("packages/swift/Sources/ReallyMeCodec/Generated/CodecProto.swift") !== readText("gen/swift/reallyme/codec/v1/codec.pb.swift")) {
  fail("Swift internal protobuf source differs from the generated wire contract");
}
assertNotContains("packages/swift/Sources/ReallyMeCodec/Generated/CodecProto.swift", "public nonisolated struct ReallyMeProto");
assertNotContains("gen/swift/reallyme/codec/v1/codec.pb.swift", "public nonisolated struct ReallyMeProto");
assertContains("buf.gen.yaml", "Visibility=Internal");
assertContains("Package.swift", 'name: "ReallyMeCodecFFI"');
assertContains("Package.swift", 'from: "1.40.0"');
assertContains("Package.swift", "ReallyMeCodecFFI.xcframework.zip");
assertContains("Package.swift", 'let ffiArtifactLocalPathOverride = "');
assertContains("Package.swift", 'ffiArtifactVersion == packageVersion');
assertContains("Package.swift", "path: ffiArtifactLocalPathOverride");
assertContains("Package.swift", "runtimeFfiPathExists = FileManager.default.fileExists(");
assertContains("Package.swift", "runtimeFfiPathIsDirectory.boolValue");
assertContains("Package.swift", "REALLYME_CODEC_SWIFTPM_RUNTIME_FFI");
assertContains("packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift", "public func tryParseCid(_ cid: String) throws(ReallyMeCodecError) -> String?");
assertContains("packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift", "public func dagCborCodecCode() throws(ReallyMeCodecError) -> UInt32");
assertContains("packages/swift/Sources/ReallyMeCodec/WithCodecBytes.swift", "consuming [UInt8]");
assertContains("packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift", "withTextBytes");
assertContains("packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift", "withOwnedBytes");
assertContains("packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift", "operationResult.result = nil");
assertContains("packages/swift/Sources/ReallyMeCodec/MemoryHygiene.swift", "memset_s");
assertContains("packages/swift/Sources/ReallyMeCodec/MemoryHygiene.swift", "explicit_bzero");
assertContains("packages/swift/Sources/ReallyMeCodec/MemoryHygiene.swift", "clearOwned");
assertNotContains("packages/swift/Sources/ReallyMeCodec/MemoryHygiene.swift", "resetBytes");
assertNotContains("packages/swift/Sources/ReallyMeCodec/MemoryHygiene.swift", "initialize(repeating:");
assertContains("packages/swift/Sources/ReallyMeCodec/ProcessCbor.swift", "swap(&value.value, &detachedValue)");
assertContains("packages/swift/Sources/ReallyMeCodec/ProcessCbor.swift", "detachedValue = nil");
assertContains("packages/swift/Sources/ReallyMeCodec/CallCodecWithRustCAbi.swift", "expectedCodecAbiVersion");
assertContains(
  "packages/swift/Sources/ReallyMeCodec/CallCodecWithRustCAbi.swift",
  "expectedCodecAbiVersion: UInt32 = 6",
);
assertContains(
  "packages/swift/Sources/ReallyMeCodec/CallCodecWithRustCAbi.swift",
  "rm_codec_process_operation",
);
assertContains(
  "packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift",
  "func processGeneratedOperation(request: [UInt8]) throws(ReallyMeCodecError) -> ReallyMeProtoCodecOperationResult",
);
assertContains(
  "packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift",
  "options.messageDepthLimit = maxDeterministicCborProtoMessageDepth",
);
for (const resultCase of [
  ".multicodecPrefixForName(let result)",
  ".multicodecLookupPrefix(let result)",
  ".multicodecTable(let result)",
  ".multikeyParse(var result)",
  ".dagCborVerifyCid(let result)",
  ".pemDecode(var result)",
]) {
  assertContains(
    "packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift",
    resultCase,
  );
}
assertContains(
  "packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift",
  "ReallyMeCodecMemory.clearOwned(&responseBytes)",
);
assertContains("packages/swift/Sources/ReallyMeCodec/CallCodecWithRustCAbi.swift", "throw ReallyMeCodecError.providerFailure");
assertContains("packages/swift/Sources/ReallyMeCodec/CallCodecWithRustCAbi.swift", ".canonicalInternal");
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", 'XCTAssertNil(try codec.tryParseCid("not-a-cid"))');
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", "XCTAssertEqual(try codec.dagCborCodecCode(), 0x71)");
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", 'codec.base58btcDecode("")');
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", "codec.base58btcEncode(oversizedBase58Input)");
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", 'codec.multicodecPrefixForName("not-a-codec")');
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", "codec.multicodecLookupPrefix([0, 0, 7])");
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", 'codec.dagCborVerifyCid(cid: "", bytes: encoded)');
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", "testSharedVectorSuiteCoversSwiftPublicMethods");
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", "testSharedVectorSuiteRejectsNonCanonicalInputs");
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", "codec.processOperationJson");
assertContains("packages/swift/Tests/ReallyMeCodecTests/ProviderSafetyTests.swift", "testAbiVersionMismatchFailsClosed");
assertContains("packages/swift/Tests/ReallyMeCodecTests/ProviderSafetyTests.swift", "testSensitiveGeneratedMultikeyRequestFormattingIsRedacted");
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", "testSwiftDataAndCborBuildersPreserveCanonicalBytes");
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", "deterministicAndDagCborBuildersPreserveCanonicalBytes");
assertContains("packages/ts/test/reallyme-codec.test.mjs", "CBOR helper builders preserve canonical bytes");
assertContains("README.md", "Deterministic generic CBOR is a first-class structured surface");
assertContains("packages/swift/README.md", "SwiftProtobuf message-depth limit");
assertContains("packages/kotlin/README.md", "does not keep a parallel hand-written JSON result path");
assertContains("packages/ts/README.md", "the package does not");
assertContains("packages/ts/README.md", "operation-specific `*Proto` helper APIs");
assertContains("scripts/build_swift_xcframework.sh", "xcodebuild -create-xcframework");
assertContains("scripts/build_swift_xcframework.sh", "-C panic=unwind");
assertContains("scripts/build_swift_xcframework.sh", "cargo build --locked");
assertContains("scripts/build_swift_xcframework.sh", "rm_codec_abi_version");
assertContains("scripts/build_swift_xcframework.sh", "rm_codec_max_operation_response_bytes");
assertContains("scripts/build_swift_xcframework.sh", "rm_codec_max_ffi_input_bytes");
assertContains("scripts/build_swift_xcframework.sh", "rm_codec_max_ffi_output_bytes");
assertContains("scripts/test_ffi_abi_release_artifact.sh", "env -u RUSTFLAGS cargo build --locked -p reallyme-codec-ffi --release");
assertContains("scripts/test_ffi_abi_release_artifact.sh", "-C panic=unwind");
assertContains("scripts/test_ffi_abi_release_artifact.sh", "rm_codec_process_operation_json");
assertContains("packages/swift/README.md", "cargo build --locked");
assertContains("docs/protobuf.md", "any SwiftProtobuf.Message");
assertContains("docs/protobuf.md", "serde_json::to_*");
assertContains("docs/protobuf.md", "TypeScript protobuf-es messages are plain JavaScript objects");
assertContains("docs/protobuf.md", "degrade toward quadratic behavior");
assertContains("scripts/build_swift_xcframework.sh", 'cat >"${slice}/Headers/module.modulemap"');
assertContains("scripts/build_swift_xcframework.sh", 'header "reallyme_codec_ffi.h"');
assertNotContains("packages/swift/Sources/ReallyMeCodec/CallCodecWithRustCAbi.swift", "@_silgen_name(");
assertNotContains("scripts/build_swift_xcframework.sh", 'cat >"${slice}/Modules/module.modulemap"');
assertContains("scripts/build_swift_xcframework.sh", "verify_xcframework_layout");
assertContains("scripts/build_swift_xcframework.sh", "normalize_xcframework_info_plist");
assertContains("scripts/build_swift_xcframework.sh", "Headers/module.modulemap");
assertContains("scripts/prepare_swift_binary_manifest.mjs", "ffiArtifactChecksum");
assertContains("scripts/prepare_swift_binary_manifest.mjs", "--local-artifact-path");
assertContains("packages/swift/README.md", "REALLYME_CODEC_SWIFTPM_RUNTIME_FFI=1");
assertNotContains(".github/workflows/swift-package-preflight.yml", "REALLYME_CODEC_SWIFTPM_RUNTIME_FFI");
assertNotContains(".github/workflows/swift-package-preflight.yml", "cargo build --locked -p reallyme-codec-ffi");
assertNotContains(".github/workflows/swift-package-preflight.yml", "node --test scripts/verify_swift_release_artifact.test.mjs");
assertContains(".github/workflows/protobuf-ci.yml", "scripts/build_kotlin_native_resource.sh build/sdk-native-resources");
assertContains(".github/workflows/protobuf-ci.yml", "node scripts/write_native_manifest.mjs build/sdk-native-resources");
assertContains(".github/workflows/protobuf-ci.yml", "/target/release/libreallyme_codec_ffi.dylib");
assertContains(".github/workflows/protobuf-ci.yml", "REALLYME_CODEC_KOTLIN_NATIVE_RESOURCES_DIR");
assertContains(".github/workflows/swift-package-preflight.yml", "Build SwiftPM binary artifact");
assertContains(".github/workflows/swift-package-preflight.yml", "Upload Swift release candidate");
assertContains(".github/workflows/swift-package-preflight.yml", "Bind manifest to Swift release candidate");
assertContains(".github/workflows/swift-package-preflight.yml", "Verify generated SwiftPM manifest and release candidate");
assertContains(
  ".github/workflows/swift-package-preflight.yml",
  `node scripts/verify_swift_release_artifact.mjs build/swift/ReallyMeCodecFFI.xcframework.zip build/swift/ReallyMeCodecFFI.xcframework.checksum Package.swift "\${RELEASE_VERSION}"`,
);
assertContains(".github/workflows/swift-package-preflight.yml", "Prepare local SwiftPM binary manifest");
assertContains(".github/workflows/swift-package-preflight.yml", "--local-artifact-path build/swift/ReallyMeCodecFFI.xcframework");
assertNotContains(".github/workflows/swift-package-preflight.yml", "swift package reset");
assertContains(".github/workflows/swift-package-preflight.yml", "Test Swift package with linked binary target");
assertContains(".github/workflows/swift-package-preflight.yml", "swift test --arch arm64 --verbose");
assertContains("scripts/verify_release_attestation.mjs", "PROTOBUF_WORKFLOW");
assertContains("scripts/verify_swift_release_artifact.mjs", "Package.swift local artifact override must be empty");

assertSubstringsInOrder(".github/workflows/swift-package-preflight.yml", [
  "Build SwiftPM binary artifact",
  "Bind manifest to Swift release candidate",
  "Verify generated SwiftPM manifest and release candidate",
  "Prepare local SwiftPM binary manifest",
  "Test Swift package with linked binary target",
  "Upload Swift release candidate",
]);
assertContains("packages/ts/src/multiformat.ts", "ensureStringValue(encoded)");
assertContains("packages/ts/src/multiformat.ts", "snapshotBoundedBytesInput(bytes)");
assertContains("packages/ts/src/processCbor.ts", "ensureStringValue(cid)");
assertContains("packages/ts/src/processCbor.ts", "payloadSnapshot.fill(0)");
assertContains("packages/ts/src/validateDagCbor.ts", "readDataProperty");
assertContains("packages/ts/src/validateDagCbor.ts", 'readDataProperty(value, "length")');
assertContains("packages/ts/src/processCbor.ts", 'throw new ReallyMeCodecError("provider-failure")');
assertContains(
  "packages/ts/src/readDeterministicCborProto.ts",
  "recordDeterministicCborMapKey(key, keys, providerFailure)",
);
assertContains("packages/ts/src/boundary.ts", "MAX_CODEC_BOUNDARY_NODES = MAX_CODEC_FFI_INPUT_BYTES");
assertNotContains("packages/ts/src/boundary.ts", "MAX_CODEC_BOUNDARY_NODES = 65_536");
assertContains("packages/ts/src/boundary.ts", 'readOwnDataProperty(value, "length")');
assertContains("packages/ts/src/jcs.ts", "stringifyBoundaryJson");
assertContains("packages/ts/src/multiformat.ts", "MAX_MULTICODEC_TABLE_ENTRIES");
assertContains("packages/ts/src/pem.ts", "snapshotDecodePolicy");
assertContains("packages/ts/src/pem.ts", "const pemSnapshot = snapshotBoundedBytesInput(input, MAX_CODEC_PEM_INPUT_BYTES)");
assertContains("packages/ts/src/pem.ts", "const derSnapshot = snapshotBoundedBytesInput(der)");
assertContains("packages/ts/src/operationContract.ts", "requestBytes.fill(0)");
assertContains("packages/ts/src/operationContract.ts", "responseBytes.fill(0)");
assertContains("packages/ts/src/operationContract.ts", "const requestSnapshot = snapshotBoundedBytesInput(");
assertContains("packages/ts/src/operationContract.ts", "requestSnapshot.fill(0)");
assertContains(
  "packages/ts/src/operationContract.ts",
  "MAX_CODEC_PROTO_JSON_BYTES",
);
assertContains(
  "packages/ts/src/operationContract.ts",
  "clearGeneratedOperationResult(response.outcome.value)",
);
assertContains("packages/ts/src/clearGeneratedOperation.ts", "wipeGeneratedCborValue");
assertContains("packages/ts/src/wasmProvider.ts", '"processOperation"');
assertContains(
  "packages/ts/src/operationContract.ts",
  "export const processGeneratedOperationRequest =",
);
assertContains("packages/ts/src/operationContract.ts", "CodecOperationResponseSchema");
assertContains(
  "packages/ts/src/operationContract.ts",
  "detachGeneratedOperationResultBytes(response.outcome.value)",
);
assertContains(
  "packages/ts/src/clearGeneratedOperation.ts",
  "export const clearGeneratedOperationResult =",
);
assertContains("packages/ts/src/proto.ts", "CodecOperationResponseSchema");
assertContains("packages/ts/src/proto.ts", "CodecOperationResultSchema");
assertContains("packages/ts/src/operationContract.ts", "errorCodeForCodecErrorMessage");
assertContains("packages/ts/src/operationContract.ts", "MAX_CODEC_PROTO_MESSAGE_BYTES");
assertContains("packages/ts/src/classifyOperationError.ts", "CodecErrorReason.CANONICAL_INTERNAL");
assertContains("packages/ts/src/classifyOperationError.ts", 'case "boundary":');
assertContains("packages/ts/src/classifyOperationError.ts", "expectedOrigin = CodecErrorOrigin.CALLER");
assertNotContains("packages/ts/tsconfig.json", '"DOM"');
assertContains("packages/ts/src/readOutput.ts", "MAX_CODEC_FFI_OUTPUT_BYTES");
assertContains("packages/ts/src/readOutput.ts", "export const snapshotBoundedBytesInput =");
assertContains("packages/ts/src/readOutput.ts", "ArrayBuffer.isView(value)");
assertContains("packages/ts/src/readOutput.ts", "Object.getPrototypeOf(value) !== Uint8Array.prototype");
assertContains("packages/ts/src/readOutput.ts", "Object.getOwnPropertyDescriptor(value, property)");
assertContains("packages/ts/src/validateGeneratedCbor.ts", "if (text.length > maximum)");
assertContains(
  "packages/ts/src/validateDagCbor.ts",
  "MAX_DETERMINISTIC_CBOR_NODES - state.nodes",
);
assertContains("packages/ts/src/convertDagCbor.ts", "wipeDagCborValueBytes(child)");
assertContains("packages/ts/src/processCbor.ts", "wipeDeterministicCborValueBytes(normalized)");
assertContains("packages/ts/test/reallyme-codec.test.mjs", "decodedBytes.value instanceof Uint8Array");
assertContains(
  "packages/ts/test/dag-cbor-provider.test.mjs",
  "DAG-CBOR decode wipes partially built byte nodes when a later node fails",
);
assertContains(
  "packages/ts/test/dag-cbor-provider.test.mjs",
  "wipedSensitiveOwners.length >= 3",
);
assertContains(
  "packages/ts/test/reallyme-codec.test.mjs",
  "Symbol.toStringTag",
);
assertContains(
  "packages/ts/test/reallyme-codec.test.mjs",
  "multicodecStripPrefix(forged)",
);
assertContains("packages/ts/src/wasmProvider.ts", "Object.getOwnPropertyDescriptor(module, name)");
assertContains(
  "packages/ts/test/deterministic-cbor-provider.test.mjs",
  "deterministic CBOR rejects duplicate provider map keys",
);

// The multicodec pilot freezes one semantic implementation per operation.
// Boundary adapters may translate representations, but must never regain a
// direct dependency on the primitive registry or primitive lookup function.
assertContains("crates/codec/src/lib.rs", "pub mod multicodec;");
assertNotContains("crates/codec/src/lib.rs", "multicodec_ops");
assertNotContains("crates/codec/src/lib.rs", "dag_cbor_ops");
assertNotContains("crates/codec/src/lib.rs", "multikey_ops");
assertNotContains("crates/codec/src/lib.rs", "pem_ops");
assertContains("crates/codec/src/operation_contract/core/mod.rs", "mod dag_cbor;");
assertContains("crates/codec/src/operation_contract/core/mod.rs", "mod multikey;");
assertContains("crates/codec/src/operation_contract/core/mod.rs", "mod pem;");
for (const semanticOperation of [
  "pub fn prefix_for_name(",
  "pub fn lookup_prefix(",
  "pub fn strip_prefix(",
  "pub fn supported_table(",
]) {
  assertContains("crates/codec/src/multicodec.rs", semanticOperation);
}
assertContains("crates/codec/src/multicodec.rs", ".try_reserve(capacity)");
assertNotContains(
  "crates/codec/src/multicodec.rs",
  "#[derive(Debug, Clone, PartialEq, Eq)]\npub struct MulticodecTable",
);
assertContains(
  "crates/codec/src/multicodec.rs",
  "MulticodecOperationError::AllocationFailure",
);
for (const adapterPath of [
  codecContractExecuteMultiformatPath,
  "crates/ffi/src/codec.rs",
  "crates/wasm/src/multiformat.rs",
]) {
  assertNotContains(adapterPath, "codec_multicodec");
  assertNotContains(adapterPath, "lookup_codec_prefix");
}
assertContains(codecContractMapErrorsPath, "MulticodecOperationError::AllocationFailure");
assertContains("crates/ffi/src/codec.rs", "MulticodecOperationError::AllocationFailure");
assertContains(
  "crates/wasm/src/multiformat.rs",
  "MulticodecOperationError::AllocationFailure",
);
for (const supersededWasmStructuredExport of [
  "multicodecPrefixForName",
  "multicodecLookupPrefix",
  "multicodecTable",
  "multikeyParse",
  "dagCborVerifyCid",
  "pemDecode",
]) {
  assertNotContains(codecWasmProviderPath, `"${supersededWasmStructuredExport}"`);
}
assertContains(
  "packages/ts/test/reallyme-codec.test.mjs",
  "superseded direct WASM structured result exports are absent",
);
if (listFiles("crates/wasm/src").includes("crates/wasm/src/write_js_object.rs")) {
  fail("superseded WASM JS-object result writer must not be restored");
}
assertNotContains("crates/wasm/src/multiformat.rs", "js_name = multicodecPrefixForName");
assertNotContains("crates/wasm/src/multiformat.rs", "js_name = multicodecLookupPrefix");
assertNotContains("crates/wasm/src/multiformat.rs", "js_name = multicodecTable");
assertNotContains("crates/wasm/src/multiformat.rs", "js_name = multikeyParse");
assertNotContains("crates/wasm/src/cbor.rs", "js_name = dagCborVerifyCid");
assertContains("packages/ts/src/proto.ts", "CodecBackendErrorSchema");
assertContains("packages/ts/test/reallyme-codec.test.mjs", 'assert.deepEqual(base58btcDecode(""), bytes())');
assertContains("packages/ts/test/reallyme-codec.test.mjs", 'dagCborVerifyCidDetails("", encoded)');
assertContains("packages/ts/src/processCbor.ts", "export const dagCborVerifyCid = (cid: string, bytes: Uint8Array): boolean =>");
assertContains("packages/ts/test/reallyme-codec.test.mjs", "binary protobuf and generated ProtoJSON return equivalent responses");
assertContains("packages/ts/test/reallyme-codec.test.mjs", "superseded direct WASM structured result exports are absent");
assertContains("packages/ts/test/reallyme-codec.test.mjs", "array metadata is snapshotted once without invoking proxy getters");
assertContains("packages/ts/test/reallyme-codec.test.mjs", "byte boundaries reject proxy-wrapped typed arrays before length reads");
assertContains("packages/ts/test/reallyme-codec.test.mjs", "JCS object boundary matches text boundary for bounded JSON values");
assertContains("packages/ts/test/reallyme-codec.test.mjs", 'canonicalizeJsonText("1e19")');
assertContains("packages/ts/test/reallyme-codec.test.mjs", "fc.assert(");
assertContains("packages/ts/test/structured-operation-provider.test.mjs", "public contract processors pass providers a wiped SDK-owned snapshot");
assertContains("packages/ts/test/structured-operation-provider.test.mjs", "public contract processors reject oversized input before provider invocation");
assertContains("packages/ts/src/pem.ts", "maxInputLen: requireProtoUint32(snapshot.maxInputLen)");
assertContains("packages/ts/src/pem.ts", "maxDerLen: requireProtoUint32(snapshot.maxDerLen)");
assertContains("packages/ts/src/pem.ts", "const pemSnapshot = snapshotBoundedBytesInput(input, MAX_CODEC_PEM_INPUT_BYTES)");
assertContains("packages/ts/src/pem.ts", "pemSnapshot.fill(0)");
assertContains("packages/ts/src/pem.ts", "const derSnapshot = snapshotBoundedBytesInput(der)");
assertContains("packages/ts/src/pem.ts", "derSnapshot.fill(0)");
assertContains("packages/ts/test/reallyme-codec.test.mjs", "decodePem rejects oversized proto policy before provider work");
assertContains("packages/ts/test/reallyme-codec.test.mjs", "encodePem rejects oversized proto options before provider work");
// PEM text and DER may be private-key material. They use the generated
// request lane, whose descriptor-driven serializer leaves one wiped copy.
assertTypescriptExport({
  path: "packages/ts/src/pem.ts",
  exportNeedle: "export const encodePem = (",
  requiredNeedles: [
    "processGeneratedOperationRequest(",
    'case: "pemEncode"',
    "derSnapshot.fill(0)",
  ],
});
assertNotContains("packages/ts/src/pem.ts", "toBinary(");
assertContains("packages/ts/src/operationContract.ts", "export const processOperationRequestBytes = (");
// Generated requests are serialized from their descriptors into one exactly
// sized, wiped buffer; the protobuf-es writer would leave unwipeable copies
// of sensitive text and byte fields in its abandoned growth buffers.
assertTypescriptExport({
  path: "packages/ts/src/operationContract.ts",
  exportNeedle: "export const processGeneratedOperationRequest = (",
  requiredNeedles: [
    "validateGeneratedOperationRequest(request)",
    "serializeExactBinary(CodecOperationRequestSchema, request)",
    "processOperationRequestBytes(requestBytes)",
  ],
  forbiddenNeedles: ["toBinary("],
});
const serializerExceptionPath = "packages/ts/src/serializeExactBinary.ts";
const serializerExceptionDigest = createHash("sha256")
  .update(readFileSync(new URL(`../${serializerExceptionPath}`, import.meta.url))).digest("hex");
if (serializerExceptionDigest !== "e9f644b3c10200cf1db04434d1fcec0197c83f68e81199eb7ec61407acc861be") {
  fail("the temporary TypeScript serializer exception requires review");
}
assertContains(serializerExceptionPath, "reflect(schema, message, false)");
assertContains("packages/ts/src/serializeExactBinary.ts", "message.sortedFields");
assertContains("packages/ts/test/reallyme-codec.test.mjs", "exact request serialization matches protobuf-es for every request shape");
assertContains("packages/ts/test/reallyme-codec.test.mjs", "exact request serialization retains only the returned sensitive buffer");
assertContains("packages/ts/test/reallyme-codec.test.mjs", "exact request serialization wipes partial output on failure");
assertContains("packages/ts/test/reallyme-codec.test.mjs", "sensitive CBOR, multicodec, and PEM lanes leave no serializer copy of caller bytes");
// The packaged glue must refuse WASM bytes other than the ones built with it.
assertContains("packages/ts/scripts/build-wasm.mjs", "wasmIntegritySource(wasmDigest)");
assertContains("packages/ts/scripts/wasm-integrity.mjs", "__reallymeVerifiedWasmBytes");
assertContains("packages/ts/test/reallyme-codec.test.mjs", "WASM initialization refuses bytes other than the packaged module");
assertContains("scripts/verify_swift_release_artifact.mjs", "Package.swift binary target URL is not bound to ffiArtifactVersion");
assertContains("scripts/verify_swift_release_artifact.mjs", "Package.swift binary target checksum is not bound to ffiArtifactChecksum");
assertContains(
  "scripts/verify_swift_release_artifact.mjs",
  "assertRegularFile(archivePath, MAX_ARCHIVE_BYTES",
);
assertNotContains(
  "scripts/verify_swift_release_artifact.mjs",
  'readRegularFile(archivePath, MAX_ARCHIVE_BYTES',
);
assertContains("scripts/verify_swift_release_artifact.test.mjs", "Swift release verifier rejects unused manifest checksum variables");
assertContains("scripts/verify_release_attestation.mjs", "displayTitle");
assertContains("scripts/verify_release_attestation.mjs", "PREFLIGHT_WORKFLOW_TITLES");
assertContains("scripts/verify_release_attestation.test.mjs", "package preflight attestation is bound to the requested version");
assertContains(".github/workflows/swift-package-preflight.yml", "run-name: Swift package preflight ${{ inputs.version }}");
assertContains(".github/workflows/kotlin-android-package-preflight.yml", "run-name: Kotlin Android package preflight ${{ inputs.version }}");
assertContains(".github/workflows/npm-package-preflight.yml", "run-name: npm package preflight ${{ inputs.version }}");
assertContains(".github/workflows/crates-package-preflight.yml", "run-name: Crates package preflight ${{ inputs.version }}");
assertContains(".github/workflows/crates-package-preflight.yml", "cargo-audit@0.23.0");
assertContains(".github/workflows/crates-package-preflight.yml", "scripts/audit_committed_lockfiles.sh");
assertContains("deny.toml", 'yanked = "deny"');
assertMinOccurrences(".github/workflows/swift-package-release.yml", "RELEASE_VERSION: ${{ inputs.version }}", 2);
assertMinOccurrences(".github/workflows/kotlin-android-package-release.yml", "RELEASE_VERSION: ${{ inputs.version }}", 2);
assertMinOccurrences(".github/workflows/npm-package-release.yml", "RELEASE_VERSION: ${{ inputs.version }}", 2);
assertMinOccurrences(
  ".github/workflows/crates-release.yml",
  "RELEASE_VERSION: ${{ steps.resolve-release-sha.outputs.release_version }}",
  1,
);
assertMinOccurrences(
  ".github/workflows/crates-release.yml",
  "RELEASE_VERSION: ${{ needs.verify-release-sha.outputs.release_version }}",
  1,
);
if (releasePackagesMode) {
  const swiftPackage = readText("Package.swift");
  if (swiftPackage.includes('let ffiArtifactChecksum = "0000000000000000000000000000000000000000000000000000000000000000"')) {
    fail("Package.swift still has the Swift binary artifact checksum placeholder");
  }
  if (!swiftPackage.includes('let ffiArtifactLocalPathOverride = ""')) {
    fail("Package.swift must use the release URL artifact in release package mode");
  }
  assertContains("Package.swift", 'codecTargetDependencies.append("ReallyMeCodecFFI")');
  assertContains("Package.swift", 'codecSwiftSettings.append(.define("REALLYME_CODEC_LINKED_FFI"))');
}
if (readText("Package.swift").includes("ReallyMeCrypto")) {
  fail("Package.swift still exposes crypto Swift products");
}

const androidCodecBuild = readText("packages/kotlin-android/build.gradle.kts");
if (!androidCodecBuild.includes(`version = "${codecPackageVersion}"`)) {
  fail(`packages/kotlin-android/build.gradle.kts is not versioned ${codecPackageVersion}`);
}
assertContains("packages/kotlin-android/settings.gradle.kts", 'rootProject.name = "reallyme-codec-android"');
assertContains("packages/kotlin-android/settings.gradle.kts", 'include(":consumer-r8-runtime")');
assertContains("packages/kotlin-android/build.gradle.kts", 'id("com.android.library")');
assertContains("packages/kotlin-android/build.gradle.kts", 'id("com.android.library") version "9.5.1"');
assertContains("packages/kotlin-android/build.gradle.kts", 'id("com.android.application") version "9.5.1" apply false');
assertContains("packages/kotlin-android/build.gradle.kts", "compileSdk = 37");
assertContains("packages/kotlin-android/consumer-r8-runtime/build.gradle.kts", "compileSdk = 37");
assertContains("packages/kotlin-android/consumer-r8-runtime/build.gradle.kts", "targetSdk = 37");
assertContains("scripts/test_android_consumer_r8_runtime.sh", 'ANDROID_R8_PLATFORM="platforms;android-37.0"');
assertContains("scripts/test_android_consumer_r8_runtime.sh", 'ANDROID_R8_SYSTEM_IMAGE="system-images;android-37.1;google_apis_ps16k;x86_64"');
assertContains("packages/kotlin-android/build.gradle.kts", 'artifactId = "codec-android"');
assertContains("packages/kotlin-android/build.gradle.kts", "com.google.protobuf:protobuf-javalite:4.36.2");
assertContains("packages/kotlin-android/build.gradle.kts", "com.google.protobuf:protobuf-kotlin-lite:4.36.2");
assertContains("packages/kotlin-android/build.gradle.kts", "jniLibs.directories");
assertContains("packages/kotlin-android/build.gradle.kts", "assets.directories");
assertContains("packages/kotlin-android/build.gradle.kts", "reallyme-codec/native-manifest.json");
assertContains("packages/kotlin-android/build.gradle.kts", "inputs.dir(jniLibsDir).optional()");
assertContains("packages/kotlin-android/build.gradle.kts", "verifyAndroidJniLibs");
assertContains("packages/kotlin-android/build.gradle.kts", "verifyReleaseAarContainsJniLibs");
assertContains("packages/kotlin-android/build.gradle.kts", "verifyAndroidNativeManifestEntry");
assertContains("packages/kotlin-android/build.gradle.kts", "ZipFile(aarFiles.single())");
assertContains("packages/kotlin-android/build.gradle.kts", "Android native manifest digest does not match");
assertContains("packages/kotlin-android/build.gradle.kts", 'keepDebugSymbols.add("**/libreallyme_codec_ffi.so")');
assertContains("packages/kotlin-android/build.gradle.kts", "PublishToMavenRepository");
assertContains("packages/kotlin-android/build.gradle.kts", "dependencyLocking {");
assertContains("packages/kotlin-android/build.gradle.kts", "lockAllConfigurations()");
assertContains(
  "packages/kotlin-android/gradle.properties",
  "org.gradle.dependency.verification=strict",
);
assertContains("packages/kotlin-android/gradle.lockfile", "com.google.protobuf:protobuf-javalite");
assertContains(
  "packages/kotlin-android/gradle/verification-metadata.xml",
  "<verify-metadata>true</verify-metadata>",
);
assertContains(
  "packages/kotlin-android/gradle/verification-metadata.xml",
  "<verify-signatures>false</verify-signatures>",
);
assertContains(
  "packages/kotlin-android/gradle/verification-metadata.xml",
  '<component group="com.android.tools.build" name="gradle" version="9.5.1">',
);
assertContains(
  "packages/kotlin-android/gradle/verification-metadata.xml",
  'artifact name="aapt2-9.5.1-16666295-linux.jar"',
);
assertContains(
  "packages/kotlin-android/gradle/verification-metadata.xml",
  "fa1014cb1431622d292ef55ba870e75882ea19987bec541698b912f446a2d08f",
);
assertContains(
  "packages/kotlin-android/gradle/verification-metadata.xml",
  'artifact name="bc-jdk18on-bom-1.80.2.module"',
);
assertContains(
  "packages/kotlin-android/gradle/verification-metadata.xml",
  "dd652f7a3141dd8ad81406ae5efc9ebeb47c99c0ab136931d1cd087bccd80474",
);
assertContains("packages/kotlin-android/build.gradle.kts", "fun nonBlank(value: String?): String?");
assertContains("packages/kotlin-android/build.gradle.kts", "verifyRemoteMavenPublishingConfigured");
assertContains("packages/kotlin-android/build.gradle.kts", "remote Maven publishing is not configured");
assertContains("packages/kotlin-android/build.gradle.kts", 'parsed.scheme != "https"');
assertNotContains("packages/kotlin-android/build.gradle.kts", "reallyme.maven.requireRemote");
assertContains("packages/kotlin-android/consumer-rules.pro", "ReallyMeCodecException$*");
assertNotContains("packages/kotlin-android/consumer-rules.pro", "ReallyMeCodecProtoStatus");
assertNotContains("packages/kotlin-android/consumer-rules.pro", "ReallyMeCodecProtoResult");
assertContains("packages/kotlin-android/consumer-rules.pro", "me.really.codec.v1.**");
assertContains("packages/kotlin-android/consumer-r8-runtime/build.gradle.kts", "com.android.application");
assertContains("packages/kotlin-android/consumer-r8-runtime/build.gradle.kts", "isMinifyEnabled = true");
assertContains("packages/kotlin-android/consumer-r8-runtime/build.gradle.kts", 'signingConfig = signingConfigs.getByName("debug")');
assertContains("packages/kotlin-android/consumer-r8-runtime/build.gradle.kts", 'implementation(project(":"))');
assertContains("packages/kotlin-android/consumer-r8-runtime/src/main/AndroidManifest.xml", "ConsumerR8RuntimeActivity");
assertContains("packages/kotlin-android/consumer-r8-runtime/src/main/java/me/really/codec/consumer/r8/ConsumerR8RuntimeActivity.java", "ReallyMeCodecR8Gate");
assertContains("packages/kotlin-android/consumer-r8-runtime/src/main/java/me/really/codec/consumer/r8/ConsumerR8RuntimeActivity.java", "ReallyMeCodec.deterministicCborDecode");
assertContains("scripts/test_android_consumer_r8_runtime.sh", ":consumer-r8-runtime:assembleRelease");
assertContains("scripts/test_android_consumer_r8_runtime.sh", "logcat");
assertContains("scripts/test_android_consumer_r8_runtime.sh", 'install --no-streaming -r "$APK_PATH"');
assertContains("scripts/test_android_consumer_r8_runtime.sh", 'ADB_INSTALL_MAX_ATTEMPTS="3"');
assertContains("scripts/test_android_consumer_r8_runtime.sh", 'am get-started-user-state 0');
assertContains("scripts/test_android_consumer_r8_runtime.sh", '"$user_state" == "RUNNING_UNLOCKED"');
assertContains("scripts/test_android_consumer_r8_runtime.sh", '"$volume_state" == *" mounted "*');
assertContains("scripts/test_android_consumer_r8_runtime.sh", 'ANDROID_R8_PAGE_SIZE_BYTES="16384"');
assertContains("scripts/test_android_consumer_r8_runtime.sh", 'shell getconf PAGE_SIZE');
assertContains("scripts/test_android_consumer_r8_runtime.sh", '"$page_size" == "$ANDROID_R8_PAGE_SIZE_BYTES"');
assertContains("scripts/test_android_consumer_r8_runtime.sh", 'service check package');
assertContains("scripts/test_android_consumer_r8_runtime.sh", 'service check mount');
assertContains("scripts/test_android_consumer_r8_runtime.sh", 'sm list-volumes all');
assertContains("scripts/test_android_consumer_r8_runtime.sh", 'pm path android');
assertContains("scripts/test_android_consumer_r8_runtime.sh", '"$install_output" != *"Broken pipe"*');
assertContains("scripts/test_android_consumer_r8_runtime.sh", '"$install_output" != *"StorageManager.getVolumes()"*');
assertContains("scripts/test_android_consumer_r8_runtime.sh", '"$install_output" != *"PackageManagerInternal.freeStorage"*');
assertContains("scripts/test_android_consumer_r8_runtime.sh", '        wait_for_runtime_services\n        if install_output=');
assertContains("scripts/test_android_consumer_r8_runtime.sh", "Android consumer R8 APK installation failed");
assertContains("scripts/test_android_consumer_r8_runtime.sh", "Android consumer R8 activity launch failed");
assertContains("scripts/test_android_consumer_r8_runtime.sh", "ensure_avd_exists");
assertContains("scripts/test_android_consumer_r8_runtime.sh", "ANDROID_AVD_HOME_VALUE");
assertContains("scripts/test_android_consumer_r8_runtime.sh", "export ANDROID_AVD_HOME");
assertContains("scripts/test_android_consumer_r8_runtime.sh", 'ANDROID_R8_DATA_PARTITION_MB="4096"');
assertContains("scripts/test_android_consumer_r8_runtime.sh", '-wipe-data -partition-size "$ANDROID_R8_DATA_PARTITION_MB"');
assertContains("scripts/test_android_consumer_r8_runtime.sh", "$EMULATOR\" -list-avds");
assertContains("scripts/test_android_consumer_r8_runtime.sh", "{ yes 2>/dev/null || true; } | \"$SDKMANAGER\"");
assertContains("scripts/test_android_consumer_r8_runtime.sh", "$SDKMANAGER\" \"emulator\"");
assertContains("scripts/test_android_consumer_r8_runtime.sh", "Android AVD was not available after creation");
assertContains("scripts/test_android_consumer_r8_runtime.sh", "$AVDMANAGER\" create avd --force");
assertContains("scripts/test_android_consumer_r8_runtime.sh", "Android consumer R8 runtime gate passed");
assertContains(
  ".github/workflows/kotlin-android-package-preflight.yml",
  '{ yes 2>/dev/null || true; } | "${ANDROID_HOME}/cmdline-tools/latest/bin/sdkmanager" "platforms;android-37.0" "ndk;29.0.14206865"',
);
assertContains(
  ".github/workflows/kotlin-android-package-release.yml",
  '{ yes 2>/dev/null || true; } | "${ANDROID_HOME}/cmdline-tools/latest/bin/sdkmanager" "platforms;android-37.0" "ndk;29.0.14206865"',
);
assertContains("packages/kotlin-android/README.md", "me.really:codec-android:0.4.0");
assertContains("packages/kotlin-android/README.md", "never sourced from the Git worktree");
assertContains(
  "packages/kotlin-android/gradle.properties",
  "org.gradle.jvmargs=-Xmx1g -XX:MaxMetaspaceSize=768m -Dfile.encoding=UTF-8",
);
for (const trackedAndroidFile of listFiles("packages/kotlin-android")) {
  if (trackedAndroidFile.endsWith(".so")) {
    fail(`${trackedAndroidFile} is a tracked prebuilt Android native library`);
  }
}
assertContains("scripts/build_android_native_resources.sh", "aarch64-linux-android");
assertContains("scripts/build_android_native_resources.sh", "packages/kotlin-android/build/generated/android-jniLibs");
assertContains("scripts/build_android_native_resources.sh", "-C panic=unwind");
assertContains("scripts/build_android_native_resources.sh", "cargo build --locked");
assertContains("scripts/build_android_native_resources.sh", "llvm-strip");
assertContains("scripts/build_android_native_resources.sh", "--strip-debug");
assertContains("scripts/build_android_native_resources.sh", "armv7-linux-androideabi");
assertContains("scripts/build_android_native_resources.sh", "x86_64-linux-android");
assertContains("scripts/build_android_native_resources.sh", "i686-linux-android");
assertContains("scripts/build_kotlin_native_resource.sh", "-C panic=unwind");
assertContains("scripts/build_kotlin_native_resource.sh", "cargo build --locked");
assertContains("packages/kotlin/build.gradle.kts", "-C panic=unwind");
assertContains(".github/workflows/kotlin-android-package-release.yml", "ANDROID_NDK_HOME=${ANDROID_HOME}/ndk/29.0.14206865");
assertNotContains(".github/workflows/kotlin-android-package-release.yml", "ANDROID_NDK_HOME: ${{ env.ANDROID_HOME }}/ndk/29.0.14206865");
assertContains(".github/workflows/kotlin-android-package-preflight.yml", "ANDROID_NDK_HOME=${ANDROID_HOME}/ndk/29.0.14206865");
assertContains(".github/workflows/kotlin-android-package-preflight.yml", "Write Android native checksum manifest");
assertContains("scripts/maven_central_bundle_local.sh", "validate_bundle_files");
assertContains(".github/workflows/kotlin-android-package-release.yml", "RELEASE_VERSION");
assertContains(".github/workflows/swift-package-release.yml", "needs: [verify-release-sha, swift-verify]");
assertNotContains(".github/workflows/swift-package-release.yml", "if: inputs.publish == true");
assertNotContains(".github/workflows/kotlin-android-package-release.yml", "if: inputs.publish == true");
assertNotContains(".github/workflows/npm-package-release.yml", "if: inputs.publish == true");
assertContains(".github/workflows/kotlin-android-package-preflight.yml", "android aar preflight");
assertContains(".github/workflows/kotlin-android-package-preflight.yml", "requireAndroidJniLibs=true");
assertNotContains(".github/workflows/kotlin-android-package-preflight.yml", "Install Android emulator image");
assertNotContains(".github/workflows/kotlin-android-package-preflight.yml", "Test Android consumer R8 runtime");
assertContains(".github/workflows/kotlin-android-package-preflight.yml", "Build Android consumer R8 APK");
assertContains(".github/workflows/android-runtime-gate.yml", "Android Runtime Gate");
assertContains(".github/workflows/android-runtime-gate.yml", "Enable Android emulator KVM access");
assertContains(".github/workflows/android-runtime-gate.yml", 'sudo chown "${USER}:${USER}" /dev/kvm');
assertContains(".github/workflows/android-runtime-gate.yml", "Test Android consumer R8 runtime");
assertContains(".github/workflows/android-runtime-gate.yml", "REALLYME_CODEC_ANDROID_AVD: reallyme-r8-gate");
assertContains(".github/workflows/android-runtime-gate.yml", "timeout-minutes: 20");

assertContains("packages/kotlin/build.gradle.kts", 'val localReleaseRepositoryDir = providers.gradleProperty("reallyme.maven.localReleaseRepositoryDir")');
assertContains("packages/kotlin/build.gradle.kts", 'name = "localRelease"');
assertContains("packages/kotlin/build.gradle.kts", 'if (name.endsWith("ToRemoteReleaseRepository"))');
assertContains("packages/kotlin-android/build.gradle.kts", 'val localReleaseRepositoryDir = providers.gradleProperty("reallyme.maven.localReleaseRepositoryDir")');
assertContains("packages/kotlin-android/build.gradle.kts", 'name = "localRelease"');
assertContains("packages/kotlin-android/build.gradle.kts", 'if (name.endsWith("ToRemoteReleaseRepository"))');

assertContains("packages/ts/package.json", '"test:browser": "npm run build && npm run test:browser:built"');
assertContains("packages/ts/package.json", '"test:browser:built": "node scripts/browser-wasm-test.mjs"');
assertContains("packages/ts/package.json", '"test": "npm run build && node --test \\"test/*.test.mjs\\""');
assertSubstringsInOrder(".github/workflows/protobuf-ci.yml", [
  "node scripts/run_pinned_release_readiness.mjs --generated-freshness",
  "run: npm --prefix packages/ts run test:browser:built",
]);
for (const path of [
  ".github/workflows/npm-package-preflight.yml",
  ".github/workflows/npm-package-release.yml",
]) {
  assertSubstringsInOrder(path, ["run: npm test", "run: npm run test:browser:built"]);
  assertNotContains(path, "run: npm run test:browser\n");
}
assertNotContains(".github/workflows/code-checks.yml", "run: npm --prefix packages/ts run test:browser\n");
assertContains(".github/workflows/npm-package-preflight.yml", "npm pack --dry-run --ignore-scripts");
assertContains("packages/ts/scripts/browser-wasm-test.mjs", "__REALLYME_CODEC_BROWSER_WASM_RESULT__");
assertContains("packages/ts/scripts/browser-wasm-test.mjs", "installReallyMeCodecWasmProvider");
assertContains("packages/ts/scripts/browser-wasm-test.mjs", "deterministicCborDecode");
assertContains("packages/ts/scripts/browser-wasm-test.mjs", '"@bufbuild/protobuf/reflect": "/node_modules/@bufbuild/protobuf/dist/esm/reflect/index.js"');
assertContains("packages/ts/scripts/browser-wasm-test.mjs", '"@bufbuild/protobuf/wire": "/node_modules/@bufbuild/protobuf/dist/esm/wire/index.js"');
assertContains("packages/ts/scripts/browser-wasm-test.mjs", "Chrome or Chromium is required");
assertContains(".github/workflows/protobuf-ci.yml", "Test TypeScript codec package in browser");
assertContains(".github/workflows/npm-package-preflight.yml", "Test TypeScript codec package in browser");

assertContains("README.md", "https://github.com/reallyme/codec");
assertContains("README.md", "https://www.npmjs.com/package/@reallyme/codec");
assertContains("README.md", "me.really:codec:0.4.0");
assertContains("README.md", "reallyme-codec-proto");
assertContains("README.md", "## Published Surfaces");
assertContains("README.md", "`me.really:codec-android` AAR");
assertContains("README.md", "## Source Map");
assertContains("CONTRACT.md", "reallyme/codec");
assertContains("SECURITY.md", "reallyme-codec");
assertContains("SECURITY_MEMORY_MODEL.md", "reallyme-codec");
assertContains("buf.yaml", "modules:");
assertContains("buf.yaml", "- path: crates/proto/proto");
assertContains("crates/proto/proto/reallyme/codec/v1/codec.proto", "package reallyme.codec.v1;");
assertContains("crates/proto/proto/reallyme/codec/v1/codec.proto", "message CodecError");
assertContains("crates/proto/proto/reallyme/codec/v1/codec.proto", "message CodecOperationRequest");
assertContains("crates/proto/proto/reallyme/codec/v1/codec.proto", "reserved 1 to 999;");
assertContains(
  "crates/proto/proto/reallyme/codec/v1/codec.proto",
  "CodecMulticodecPrefixForNameRequest multicodec_prefix_for_name = 1000;",
);
assertContains(
  "crates/proto/proto/reallyme/codec/v1/codec.proto",
  "CodecMultikeyParseRequest multikey_parse = 2000;",
);
assertContains(
  "crates/proto/proto/reallyme/codec/v1/codec.proto",
  "CodecDagCborVerifyCidRequest dag_cbor_verify_cid = 3000;",
);
assertContains(
  "crates/proto/proto/reallyme/codec/v1/codec.proto",
  "CodecPemDecodeRequest pem_decode = 4000;",
);
assertContains("crates/proto/proto/reallyme/codec/v1/codec.proto", "message CodecBaseEncodingError");
assertContains("crates/proto/proto/reallyme/codec/v1/codec.proto", "message CodecPemError");
assertContains("crates/proto/proto/reallyme/codec/v1/codec.proto", "message CodecMultiformatError");
assertContains("crates/proto/proto/reallyme/codec/v1/codec.proto", "message CodecCanonicalizationError");
assertContains("crates/proto/proto/reallyme/codec/v1/codec.proto", "message CodecBackendError");
assertContains("crates/proto/proto/reallyme/codec/v1/codec.proto", "message CodecMulticodecSpec");
assertContains("crates/proto/proto/reallyme/codec/v1/codec.proto", "message CodecMultikeyParseResult");
assertContains("crates/proto/proto/reallyme/codec/v1/codec.proto", "message CodecPemDecodeResult");
assertContains("crates/proto/proto/reallyme/codec/v1/codec.proto", "message CodecDagCborVerifyCidResult");
assertContains("crates/proto/proto/reallyme/codec/v1/codec.proto", "enum CodecErrorReason");
assertContains(".github/workflows/protobuf-ci.yml", "--generated-freshness");
assertContains("crates/proto/Cargo.toml", '"buffa/json"');
assertContains("scripts/codec_proto_sensitivity.mjs", "codecProtoScalarFieldClassifications");
assertContains(
  "scripts/codec_proto_sensitivity.mjs",
  "codecProtoSensitiveNonTextFieldClassifications",
);
assertContains("scripts/codec_proto_sensitivity.mjs", "codecProtoSensitiveOwnerMessages");
assertContains("scripts/codec_proto_sensitivity.mjs", "codecProtoProviderOutputMessages");
for (const messageName of codecProtoProviderOutputMessages) {
  assertContains(
    `gen/java/me/really/codec/v1/${messageName}.java`,
    "public boolean reallyMeHasUnknownFieldsForValidation()",
  );
}
// Preserve the generator's Sendable conformance, including its checked COW
// representation, while enforcing internal visibility and redacted reflection.
const hardenedSwiftSource = readText("gen/swift/reallyme/codec/v1/codec.pb.swift");
for (const message of codecProtoSensitiveMessageNames) {
  const declaration = new RegExp(String.raw`nonisolated struct ReallyMeProto${message}: (?:@unchecked )?Sendable, CustomReflectable \{`, "u");
  if (!declaration.test(hardenedSwiftSource)) {
    fail(`generated Swift ${message} is missing its internal Sendable redacted facade`);
  }
}

const { assertNativeProtoPolicy } = await import("./release-readiness/native-proto-policy.mjs");
assertNativeProtoPolicy(readinessContext, {
  sensitiveMessages: codecProtoSensitiveMessageNames,
  generatedFreshnessMode,
});

assertContains(
  "crates/proto/tests/generated_tests.rs",
  "pem_decode_result_debug_redacts_der",
);
assertContains(
  "crates/proto/tests/generated_tests.rs",
  "generated_proto_json_rejects_unknown_fields",
);
assertContains(
  "crates/proto/tests/generated_tests.rs",
  "sensitive_bytes_proto_json_is_canonical_for_owned_and_borrowed_messages",
);
assertContains(
  "crates/proto/tests/generated_tests/operation_wire.rs",
  "operation_request_wire_tags_use_sparse_family_bands",
);
assertContains(
  "crates/proto/tests/generated_tests/operation_wire.rs",
  "operation_result_wire_tags_use_sparse_family_bands",
);
assertContains(
  "crates/proto/tests/generated_tests/operation_wire.rs",
  "operation_response_outcome_wire_tags_are_stable",
);

const { assertSdkSourcePolicies } = await import("./release-readiness/sdk-source-policy.mjs");
assertSdkSourcePolicies(readinessContext);

console.log("codec release readiness checks passed");
