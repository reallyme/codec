#!/usr/bin/env node
// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { execFileSync } from "node:child_process";

import { createReleaseReadinessContext } from "./release-readiness/core.mjs";
import {
  codecProtoProviderOutputMessages,
  codecProtoScalarFieldClassifications,
  codecProtoSensitiveNonTextFieldClassifications,
  codecProtoSensitiveOwnerMessages,
} from "./codec_proto_sensitivity.mjs";

const codecProtoSensitiveMessageNames = [...new Set([
  ...codecProtoScalarFieldClassifications
    .filter((classification) => classification.sensitivity === "sensitive")
    .map((classification) => classification.message),
  ...codecProtoSensitiveNonTextFieldClassifications
    .map((classification) => classification.message),
  ...codecProtoSensitiveOwnerMessages,
])];
const codecProtoDirectSensitiveMessageNames = [...new Set(
  [
    ...codecProtoScalarFieldClassifications,
    ...codecProtoSensitiveNonTextFieldClassifications,
  ]
    .filter((classification) => classification.sensitivity === "sensitive")
    .map((classification) => classification.message),
)];
const codecProtoDropRequiredMessageNames = [...new Set([
  ...codecProtoDirectSensitiveMessageNames,
  ...codecProtoSensitiveOwnerMessages,
])];

const {
  readText,
  readJson,
  listFiles,
  fail,
  requireTracked,
  assertContains,
  assertNotContains,
  assertMinOccurrences,
  assertNodeWorkflowJobsPinNode,
  assertProtoContract,
  assertReallyMeOperationBoundaryContract,
  assertReallyMeProtobufReleasePolicy,
  assertReallyMeVendoredCorePolicy,
  assertWorkflowActionsPinned,
  assertCargoFuzzWorkflowPolicy,
  assertWorkflowPermissionsPolicy,
  assertWorkflowRunStep,
} = createReleaseReadinessContext({
  scriptUrl: import.meta.url,
  requireTrackedFiles: true,
});

// This policy module executes inside the release gate. Requiring it explicitly
// prevents an untracked worktree file from influencing release readiness.
requireTracked("scripts/release-readiness/source-policy.mjs");
try {
  execFileSync(process.execPath, ["scripts/validate_codec_vectors.mjs"], {
    stdio: "pipe",
  });
} catch {
  fail("scripts/validate_codec_vectors.mjs failed");
}
const {
  sourceBlockFromNeedle,
  stripSourceComments,
  stripSourceStringsAndComments,
} = await import("./release-readiness/source-policy.mjs");

const supportedArguments = new Set([
  "--generated-freshness",
  "--release-packages",
]);
const suppliedArguments = new Set();
for (const argument of process.argv.slice(2)) {
  if (!supportedArguments.has(argument)) {
    fail(`unsupported argument ${argument}`);
  }
  if (suppliedArguments.has(argument)) {
    fail(`argument ${argument} was specified more than once`);
  }
  suppliedArguments.add(argument);
}

const rustProductionSource = (path) => {
  const source = readText(path);
  const testStart = source.indexOf("\n#[cfg(test)]");
  return testStart === -1 ? source : source.slice(0, testStart);
};

const scrubProtoCommentsAndStrings = (source) => {
  let output = "";
  let index = 0;
  while (index < source.length) {
    const char = source[index];
    const next = source[index + 1] ?? "";
    if (char === "/" && next === "/") {
      while (index < source.length && source[index] !== "\n") {
        output += " ";
        index += 1;
      }
      continue;
    }
    if (char === "/" && next === "*") {
      output += "  ";
      index += 2;
      while (index < source.length) {
        if (source[index] === "*" && source[index + 1] === "/") {
          output += "  ";
          index += 2;
          break;
        }
        output += source[index] === "\n" ? "\n" : " ";
        index += 1;
      }
      continue;
    }
    if (char === "\"" || char === "'") {
      const quote = char;
      output += " ";
      index += 1;
      while (index < source.length) {
        const current = source[index];
        output += current === "\n" ? "\n" : " ";
        index += current === "\\" ? 2 : 1;
        if (current === quote) {
          break;
        }
      }
      continue;
    }
    output += char;
    index += 1;
  }
  return output;
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
  const typeBlock = facadeSource.match(/export type\s*\{([\s\S]*?)\}\s*from/u);
  if (typeBlock === null) {
    fail(`${facadePath} is missing its generated type export block`);
  }

  assertSetEquals({
    label: `${facadePath} generated schema exports`,
    actual: sortedUniqueMatches(facadeSource, /\b(Codec[A-Za-z0-9_]+Schema)\b/gu),
    expected: sortedUniqueMatches(generatedSource, /export const (Codec[A-Za-z0-9_]+Schema)\b/gu),
  });
  assertSetEquals({
    label: `${facadePath} generated type exports`,
    actual: sortedUniqueMatches(typeBlock[1], /\b(Codec[A-Za-z0-9_]+)\b/gu),
    expected: sortedUniqueMatches(generatedSource, /export type (Codec[A-Za-z0-9_]+)\b/gu),
  });
};

assertReallyMeVendoredCorePolicy();
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

const codecPackageVersion = "0.2.0";
const codecProtoPackageVersion = "0.2.0";
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

assertNodeWorkflowJobsPinNode({ nodeVersion: "24" });

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
assertContains("Cargo.toml", 'buffa = { version = "0.9.0", features = ["json"] }');
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
assertContains("crates/ffi/Cargo.toml", "publish = false");
assertContains("crates/ffi/src/codec.rs", "rm_codec_process_operation");
assertContains("crates/ffi/src/codec.rs", "rm_codec_process_operation_json");
assertContains("crates/ffi/src/codec.rs", "rm_codec_abi_version");
assertContains("crates/ffi/src/codec.rs", "rm_codec_max_operation_response_bytes");
assertContains("crates/ffi/src/codec.rs", "rm_codec_max_ffi_input_bytes");
assertContains("crates/ffi/src/codec.rs", "rm_codec_max_ffi_output_bytes");
assertContains("crates/ffi/src/codec.rs", "CODEC_ABI_VERSION");
assertContains("crates/ffi/src/codec.rs", "CODEC_ABI_VERSION: u32 = 5");
assertContains("crates/codec/src/operation_contract/mod.rs", "include!(\"dispatch.rs\")");
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
  path: "packages/ts/src/cbor.ts",
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
  path: "packages/ts/src/cbor.ts",
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
assertContains("packages/ts/src/cbor.ts", 'Readonly<{ type: "bytes"; value: Uint8Array }>');
assertContains("packages/ts/src/cbor.ts", 'return { type: "bytes", value: value.value.slice() }');
assertContains("packages/ts/src/cbor.ts", "const entries: CodecDeterministicCborMapEntry[] = []");
assertContains("packages/ts/src/cbor.ts", "const values: ReallyMeCborValue[] = []");
assertContains("packages/ts/src/cbor.ts", "const bytes = value.value.slice()");
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
  assertNotContains("packages/ts/src/cbor.ts", removedTsDagCborJsonNeedle);
}
assertContains("packages/swift/Sources/ReallyMeCodec/DeterministicCbor.swift", ".dagCborEncode(request)");
assertContains("packages/swift/Sources/ReallyMeCodec/DeterministicCbor.swift", ".dagCborDecode(request)");
assertContains("packages/swift/Sources/ReallyMeCodec/DeterministicCbor.swift", "processGeneratedOperation(request: request)");
assertContains("packages/swift/Sources/ReallyMeCodec/DeterministicCbor.swift", "case .dagCborEncode(var result)?");
assertContains("packages/swift/Sources/ReallyMeCodec/DeterministicCbor.swift", "case .dagCborDecode(var result)?");
assertNotContains("packages/swift/Sources/ReallyMeCodec/DeterministicCbor.swift", "deterministicCborPayload(request:");
assertNotContains("packages/swift/Sources/ReallyMeCodec/StructuredResults.swift", "serializedBytes: resultBytes");
assertNotContains("packages/swift/Sources/ReallyMeCodec/StructuredResults.swift", "from resultBytes");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", ".setDagCborEncode(");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", ".setDagCborDecode(");
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
assertContains("crates/ffi/src/codec.rs", "CODEC_ABI_VERSION: u32 = 5");
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
assertContains("crates/codec/src/lib.rs", "canonicalize_json_text");
assertContains("crates/codec/src/lib.rs", "canonicalize_trusted_json_value");
assertNotContains("crates/codec/src/lib.rs", "canonicalize_json, JcsError");
assertNotContains("crates/jcs/src/lib.rs", "canonicalize_json;");
assertNotContains("crates/jcs/src/canonicalize.rs", "pub fn canonicalize_json(");
assertNotContains("crates/jcs/tests/jcs_tests.rs", "deprecated_value_alias");
assertContains("crates/jcs/src/canonicalize.rs", "non-integer binary64 numbers follow RFC 8785");
assertContains("crates/jcs/src/canonicalize.rs", "validate_interoperable_float_integer");
assertContains("crates/jcs/src/canonicalize.rs", "MIN_INTEROPERABLE_INTEGER_F64");
assertContains("crates/jcs/src/lib.rs", "integer-valued");
assertContains(
  "crates/jcs/tests/jcs_tests.rs",
  "integer_valued_binary64_numbers_outside_interoperable_range_are_rejected",
);
assertContains("crates/jcs/tests/jcs_tests.rs", '"1e19"');
assertContains("crates/jcs/tests/jcs_tests.rs", '"9007199254740992.0"');
assertContains("crates/ffi/src/codec.rs", "validate_boundary_input_lengths");
assertContains(
  "crates/ffi/src/codec.rs",
  "validate_proto_boundary_input_length(",
);
assertContains(
  "crates/ffi/src/kotlin_codec.rs",
  "validate_managed_input_lengths",
);
assertContains("crates/ffi/src/kotlin_codec.rs", "aggregate.checked_add(length)");
assertContains("crates/wasm/Cargo.toml", 'name = "reallyme-codec-wasm"');
assertContains("crates/wasm/src/boundary.rs", "MAX_WASM_INPUT_BYTES");
assertContains("crates/wasm/src/boundary.rs", "checked_add");
assertContains("crates/wasm/src/boundary.rs", "validate_js_inputs");
assertContains("crates/wasm/src/boundary.rs", "utf8_byte_len_for_js_string");
assertContains("crates/wasm/src/boundary.rs", "value.iter().peekable()");
assertNotContains("crates/wasm/src/boundary.rs", "MAX_WASM_STRING_CODE_UNITS");
assertContains("crates/wasm/src/boundary.rs", "zeroizing_string");
assertContains("crates/wasm/src/boundary.rs", "zeroizing_bytes_with_maximum");
assertContains("crates/wasm/src/boundary.rs", "value.subarray");
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
  "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt",
  "private fun processOperation(request: CodecOperationRequest): CodecOperationResult",
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
  "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt",
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
assertContains(".github/workflows/code-checks.yml", "cargo-deny@0.19.6");
assertContains(
  ".github/workflows/code-checks.yml",
  "node --test scripts/release-readiness/cli.test.mjs",
);
assertContains(
  ".github/workflows/code-checks.yml",
  "node --test scripts/release-readiness/source-policy.test.mjs",
);
assertContains(
  ".github/workflows/code-checks.yml",
  "node --test scripts/write_native_manifest.test.mjs",
);
assertContains(
  ".github/workflows/code-checks.yml",
  "node --test scripts/verify_release_attestation.test.mjs",
);
assertContains(
  ".github/workflows/code-checks.yml",
  "node --test scripts/verify_swift_release_artifact.test.mjs",
);
assertContains(
  ".github/workflows/code-checks.yml",
  "node scripts/run_pinned_release_readiness.mjs",
);
assertContains(
  "scripts/run_pinned_release_readiness.mjs",
  'const RELEASE_READINESS_COMMIT = "f27973caf9d3a12847cac4032c361f5f553c97e9"',
);
assertContains(
  "scripts/run_pinned_release_readiness.mjs",
  '"70cc78721738cf352024938e8fc86e73380e71b2cdf7a9a733687543167cbaae"',
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
assertContains(
  "scripts/run_pinned_release_readiness.mjs",
  "vendored core bytes do not match the pinned upstream core",
);
assertContains(
  "scripts/run_pinned_release_readiness.mjs",
  "spawnSync(process.execPath, [LOCAL_CHECKER_PATH",
);
assertContains(
  "scripts/release-readiness/core.mjs",
  "job ${job.name} permissions must be a flat explicit mapping",
);
assertContains(
  "scripts/release-readiness/core.mjs",
  "extractWorkflowRunCommands",
);
assertContains(
  "scripts/release-readiness/core.mjs",
  "cargo-fuzz installation must be in a named workflow step",
);
assertContains(
  "scripts/release-readiness/source-policy.test.mjs",
  "TypeScript block comments do not nest or hide following code",
);
assertContains(
  "scripts/release-readiness/source-policy.test.mjs",
  "regex literals with quotes do not blank following executable code",
);
assertContains(
  ".github/workflows/code-checks.yml",
  "gradle/actions/wrapper-validation@0723195856401067f7a2779048b490ace7a47d7c",
);
assertContains(".github/workflows/code-checks.yml", "node-version: '24'");
assertMinOccurrences(".github/workflows/crates-release.yml", "node-version: '24'", 2);

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
assertReallyMeOperationBoundaryContract({
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
        '@_silgen_name("rm_codec_max_ffi_input_bytes")',
        '@_silgen_name("rm_codec_max_ffi_output_bytes")',
        '@_silgen_name("rm_codec_max_operation_response_bytes")',
        "private func rmCodecMaxFfiInputBytesLinked() -> UInt",
        "private func rmCodecMaxFfiOutputBytesLinked() -> UInt",
        "private func rmCodecMaxOperationResponseBytesLinked() -> UInt",
        "private let maxFfiInputLength: Int",
        "private let maxFfiOutputLength: Int",
        "private let maxOperationResponseLength: Int",
        "rmCodecMaxFfiInputBytesLinked()",
        "rmCodecMaxFfiOutputBytesLinked()",
        "rmCodecMaxOperationResponseBytesLinked()",
        '"rm_codec_max_ffi_input_bytes"',
        '"rm_codec_max_ffi_output_bytes"',
        '"rm_codec_max_operation_response_bytes"',
        "static func requireValidOperationResponseLimit(_ limit: UInt, maxFfiOutputLength: Int) throws -> Int",
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
    },
  ],
});
assertContains(
  "packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift",
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
    protoVariant: "CodecOperation::MulticodecTable(request) => {",
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
    tsPath: "packages/ts/src/cbor.ts",
    tsExport:
      "export const dagCborVerifyCid = (\n  cid: string,\n  bytes: Uint8Array,",
    tsNeedles: [
      "processGeneratedOperationRequest(",
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
      "processGeneratedOperationRequest(request)",
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
    tsPath: "packages/ts/src/cbor.ts",
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
    tsPath: "packages/ts/src/cbor.ts",
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
  assertNotContains("crates/codec/src/lib.rs", `pub mod ${operation.moduleName};`);
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

for (const stage13RegressionTest of [
  "retired_scalar_structured_json_ids_fail_closed",
]) {
  assertContains("crates/ffi/src/codec/tests.rs", stage13RegressionTest);
}

for (const stage14OwnershipNeedle of [
  "verification.into_parts()",
  "decoded.into_der()",
  "try_copy_result_bytes",
  "try_copy_result_string",
]) {
  assertContains(codecContractShapePath, stage14OwnershipNeedle);
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
  functionNeedle: "pub fn dag_cbor_verify_cid_result_proto(",
  requiredNeedles: ["verification.into_parts()", "expected_cid", "actual_cid"],
  forbiddenNeedles: ["to_owned()", "to_string()", "clone()"],
});
assertRustFunction({
  path: codecContractShapePath,
  functionNeedle: "pub fn pem_decode_result_proto(",
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
  assertBlockContains({
    path: codecContractDispatchPath,
    block: variantBlock,
    needle: "reject_unknown_fields(&request.__buffa_unknown_fields)?",
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
assertContains("packages/ts/src/operationContract.ts", "const errorCodeForCodecErrorMessage =");
for (const reasonRange of [
  "reason < 100 || reason > 199",
  "reason < 200 || reason > 299",
  "reason < 300 || reason > 399",
  "reason < 400 || reason > 499",
]) {
  assertContains("packages/ts/src/operationContract.ts", reasonRange);
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
  "return expectedOrigin == .caller ? .invalidInput : .providerFailure",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt",
  "private fun exceptionForCodecError(codecError: CodecError)",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt",
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

for (const limit of [
  {
    rust: "pub const MAX_DETERMINISTIC_CBOR_INPUT_LEN: usize = 1024 * 1024;",
    typescript: "export const MAX_DETERMINISTIC_CBOR_INPUT_LEN = 1_048_576;",
  },
  {
    rust: "pub const MAX_DETERMINISTIC_CBOR_OUTPUT_LEN: usize = 1024 * 1024;",
    typescript: "export const MAX_DETERMINISTIC_CBOR_OUTPUT_LEN = 1_048_576;",
  },
  {
    rust: "pub const MAX_DETERMINISTIC_CBOR_NESTING_DEPTH: usize = 64;",
    typescript: "export const MAX_DETERMINISTIC_CBOR_NESTING_DEPTH = 64;",
    kotlin: "private const val MAX_DETERMINISTIC_CBOR_NESTING_DEPTH: Int = 64",
    swift: "private let maxDeterministicCborNestingDepth = 64",
  },
  {
    rust: "pub const MAX_DETERMINISTIC_CBOR_NODES: usize = 65_536;",
    typescript: "export const MAX_DETERMINISTIC_CBOR_NODES = 65_536;",
    kotlin: "private const val MAX_DETERMINISTIC_CBOR_NODES: Int = 65_536",
    swift: "private let maxDeterministicCborNodes = 65_536",
  },
  {
    rust: "pub const MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES: usize = 16_384;",
    typescript: "export const MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES = 16_384;",
    kotlin: "private const val MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES: Int = 16_384",
    swift: "private let maxDeterministicCborContainerEntries = 16_384",
  },
  {
    rust: "pub const MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES: usize = 1024 * 1024;",
    typescript: "export const MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES = 1_048_576;",
    kotlin:
      "private const val MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES: Int = 1_048_576",
    swift: "private let maxDeterministicCborAggregateTextBytes = 1_048_576",
  },
  {
    rust:
      "pub const MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES: usize = 1024 * 1024;",
    typescript:
      "export const MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES = 1_048_576;",
    kotlin:
      "private const val MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES: Int = 1_048_576",
    swift:
      "private let maxDeterministicCborAggregateByteStringBytes = 1_048_576",
  },
]) {
  assertContains("crates/cbor/src/deterministic/limits.rs", limit.rust);
  assertContains("packages/ts/src/deterministicCborBoundary.ts", limit.typescript);
  if (limit.kotlin !== undefined) {
    assertContains(
      "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt",
      limit.kotlin,
    );
  }
  if (limit.swift !== undefined) {
    assertContains(
      "packages/swift/Sources/ReallyMeCodec/DeterministicCbor.swift",
      limit.swift,
    );
  }
}
assertContains(
  "packages/ts/src/deterministicCborBoundary.ts",
  "MAX_DETERMINISTIC_CBOR_NESTING_DEPTH * 3 + 5",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt",
  "(MAX_DETERMINISTIC_CBOR_NESTING_DEPTH * 3) + 5",
);
assertContains(
  "packages/swift/Sources/ReallyMeCodec/DeterministicCbor.swift",
  "(maxDeterministicCborNestingDepth * 3) + 7",
);
for (const swiftTransportNeedle of [
  "maxCodecProtoStructuralBytesPerDeterministicCborNode = 128",
  "maxCodecProtoFixedDeterministicCborOperationBytes = 4_096",
  "maxDeterministicCborProtoMessageBytes =",
  "serialized.count <= maxDeterministicCborProtoMessageBytes",
]) {
  assertContains(
    "packages/swift/Sources/ReallyMeCodec/DeterministicCbor.swift",
    swiftTransportNeedle,
  );
}
assertContains(
  "packages/swift/Sources/ReallyMeCodec/DeterministicCbor.swift",
  "boundedDeterministicCborUtf8Length(",
);
assertNotContains(
  "packages/swift/Sources/ReallyMeCodec/DeterministicCbor.swift",
  ".utf8.count",
);
assertContains(
  "packages/swift/Sources/ReallyMeCodec/DeterministicCbor.swift",
  "rejectDuplicateProviderDeterministicCborMapKeys(",
);
assertContains(
  "packages/swift/Sources/ReallyMeCodec/DeterministicCbor.swift",
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
  "func base64urlEncode(_ data: Data) throws -> String",
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
  "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt",
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
assertContains("packages/ts/src/cbor.ts", "export const ReallyMeDeterministicCbor =");
assertContains("packages/ts/src/cbor.ts", "export const ReallyMeDagCbor =");
assertContains("packages/ts/src/cbor.ts", "bytes(value: Uint8Array)");
assertContains("packages/ts/src/cbor.ts", "negative(value: number | bigint)");
assertContains("packages/ts/src/cbor.ts", "text(value: string)");
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
assertContains("crates/codec/src/operation_contract/mod.rs", 'include!("copy_limits.rs")');

assertContains("buf.gen.yaml", "out: crates/proto/src/generated/buffa");
assertContains("buf.gen.yaml", "out: packages/ts/src/proto/generated");
assertContains("buf.gen.yaml", "buf.build/bufbuild/es:v2.12.1");
assertContains("buf.gen.yaml", "buf.build/apple/swift:v1.38.1");
assertContains("buf.gen.yaml", "buf.build/protocolbuffers/java:v35.1");
assertContains("buf.gen.yaml", "buf.build/protocolbuffers/kotlin:v35.1");
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
  "max_codec_proto_message_bytes_const()",
  "max_codec_proto_json_bytes_const()",
]) {
  assertContains("crates/proto/src/limits.rs", transportDerivationNeedle);
}
for (const typescriptTransportNeedle of [
  "MAX_CODEC_PROTO_STRUCTURAL_BYTES_PER_NODE",
  "MAX_CODEC_PROTO_FIXED_OPERATION_BYTES",
  "MAX_CODEC_PROTO_MESSAGE_BYTES =",
  "MAX_CODEC_PROTO_JSON_BYTES =",
]) {
  assertContains("packages/ts/src/boundary.ts", typescriptTransportNeedle);
}
for (const kotlinTransportNeedle of [
  "MAX_CODEC_PROTO_STRUCTURAL_BYTES_PER_NODE",
  "MAX_CODEC_PROTO_FIXED_OPERATION_BYTES",
  "MAX_CODEC_PROTO_MESSAGE_BYTES: Int =",
]) {
  assertContains(
    "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt",
    kotlinTransportNeedle,
  );
}
assertContains(
  "crates/ffi/src/kotlin_codec.rs",
  "max_request_len.checked_add(1)",
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
  "pub fn encode_protobuf<M: Message>(message: &M) -> Zeroizing<Vec<u8>>",
);
assertContains(
  "crates/proto/src/wire.rs",
  "pub fn encode_protobuf<M: Message>(message: &M) -> Zeroizing<Vec<u8>>",
);
assertContains("crates/proto/tests/generated_tests/error_wire.rs", "bounded_protobuf_decode_rejects_oversized_messages");
assertContains("crates/proto/tests/generated_tests/error_wire.rs", "json_decode_rejects_inputs_that_expand_past_binary_cap");
assertContains(".github/workflows/protobuf-ci.yml", "BUFFA_VERSION: 0.9.0");
assertContains(".github/workflows/protobuf-ci.yml", "BUF_VERSION: 1.71.0");
assertContains(".github/workflows/protobuf-ci.yml", "scripts/release-readiness/core.mjs");
assertContains(".github/workflows/protobuf-ci.yml", "scripts/release-readiness/source-policy.mjs");
assertContains(".github/workflows/protobuf-ci.yml", "scripts/run_pinned_release_readiness.mjs");
assertContains(".github/workflows/protobuf-ci.yml", "scripts/codec_proto_sensitivity.mjs");
assertContains(".github/workflows/protobuf-ci.yml", "node-version: '24'");
assertContains(".github/workflows/protobuf-ci.yml", "cargo install protoc-gen-buffa-packaging");
assertContains("scripts/check_release_readiness.mjs", '["buf", ["lint"]]');
assertContains(".github/workflows/protobuf-ci.yml", "buf breaking --against '.git#branch=origin/main'");
assertContains(
  "crates/proto/proto/reallyme/codec/v1/codec.proto",
  "Protobuf map<> is intentionally not used",
);
assertContains(
  "crates/proto/proto/reallyme/codec/v1/codec.proto",
  "repeated CodecDeterministicCborMapEntry entries = 1;",
);
assertContains(
  ".github/workflows/protobuf-ci.yml",
  "node scripts/run_pinned_release_readiness.mjs --generated-freshness",
);
assertContains(
  ".github/workflows/protobuf-ci.yml",
  "bufbuild/buf-setup-action@a47c93e0b1648d5651a065437926377d060baa99",
);
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
assertContains("packages/ts/package.json", '"@bufbuild/protobuf": "2.12.1"');
assertContains("packages/ts/package.json", '"fast-check": "3.23.2"');
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
assertContains("packages/kotlin/build.gradle.kts", 'artifactId = "codec"');
assertContains("packages/kotlin/build.gradle.kts", "Java, Kotlin, JVM, and Android");
assertContains("packages/kotlin/build.gradle.kts", "com.google.protobuf:protobuf-javalite:4.35.1");
assertContains("packages/kotlin/build.gradle.kts", "com.google.protobuf:protobuf-kotlin-lite:4.35.1");
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
  "distributionSha256Sum=9c0f7faeeb306cb14e4279a3e084ca6b596894089a0638e68a07c945a32c9e14",
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
assertContains("docs/dependency-updates.md", "verifying every trusted full fingerprint");
assertContains("scripts/write_native_manifest.mjs", "sha256");
assertContains("scripts/write_native_manifest.mjs", 'writeFileSync(`${path}.sha256`');
assertContains("scripts/write_native_manifest.mjs", "GITHUB_SHA does not match the checked-out source SHA");
assertContains("packages/kotlin-android/build.gradle.kts", "fun checkedOutCommitSha()");
assertContains("packages/kotlin-android/build.gradle.kts", "GITHUB_SHA does not match the checked-out source SHA");
const workflowFiles = listFiles(".github/workflows");
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
  assertContains(workflowPath, "default: 0.2.0");
}
for (const workflowPath of packageReleaseWorkflows) {
  assertContains(workflowPath, "Verify reviewed release SHA");
  assertContains(workflowPath, "Resolve current release SHA");
  assertNotContains(workflowPath, "RELEASE_SHA_INPUT");
  assertNotContains(workflowPath, "inputs.publish");
}
assertContains(".github/workflows/kotlin-android-package-release.yml", "Write native checksum manifest");
assertContains(".github/workflows/kotlin-android-package-preflight.yml", "Write native checksum manifest");
assertContains(".github/workflows/kotlin-android-package-release.yml", "Test host native loader");
assertContains(".github/workflows/kotlin-android-package-preflight.yml", "Test host native loader");
assertContains("scripts/maven_central_bundle_local.sh", "kotlin-android-package-preflight.yml");
assertContains("scripts/maven_central_bundle_local.sh", '-f "version=${VERSION}"');
assertWorkflowPermissionsPolicy({
  path: ".github/workflows/swift-package-release.yml",
  workflow: { contents: "read" },
  jobs: {
    "verify-release-sha": { actions: "read", contents: "read" },
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
    "npm-package": { actions: "read", contents: "read", "id-token": "write" },
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
assertNotContains(".github/workflows/swift-package-release.yml", "publish_maven:");
assertNotContains(".github/workflows/swift-package-release.yml", "publish_npm:");
assertNotContains(".github/workflows/kotlin-android-package-release.yml", "publish_swift:");
assertNotContains(".github/workflows/kotlin-android-package-release.yml", "publish_npm:");
assertNotContains(".github/workflows/npm-package-release.yml", "publish_swift:");
assertNotContains(".github/workflows/npm-package-release.yml", "publish_maven:");
assertContains(".github/workflows/crates-release.yml", "Verify reviewed release SHA");
assertContains(".github/workflows/crates-release.yml", "Resolve current release SHA");
assertContains(".github/workflows/crates-release.yml", "release_version:");
assertContains(".github/workflows/crates-release.yml", "crates/codec/Cargo.toml");
assertContains(".github/workflows/crates-release.yml", "RELEASE_VERSION=${release_version}");
assertContains(".github/workflows/crates-release.yml", "steps.resolve-release-sha.outputs.release_version");
assertContains(".github/workflows/crates-release.yml", "needs.verify-release-sha.outputs.release_version");
assertNotContains(".github/workflows/crates-release.yml", "RELEASE_VERSION: ${{ inputs.version }}");
assertContains(".github/workflows/kotlin-android-package-preflight.yml", "needs: [verify-source-sha, jvm-native]");
assertContains(".github/workflows/kotlin-android-package-release.yml", "needs: [verify-release-sha, jvm-native]");
assertContains(".github/workflows/swift-package-release.yml", "needs: [verify-release-sha, swift-artifact]");
assertContains(".github/workflows/swift-package-release.yml", "needs: [verify-release-sha, swift-artifact, swift-verify]");
assertContains(
  ".github/workflows/swift-package-release.yml",
  `swift-verify:
    name: SwiftPM artifact verification
    needs: [verify-release-sha, swift-artifact]
    runs-on: macos-26`,
);
assertContains(".github/workflows/crates-release.yml", "needs: [verify-release-sha, dry-run]");
assertWorkflowRunStep(
  ".github/workflows/swift-package-release.yml",
  "Require current main and successful Swift package checks",
  "node scripts/verify_release_attestation.mjs",
);
assertWorkflowRunStep(
  ".github/workflows/kotlin-android-package-release.yml",
  "Require current main and successful Kotlin Android package checks",
  "node scripts/verify_release_attestation.mjs",
);
assertWorkflowRunStep(
  ".github/workflows/npm-package-release.yml",
  "Require current main and successful npm package checks",
  "node scripts/verify_release_attestation.mjs",
);
assertWorkflowRunStep(
  ".github/workflows/crates-release.yml",
  "Require current main and successful checks for exact SHA",
  "node scripts/verify_release_attestation.mjs",
);
assertWorkflowRunStep(
  ".github/workflows/kotlin-android-package-release.yml",
  "Publish Maven artifact",
  `node ../../scripts/verify_release_attestation.mjs
./gradlew publish -Preallyme.codec.nativeResourcesDir=\${{ github.workspace }}/build/kotlin-native-resources -Preallyme.codec.requireFullNativeResources=true`,
);
assertWorkflowRunStep(
  ".github/workflows/kotlin-android-package-release.yml",
  "Publish Android AAR",
  `node scripts/verify_release_attestation.mjs
packages/kotlin/gradlew -p packages/kotlin-android publish -Preallyme.codec.androidJniLibsDir=\${{ github.workspace }}/build/android-jniLibs -Preallyme.codec.androidNativeAssetsDir=\${{ github.workspace }}/build/android-native-assets -Preallyme.codec.requireAndroidJniLibs=true`,
);
assertWorkflowRunStep(
  ".github/workflows/npm-package-release.yml",
  "Publish npm package",
  `if [ -z "\${NODE_AUTH_TOKEN}" ]; then
  echo "::error::NPM_TOKEN is required"
  exit 1
fi
node ../../scripts/verify_release_attestation.mjs
npm publish --provenance --access public`,
);
assertContains(".github/workflows/npm-package-release.yml", "registry-url: 'https://registry.npmjs.org'");
assertContains(".github/workflows/npm-package-release.yml", "NODE_AUTH_TOKEN: ${{ secrets.NPM_TOKEN }}");
assertContains(".github/workflows/npm-package-release.yml", "wasm-pack@0.15.0");
assertContains(".github/workflows/npm-package-release.yml", "wasm-bindgen-cli@0.2.126");
assertNotContains(".github/workflows/kotlin-android-package-release.yml", "steps.maven_remote.outputs.configured == 'true'");
assertNotContains(".github/workflows/kotlin-android-package-release.yml", "configured=false");
assertNotContains(
  ".github/workflows/kotlin-android-package-release.yml",
  "remote Maven credentials are incomplete; packaged artifacts were verified locally and remote publish is skipped",
);
assertNotContains("scripts/publish_crates_in_order.mjs", "already published; continuing");
assertContains(
  "scripts/publish_crates_in_order.mjs",
  "refusing to treat a prior upload as this release's attested publish",
);
assertWorkflowRunStep(
  ".github/workflows/crates-release.yml",
  "Publish crates in dependency order",
  `node scripts/verify_release_attestation.mjs
node scripts/publish_crates_in_order.mjs publish`,
);
assertWorkflowRunStep(
  ".github/workflows/swift-package-release.yml",
  "Verify SwiftPM manifest",
  `node scripts/verify_swift_release_artifact.mjs build/swift/ReallyMeCodecFFI.xcframework.zip build/swift/ReallyMeCodecFFI.xcframework.checksum Package.swift "\${RELEASE_VERSION}"
node scripts/run_pinned_release_readiness.mjs --release-packages`,
);
assertWorkflowRunStep(
  ".github/workflows/swift-package-release.yml",
  "Select Xcode 26.4 for Swift artifact",
  `sudo xcode-select -s /Applications/Xcode_26.4.app
swift --version`,
);
assertWorkflowRunStep(
  ".github/workflows/swift-package-release.yml",
  "Select Xcode 26.4 for Swift verification",
  `sudo xcode-select -s /Applications/Xcode_26.4.app
swift --version`,
);
assertContains(".github/workflows/swift-package-release.yml", "SwiftPM artifact verification");
assertWorkflowRunStep(
  ".github/workflows/swift-package-release.yml",
  "Create immutable GitHub release with Swift artifact",
  `node scripts/verify_release_attestation.mjs
if gh release view "v\${RELEASE_VERSION}" >/dev/null 2>&1; then
  echo "::error::GitHub release v\${RELEASE_VERSION} already exists"
  exit 1
fi
if git ls-remote --exit-code --tags origin "refs/tags/v\${RELEASE_VERSION}" >/dev/null 2>&1; then
  echo "::error::Git tag v\${RELEASE_VERSION} already exists"
  exit 1
fi
gh release create "v\${RELEASE_VERSION}" build/swift/ReallyMeCodecFFI.xcframework.zip --target "\${RELEASE_SHA}" --title "ReallyMe Codec v\${RELEASE_VERSION}" --notes "ReallyMe Codec package release v\${RELEASE_VERSION}."`,
);
assertMinOccurrences(".github/workflows/swift-package-release.yml", "node-version: '24'", 3);
assertMinOccurrences(".github/workflows/kotlin-android-package-release.yml", "node-version: '24'", 3);
assertMinOccurrences(".github/workflows/npm-package-release.yml", "node-version: '24'", 2);
assertMinOccurrences(".github/workflows/kotlin-android-package-preflight.yml", "node-version: '24'", 3);
assertMinOccurrences(".github/workflows/npm-package-preflight.yml", "node-version: '24'", 1);
for (const workflowPath of [
  ".github/workflows/fuzz.yml",
  ...packagePreflightWorkflows,
  ...packageReleaseWorkflows,
]) {
  assertNotContains(workflowPath, "actions/upload-artifact@330a01c490aca151604b8cf639adc76d48f6c5d4");
  assertNotContains(workflowPath, "actions/download-artifact@634f93cb2916e3fdff6788551b99b062d0335ce0");
}
assertContains(
  ".github/workflows/kotlin-android-package-release.yml",
  "actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a # v7.0.1",
);
assertMinOccurrences(".github/workflows/fuzz.yml", "toolchain: nightly-2026-07-01", 2);
assertNotContains(".github/workflows/fuzz.yml", "cargo +nightly fuzz");
assertContains(".github/workflows/fuzz.yml", "crates/proto/**");
assertContains(".github/workflows/fuzz.yml", "- operation_contract");
assertContains("fuzz/Cargo.toml", 'name = "operation_contract"');
assertContains("fuzz/README.md", "`operation_contract`");
assertContains("fuzz/fuzz_targets/operation_contract.rs", "process_operation_response(data)");
assertContains("fuzz/fuzz_targets/operation_contract.rs", "process_operation_response_json(data)");
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
  "fuzz/fuzz_targets/deterministic_cbor.rs",
  "decode_deterministic_cbor(data)",
);
assertContains(
  "fuzz/fuzz_targets/deterministic_cbor.rs",
  "encode_deterministic_cbor(&value)",
);
assertContains(
  "fuzz/fuzz_targets/deterministic_cbor.rs",
  "decode_deterministic_cbor(&encoded)",
);
assertContains(
  "fuzz/fuzz_targets/deterministic_cbor.rs",
  "assert_eq!(encoded_again, encoded)",
);
assertContains("fuzz/Cargo.toml", 'name = "jcs_text"');
assertContains("fuzz/fuzz_targets/jcs_text.rs", "canonicalize_json_text(json)");
assertContains("fuzz/README.md", "`jcs_text`");
assertContains(".github/workflows/fuzz.yml", "- jcs_text");
assertContains("scripts/test_native_sanitizers.sh", "nightly-2026-07-01");
assertContains("scripts/test_native_sanitizers.sh", "-Zsanitizer=address");
assertContains("scripts/test_native_sanitizers.sh", "-Zub-checks=yes -Zextra-const-ub-checks=yes");
assertContains(".github/workflows/code-checks.yml", "Test native sanitizer lanes");
assertContains(
  ".github/workflows/kotlin-android-package-release.yml",
  "actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c # v8.0.1",
);
assertContains(".github/workflows/kotlin-android-package-release.yml", "requireFullNativeResources=true");
assertContains(".github/workflows/kotlin-android-package-preflight.yml", "requireFullNativeResources=true");
assertContains("packages/kotlin/settings.gradle.kts", 'rootProject.name = "reallyme-codec"');
assertContains("packages/kotlin/README.md", "me.really:codec:0.2.0");
assertContains("packages/kotlin/README.md", "ships Rust JNI libraries as platform resources");
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/RustNativeProvider.kt",
  "/me/really/codec/native",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/RustNativeProvider.kt",
  'ANDROID_LIBRARY_NAME: String = "reallyme_codec_ffi"',
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/RustNativeProvider.kt",
  "System.loadLibrary(ANDROID_LIBRARY_NAME)",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/RustNativeProvider.kt",
  "createPrivateExtractionDirectory",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/RustNativeProvider.kt",
  "MessageDigest.isEqual",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/RustNativeProvider.kt",
  "isSecurePosixTempMode",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/RustNativeProvider.kt",
  "isTrustedPosixTempOwner",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/RustNativeProvider.kt",
  "AclFileAttributeView",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/RustNativeProvider.kt",
  "restrictAclToOwner",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/RustNativeProvider.kt",
  "makeExtractedLibraryReadOnly",
);
assertNotContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/RustNativeProvider.kt",
  "File.createTempFile",
);
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "@JvmStatic");
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt",
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
assertNotContains("packages/kotlin/build.gradle.kts", 'name = "localRelease"');
assertNotContains(".github/workflows/kotlin-android-package-release.yml", "-Preallyme.maven.requireRemote=true");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "public fun tryParseCid(cid: String): String?");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "public fun dagCborCodecCode(): Int");
assertContains("crates/ffi/src/codec.rs", "MAX_CODEC_FFI_OUTPUT_BYTES");
assertContains("crates/ffi/src/kotlin_codec.rs", "probed_output_capacity");
assertContains("crates/ffi/src/kotlin_codec.rs", "produced_len != output.len()");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "CodingErrorAction.REPORT");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "bytes.fill(0)");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "Character.isSurrogate(character)");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "CODEC_ERROR_REASON_CANONICAL_INTERNAL");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "withTextBytes");
assertContains(
  "packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt",
  "nativeDigestMetadataAndTempPermissionsFailClosed",
);
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", 'ReallyMeCodec.canonicalizeJson("\\uD800")');
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", 'assertNull(codec.tryParseCid("not-a-cid"))');
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", "assertEquals(0x71, codec.dagCborCodecCode())");
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", 'codec.base58btcDecode("")');
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", "codec.base58btcEncode(oversizedBase58Input)");
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", 'codec.multicodecPrefixForName("not-a-codec")');
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", "codec.multicodecLookupPrefix(byteArrayOf(0, 0, 7))");
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", 'codec.dagCborVerifyCid("", encoded)');
assertContains("crates/multibase/src/base58btc.rs", "bytes.len() > MAX_BASE58BTC_INPUT_LEN");
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
assertContains("Package.swift", "// swift-tools-version: 6.3");
assertContains("Package.swift", 'name: "ReallyMeCodec"');
assertContains("Package.swift", 'name: "ReallyMeCodecProto"');
assertContains("Package.swift", 'name: "ReallyMeCodecFFI"');
assertContains("Package.swift", 'from: "1.38.1"');
assertContains("Package.swift", "ReallyMeCodecFFI.xcframework.zip");
assertContains("Package.swift", 'let ffiArtifactLocalPathOverride = "');
assertContains("Package.swift", "path: ffiArtifactLocalPathOverride");
assertNotContains("Package.swift", "FileManager.default.fileExists");
assertContains("Package.swift", "REALLYME_CODEC_SWIFTPM_RUNTIME_FFI");
assertContains("packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift", "public func tryParseCid(_ cid: String) throws -> String?");
assertContains("packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift", "public func dagCborCodecCode() throws -> UInt32");
assertContains("packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift", "consuming [UInt8]");
assertContains("packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift", "withTextBytes");
assertContains("packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift", "withOwnedBytes");
assertContains("packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift", "operationResult.result = nil");
assertContains("packages/swift/Sources/ReallyMeCodec/MemoryHygiene.swift", "memset_s");
assertContains("packages/swift/Sources/ReallyMeCodec/MemoryHygiene.swift", "explicit_bzero");
assertContains("packages/swift/Sources/ReallyMeCodec/MemoryHygiene.swift", "clearOwned");
assertNotContains("packages/swift/Sources/ReallyMeCodec/MemoryHygiene.swift", "resetBytes");
assertNotContains("packages/swift/Sources/ReallyMeCodec/MemoryHygiene.swift", "initialize(repeating:");
assertContains("packages/swift/Sources/ReallyMeCodec/DeterministicCbor.swift", "let detachedValue = value.value");
assertContains("packages/swift/Sources/ReallyMeCodec/DeterministicCbor.swift", "value.value = nil");
assertContains("packages/swift/Sources/ReallyMeCodec/CallCodecWithRustCAbi.swift", "expectedCodecAbiVersion");
assertContains(
  "packages/swift/Sources/ReallyMeCodec/CallCodecWithRustCAbi.swift",
  "expectedCodecAbiVersion: UInt32 = 5",
);
assertContains(
  "packages/swift/Sources/ReallyMeCodec/CallCodecWithRustCAbi.swift",
  "rm_codec_process_operation",
);
assertContains(
  "packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift",
  "func processGeneratedOperation(request: [UInt8]) throws -> ReallyMeProtoCodecOperationResult",
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
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", "testAbiVersionMismatchFailsClosed");
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", "testSensitiveGeneratedMultikeyRequestFormattingIsRedacted");
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
assertContains("scripts/build_swift_xcframework.sh", "Modules/module.modulemap");
assertNotContains("scripts/build_swift_xcframework.sh", "HEADERS_DIR}/module.modulemap");
assertContains("scripts/build_swift_xcframework.sh", "verify_xcframework_layout");
assertContains("scripts/build_swift_xcframework.sh", "normalize_xcframework_info_plist");
assertContains("scripts/build_swift_xcframework.sh", "Headers/module.modulemap");
assertContains("scripts/prepare_swift_binary_manifest.mjs", "ffiArtifactChecksum");
assertContains("scripts/prepare_swift_binary_manifest.mjs", "--local-artifact-path");
assertContains("packages/swift/README.md", "REALLYME_CODEC_SWIFTPM_RUNTIME_FFI=1");
assertContains(".github/workflows/swift-package-preflight.yml", "REALLYME_CODEC_SWIFTPM_RUNTIME_FFI");
assertContains(".github/workflows/swift-package-preflight.yml", "cargo build --locked -p reallyme-codec-ffi");
assertContains(".github/workflows/code-checks.yml", "cargo build --locked -p reallyme-codec-ffi");
assertContains(".github/workflows/swift-package-preflight.yml", "Build SwiftPM binary artifact");
assertContains(".github/workflows/swift-package-preflight.yml", "Prepare local SwiftPM binary manifest");
assertContains(".github/workflows/swift-package-preflight.yml", "--local-artifact-path build/swift/ReallyMeCodecFFI.xcframework");
assertContains(".github/workflows/swift-package-preflight.yml", "Reset SwiftPM package state");
assertContains(".github/workflows/swift-package-preflight.yml", "swift package reset");
assertContains(".github/workflows/swift-package-preflight.yml", "Test Swift package with linked binary target");
assertContains(
  ".github/workflows/swift-package-preflight.yml",
  "node scripts/run_pinned_release_readiness.mjs --release-packages",
);
assertContains("packages/ts/src/multiformat.ts", "ensureStringValue(encoded)");
assertContains("packages/ts/src/multiformat.ts", "snapshotBoundedBytesInput(bytes)");
assertContains("packages/ts/src/cbor.ts", "ensureStringValue(cid)");
assertContains("packages/ts/src/cbor.ts", "payloadSnapshot.fill(0)");
assertContains("packages/ts/src/cbor.ts", "readDataProperty");
assertContains("packages/ts/src/cbor.ts", 'readDataProperty(value, "length")');
assertContains("packages/ts/src/cbor.ts", 'throw new ReallyMeCodecError("provider-failure")');
assertContains(
  "packages/ts/src/cbor.ts",
  "recordDeterministicCborMapKey(key, keys, providerFailure)",
);
assertContains("packages/ts/src/boundary.ts", "MAX_CODEC_BOUNDARY_NODES = MAX_CODEC_FFI_INPUT_BYTES");
assertNotContains("packages/ts/src/boundary.ts", "MAX_CODEC_BOUNDARY_NODES = 65_536");
assertContains("packages/ts/src/boundary.ts", 'readOwnDataProperty(value, "length")');
assertContains("packages/ts/src/jcs.ts", "stringifyBoundaryJson");
assertContains("packages/ts/src/multiformat.ts", "MAX_MULTICODEC_TABLE_ENTRIES");
assertContains("packages/ts/src/pem.ts", "snapshotDecodePolicy");
assertContains("packages/ts/src/pem.ts", "const pemSnapshot = snapshotBoundedBytesInput(input)");
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
assertContains("packages/ts/src/operationContract.ts", "wipeGeneratedCborValue");
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
  "packages/ts/src/operationContract.ts",
  "export const clearGeneratedOperationResult =",
);
assertContains("packages/ts/src/proto.ts", "CodecOperationResponseSchema");
assertContains("packages/ts/src/proto.ts", "CodecOperationResultSchema");
assertContains("packages/ts/src/operationContract.ts", "errorCodeForCodecErrorMessage");
assertContains("packages/ts/src/operationContract.ts", "MAX_CODEC_PROTO_MESSAGE_BYTES");
assertContains("packages/ts/src/operationContract.ts", "CodecErrorReason.CANONICAL_INTERNAL");
assertContains("packages/ts/src/operationContract.ts", 'case "boundary":');
assertContains("packages/ts/src/operationContract.ts", "expectedOrigin = CodecErrorOrigin.CALLER");
assertContains(
  "packages/ts/src/operationContract.ts",
  "Protobuf enums are open on the wire",
);
assertNotContains("packages/ts/tsconfig.json", '"DOM"');
assertContains("packages/ts/src/readOutput.ts", "MAX_CODEC_FFI_OUTPUT_BYTES");
assertContains("packages/ts/src/readOutput.ts", "export const snapshotBoundedBytesInput =");
assertContains("packages/ts/src/readOutput.ts", "Length-tracking resizable buffers");
assertContains("packages/ts/src/readOutput.ts", "ArrayBuffer.isView(value)");
assertContains("packages/ts/src/readOutput.ts", "value.constructor !== Uint8Array");
assertContains("packages/ts/src/operationContract.ts", "if (text.length > maximum)");
assertContains(
  "packages/ts/src/cbor.ts",
  "MAX_DETERMINISTIC_CBOR_NODES - state.nodes",
);
assertContains("packages/ts/src/cbor.ts", "wipeDagCborValueBytes(child)");
assertContains("packages/ts/src/cbor.ts", "wipeDeterministicCborValueBytes(normalized)");
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
assertContains(
  "packages/ts/src/readOutput.ts",
  "They are malformed provider output",
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
assertContains(
  "crates/codec/src/multicodec.rs",
  "deliberately does not implement `Clone`",
);
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
assertContains("packages/ts/test/reallyme-codec.test.mjs", 'dagCborVerifyCid("", encoded)');
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
assertContains("packages/ts/src/pem.ts", "const pemSnapshot = snapshotBoundedBytesInput(input)");
assertContains("packages/ts/src/pem.ts", "pemSnapshot.fill(0)");
assertContains("packages/ts/src/pem.ts", "const derSnapshot = snapshotBoundedBytesInput(der)");
assertContains("packages/ts/src/pem.ts", "derSnapshot.fill(0)");
assertContains("packages/ts/test/reallyme-codec.test.mjs", "decodePem rejects oversized proto policy before provider work");
assertContains("packages/ts/test/reallyme-codec.test.mjs", "encodePem rejects oversized proto options before provider work");
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
assertMinOccurrences(".github/workflows/swift-package-release.yml", "RELEASE_VERSION: ${{ inputs.version }}", 2);
assertMinOccurrences(".github/workflows/kotlin-android-package-release.yml", "RELEASE_VERSION: ${{ inputs.version }}", 3);
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
assertContains("packages/kotlin-android/build.gradle.kts", 'id("com.android.library") version "9.3.0"');
assertContains("packages/kotlin-android/build.gradle.kts", 'id("com.android.application") version "9.3.0" apply false');
assertContains("packages/kotlin-android/build.gradle.kts", 'artifactId = "codec-android"');
assertContains("packages/kotlin-android/build.gradle.kts", "com.google.protobuf:protobuf-javalite:4.35.1");
assertContains("packages/kotlin-android/build.gradle.kts", "com.google.protobuf:protobuf-kotlin-lite:4.35.1");
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
  '<component group="com.android.tools.build" name="gradle" version="9.3.0">',
);
assertContains("packages/kotlin-android/build.gradle.kts", "fun nonBlank(value: String?): String?");
assertContains("packages/kotlin-android/build.gradle.kts", "verifyRemoteMavenPublishingConfigured");
assertContains("packages/kotlin-android/build.gradle.kts", "remote Maven publishing is not configured");
assertContains("packages/kotlin-android/build.gradle.kts", 'parsed.scheme != "https"');
assertNotContains("packages/kotlin-android/build.gradle.kts", "reallyme.maven.requireRemote");
assertNotContains("packages/kotlin-android/build.gradle.kts", 'name = "localRelease"');
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
assertContains("scripts/test_android_consumer_r8_runtime.sh", "ensure_avd_exists");
assertContains("scripts/test_android_consumer_r8_runtime.sh", "ANDROID_AVD_HOME_VALUE");
assertContains("scripts/test_android_consumer_r8_runtime.sh", "export ANDROID_AVD_HOME");
assertContains("scripts/test_android_consumer_r8_runtime.sh", "$EMULATOR\" -list-avds");
assertContains("scripts/test_android_consumer_r8_runtime.sh", "{ yes || true; } | \"$SDKMANAGER\"");
assertContains("scripts/test_android_consumer_r8_runtime.sh", "$SDKMANAGER\" \"emulator\"");
assertContains("scripts/test_android_consumer_r8_runtime.sh", "Android AVD was not available after creation");
assertContains("scripts/test_android_consumer_r8_runtime.sh", "$AVDMANAGER\" create avd --force");
assertContains("scripts/test_android_consumer_r8_runtime.sh", "Android consumer R8 runtime gate passed");
assertContains(
  ".github/workflows/kotlin-android-package-preflight.yml",
  '{ yes || true; } | "${ANDROID_HOME}/cmdline-tools/latest/bin/sdkmanager" "ndk;29.0.14206865"',
);
assertContains(
  ".github/workflows/kotlin-android-package-release.yml",
  '{ yes || true; } | "${ANDROID_HOME}/cmdline-tools/latest/bin/sdkmanager" "ndk;29.0.14206865"',
);
assertContains("packages/kotlin-android/README.md", "me.really:codec-android:0.2.0");
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
assertContains(".github/workflows/kotlin-android-package-release.yml", "android-aar:");
assertContains(".github/workflows/kotlin-android-package-release.yml", "Write Android native checksum manifest");
assertContains(".github/workflows/kotlin-android-package-preflight.yml", "Write Android native checksum manifest");
assertContains(".github/workflows/kotlin-android-package-release.yml", "verifyReleaseAarContainsJniLibs");
assertContains(".github/workflows/kotlin-android-package-release.yml", "RELEASE_VERSION");
assertContains(".github/workflows/swift-package-release.yml", "needs: [verify-release-sha, swift-artifact]");
assertNotContains(".github/workflows/swift-package-release.yml", "if: inputs.publish == true");
assertNotContains(".github/workflows/kotlin-android-package-release.yml", "if: inputs.publish == true");
assertNotContains(".github/workflows/npm-package-release.yml", "if: inputs.publish == true");
assertContains(".github/workflows/kotlin-android-package-preflight.yml", "android aar preflight");
assertContains(".github/workflows/kotlin-android-package-preflight.yml", "requireAndroidJniLibs=true");
assertNotContains(".github/workflows/kotlin-android-package-preflight.yml", "Install Android emulator image");
assertContains(".github/workflows/kotlin-android-package-preflight.yml", "Enable Android emulator KVM access");
assertContains(".github/workflows/kotlin-android-package-preflight.yml", 'sudo chown "${USER}:${USER}" /dev/kvm');
assertContains(".github/workflows/kotlin-android-package-preflight.yml", "Test Android consumer R8 runtime");
assertContains(".github/workflows/kotlin-android-package-preflight.yml", "REALLYME_CODEC_ANDROID_AVD: reallyme-r8-gate");
assertContains(".github/workflows/kotlin-android-package-preflight.yml", "timeout-minutes: 15");

assertContains("scripts/maven_central_bundle_local.sh", "packages/kotlin/build.gradle.kts");
assertContains("scripts/maven_central_bundle_local.sh", "packages/kotlin-android/build.gradle.kts");
assertContains("scripts/maven_central_bundle_local.sh", "packages/kotlin/build/repos/releases");
assertContains("scripts/maven_central_bundle_local.sh", "packages/kotlin-android/build/repos/releases");
assertNotContains("scripts/maven_central_bundle_local.sh", "packages/kotlin-codec");
assertNotContains("scripts/maven_central_bundle_local.sh", "packages/android-codec");

assertContains("packages/ts/package.json", '"test:browser": "npm run build && node scripts/browser-wasm-test.mjs"');
assertContains("packages/ts/scripts/browser-wasm-test.mjs", "__REALLYME_CODEC_BROWSER_WASM_RESULT__");
assertContains("packages/ts/scripts/browser-wasm-test.mjs", "installReallyMeCodecWasmProvider");
assertContains("packages/ts/scripts/browser-wasm-test.mjs", "deterministicCborDecode");
assertContains("packages/ts/scripts/browser-wasm-test.mjs", "Chrome or Chromium is required");
assertContains(".github/workflows/code-checks.yml", "Test TypeScript codec package in browser");
assertContains(".github/workflows/npm-package-preflight.yml", "Test TypeScript codec package in browser");

assertContains("README.md", "https://github.com/reallyme/codec");
assertContains("README.md", "https://www.npmjs.com/package/@reallyme/codec");
assertContains("README.md", "me.really:codec:0.2.0");
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
assertContains(".github/workflows/protobuf-ci.yml", "redact_codec_proto_debug.mjs");
assertContains("crates/proto/Cargo.toml", '"buffa/json"');
assertContains("scripts/redact_codec_proto_debug.mjs", "validateScalarFieldClassifications");
assertContains("scripts/redact_codec_proto_debug.mjs", "validateSensitiveRustHardening");
assertContains("scripts/redact_codec_proto_debug.mjs", "hardenSensitiveSerialize");
assertContains("scripts/redact_codec_proto_debug.mjs", "hardenSensitiveViewSerialize");
assertContains("scripts/redact_codec_proto_debug.mjs", "__reallyme_decode_sensitive_base64");
assertContains("scripts/redact_codec_proto_debug.mjs", "replaceAllRequired");
assertContains("scripts/redact_codec_proto_debug.mjs", "impl ::core::ops::Drop for");
assertContains("scripts/redact_codec_proto_debug.mjs", "deserialize_zeroizing_bytes");
assertContains("scripts/redact_codec_proto_debug.mjs", "Zeroize::zeroize");
assertContains("scripts/redact_codec_proto_debug.mjs", "__reallyme_zeroize_unknown_fields");
assertContains("scripts/redact_codec_proto_debug.mjs", "deny_unknown_fields");
assertContains("scripts/redact_codec_proto_debug.mjs", "--check-idempotent");
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
assertReallyMeProtobufReleasePolicy({
  buffaVersion: "0.9.0",
  generatedFreshnessMode,
  workflowMode: "delegated",
  generatedFreshnessStepRun:
    "node scripts/run_pinned_release_readiness.mjs --generated-freshness",
  installBufUses:
    "bufbuild/buf-setup-action@a47c93e0b1648d5651a065437926377d060baa99",
  hardeningPolicy: {
    hardeningScript: "scripts/redact_codec_proto_debug.mjs",
    protoSchema: "crates/proto/proto/reallyme/codec/v1/codec.proto",
    generatedRust: "crates/proto/src/generated/buffa/reallyme.codec.v1.codec.rs",
    generatedView: "crates/proto/src/generated/buffa/reallyme.codec.v1.codec.__view.rs",
    protoCargo: "crates/proto/Cargo.toml",
    requiredScriptNeedles: [
      "codecProtoScalarFieldClassifications",
      "codecProtoSensitiveNonTextFieldClassifications",
      "codecProtoProviderOutputMessages",
      "validateScalarFieldClassifications",
      "validateSensitiveRustHardening",
      "hardenSensitiveSerialize",
      "hardenSensitiveViewSerialize",
      "__reallyme_decode_sensitive_base64",
      "__reallyme_deserialize_sensitive_bytes_zeroizing",
      "replaceAllRequired",
      "missing a generated-path or Drop wipe",
      "still accepts ignored ProtoJSON fields",
      "impl ::core::ops::Drop for",
      "deserialize_zeroizing_bytes",
      "Zeroize::zeroize",
      "__reallyme_zeroize_unknown_fields",
      "deny_unknown_fields",
    ],
    requiredCargoNeedles: [
      '"buffa/json"',
      "base64 = { workspace = true, optional = true }",
    ],
    scalarFieldClassifications: codecProtoScalarFieldClassifications,
    requiredGeneratedNeedles: [
      "fn __reallyme_zeroize_unknown_fields(",
      "fn __reallyme_serialize_sensitive_bytes",
      "fn __reallyme_decode_sensitive_base64",
      "fn __reallyme_deserialize_sensitive_bytes_zeroizing",
      "__REALLYME_SENSITIVE_BASE64_STANDARD",
      "__REALLYME_SENSITIVE_BASE64_URL_SAFE",
      "impl ::core::ops::Drop for CodecOperationRequest",
      "Buffa's generated message contract requires Clone",
      "#[serde(default, deny_unknown_fields)]",
      '.field("payload", &"<redacted>")',
      '.field("value", &"<redacted>")',
      '.field("pem", &"<redacted>")',
      '.field("public_key", &"<redacted>")',
      '.field("der", &"<redacted>")',
      '.field("multikey", &"<redacted>")',
      ...codecProtoDropRequiredMessageNames.map(
        (message) => `impl ::core::ops::Drop for ${message}`,
      ),
    ],
    forbiddenGeneratedNeedles: [
      "::buffa::alloc::format!(",
      "serde::de::IgnoredAny",
      '.field("payload", &self.payload)',
      '.field("value", &self.value)',
      '.field("pem", &self.pem)',
      '.field("public_key", &self.public_key)',
      '.field("der", &self.der)',
      '.field("multikey", &self.multikey)',
    ],
    requiredViewNeedles: [
      "super::super::__ReallyMeSensitiveBytes",
      ...codecProtoSensitiveMessageNames.map(
        (message) => `impl<'a> ::core::fmt::Debug for ${message}View<'a>`,
      ),
    ],
    additionalGeneratedPolicies: [
      {
        path: "gen/swift/reallyme/codec/v1/codec.pb.swift",
        required: codecProtoSensitiveMessageNames.flatMap((message) => [
          `ReallyMeProto${message}(<redacted>)`,
          `public nonisolated struct ReallyMeProto${message}: Sendable`,
        ]),
      },
      ...codecProtoSensitiveMessageNames.map((message) => ({
        path: `gen/java/me/really/codec/v1/${message}.java`,
        required: [
          `return "${message}{<redacted>}";`,
          "return 0x524d;",
        ],
      })),
    ],
  },
  generatedFreshness: {
    generatedPaths: [
      "crates/proto/src/generated",
      "packages/ts/src/proto/generated",
      "gen",
    ],
    commands: [
      ["buf", ["lint"]],
      ["buf", ["generate"]],
      ["node", ["scripts/redact_codec_proto_debug.mjs"]],
      ["node", ["scripts/redact_codec_proto_debug.mjs", "--check-idempotent"]],
      ["cargo", ["fmt", "--package", "reallyme-codec-proto"]],
    ],
  },
});
assertContains(
  "crates/proto/src/generated/buffa/reallyme.codec.v1.codec.rs",
  '.field("der", &"<redacted>")',
);
assertContains(
  "crates/proto/src/generated/buffa/reallyme.codec.v1.codec.rs",
  "fn __reallyme_zeroize_unknown_fields(",
);
assertContains(
  "crates/proto/src/generated/buffa/reallyme.codec.v1.codec.rs",
  "#[serde(default, deny_unknown_fields)]",
);
assertNotContains(
  "crates/proto/src/generated/buffa/reallyme.codec.v1.codec.rs",
  '.field("der", &self.der)',
);
assertContains(
  "crates/proto/src/generated/buffa/reallyme.codec.v1.codec.__view.rs",
  "impl<'a> ::core::fmt::Debug for CodecPemDecodeResultView<'a>",
);
assertContains(
  "crates/proto/src/generated/buffa/reallyme.codec.v1.codec.rs",
  "impl ::core::ops::Drop for CodecOperationRequest",
);
assertContains(
  "gen/java/me/really/codec/v1/CodecOperationRequest.java",
  'return "CodecOperationRequest{<redacted>}";',
);
assertContains(
  "gen/swift/reallyme/codec/v1/codec.pb.swift",
  'ReallyMeProtoCodecOperationRequest(<redacted>)',
);
for (const sensitiveOwnedView of codecProtoSensitiveMessageNames.map(
  (message) => `${message}OwnedView`,
)) {
  assertNotContains(
    "crates/proto/src/generated/buffa/reallyme.codec.v1.codec.__view.rs",
    sensitiveOwnedView,
  );
  assertNotContains(
    "crates/proto/src/generated/buffa/reallyme.codec.v1.mod.rs",
    sensitiveOwnedView,
  );
}
assertContains("scripts/redact_codec_proto_debug.mjs", "removeSensitiveRustOwnedViews");
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

console.log("codec release readiness checks passed");
