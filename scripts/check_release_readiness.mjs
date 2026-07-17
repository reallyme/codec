#!/usr/bin/env node
// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { createReleaseReadinessContext } from "./release-readiness/core.mjs";
import {
  codecProtoScalarFieldClassifications,
  codecProtoSensitiveOwnerMessages,
} from "./codec_proto_sensitivity.mjs";

const codecProtoSensitiveMessageNames = [...new Set([
  ...codecProtoScalarFieldClassifications
    .filter((classification) => classification.sensitivity === "sensitive")
    .map((classification) => classification.message),
  ...codecProtoSensitiveOwnerMessages,
])];

const {
  readText,
  readJson,
  listFiles,
  fail,
  assertContains,
  assertNotContains,
  assertMinOccurrences,
  assertNodeWorkflowJobsPinNode,
  assertProtoContract,
  assertReallyMeProtoBoundaryContract,
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

const codecPackageVersion = "0.1.22";
const codecProtoPackageVersion = "0.1.22";
const releasePackagesMode = suppliedArguments.has("--release-packages");
const generatedFreshnessMode = suppliedArguments.has("--generated-freshness");

assertNodeWorkflowJobsPinNode({ nodeVersion: "24" });

const rootCargo = readText("Cargo.toml");
for (const member of [
  '"crates/codec"',
  '"crates/codec/ffi"',
  '"crates/codec/wasm-package"',
  '"crates/proto/codec"',
]) {
  assertContains("Cargo.toml", member);
}
if (rootCargo.includes("crates/crypto") || rootCargo.includes("crates/proto/crypto")) {
  fail("root Cargo.toml still references removed crypto crates");
}
assertContains("Cargo.toml", 'repository = "https://github.com/reallyme/codec"');
assertContains("Cargo.toml", 'buffa = { version = "0.8.1", features = ["json"] }');

for (const rustPath of [
  ...listFiles("crates/codec").filter((path) => path.endsWith(".rs")),
  "crates/proto/codec/src/lib.rs",
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

assertContains("crates/codec/ffi/Cargo.toml", 'name = "reallyme-codec-ffi"');
assertContains("crates/codec/ffi/Cargo.toml", "publish = false");
assertContains("crates/codec/ffi/src/codec.rs", "rm_codec_process_proto");
assertContains("crates/codec/ffi/src/codec.rs", "rm_codec_process_proto_json");
assertContains("crates/codec/ffi/src/codec.rs", "rm_codec_abi_version");
assertContains("crates/codec/ffi/src/codec.rs", "rm_codec_max_proto_result_envelope_bytes");
assertContains("crates/codec/ffi/src/codec.rs", "CODEC_ABI_VERSION");
assertContains("crates/codec/ffi/src/codec.rs", "MAX_CODEC_FFI_INPUT_BYTES");
assertContains("crates/codec/ffi/src/codec.rs", "struct FixedJsonWriter");
assertContains("crates/codec/ffi/src/codec.rs", "initialize_output_length(output_ptr");
assertContains("crates/codec/ffi/src/codec.rs", "write_i32(result_out, 0)");
assertNotContains("crates/codec/ffi/src/codec.rs", "serde_json::to_vec(&cbor_to_tagged");
assertContains("crates/codec/ffi/src/lib.rs", '#[cfg(not(panic = "unwind"))]');
assertContains("crates/codec/ffi/src/lib.rs", "compile_error!");
assertContains("crates/codec/ffi/src/guard.rs", "with_redacted_panic_hook");
assertContains("crates/codec/ffi/src/guard.rs", "INSIDE_NATIVE_BOUNDARY");
assertContains("crates/codec/ffi/src/kotlin_codec.rs", "with_redacted_panic_hook");
assertContains("crates/codec/pem/src/encode.rs", "String::with_capacity(output_length)");
assertContains("crates/codec/pem/src/decode.rs", "String::with_capacity(body_capacity)");
assertNotContains("crates/codec/pem/src/decode.rs", '.replace("\\r\\n"');
assertContains("crates/codec/src/lib.rs", "canonicalize_json_text");
assertContains("crates/codec/src/lib.rs", "canonicalize_trusted_json_value");
assertNotContains("crates/codec/src/lib.rs", "canonicalize_json, JcsError");
assertContains("crates/codec/jcs/src/canonicalize.rs", "#[deprecated(");
assertContains("crates/codec/jcs/src/canonicalize.rs", "binary64 follow RFC 8785");
assertContains("crates/codec/ffi/src/codec.rs", "validate_boundary_input_lengths");
assertContains(
  "crates/codec/ffi/src/codec.rs",
  "validate_proto_boundary_input_length(request_len)",
);
assertContains(
  "crates/codec/ffi/src/kotlin_codec.rs",
  "validate_managed_input_lengths",
);
assertContains("crates/codec/ffi/src/kotlin_codec.rs", "aggregate.checked_add(length)");
assertContains("crates/codec/wasm-package/Cargo.toml", 'name = "reallyme-codec-wasm"');
assertContains("crates/codec/wasm-package/src/boundary.rs", "MAX_WASM_INPUT_BYTES");
assertContains("crates/codec/wasm-package/src/boundary.rs", "checked_add");
assertContains("crates/codec/wasm-package/src/boundary.rs", "validate_js_inputs");
assertContains("crates/codec/wasm-package/src/boundary.rs", "zeroizing_string");
assertContains("crates/codec/wasm-package/src/boundary.rs", "zeroizing_bytes_with_maximum");
assertContains("crates/codec/wasm-package/src/boundary.rs", "value.subarray");
assertContains("crates/codec/wasm-package/src/boundary.rs", "snapshot.fill");
for (const wasmSourcePath of [
  "crates/codec/wasm-package/src/base_encoding.rs",
  "crates/codec/wasm-package/src/cbor.rs",
  "crates/codec/wasm-package/src/jcs.rs",
  "crates/codec/wasm-package/src/multiformat.rs",
  "crates/codec/wasm-package/src/pem.rs",
  "crates/codec/wasm-package/src/proto_output.rs",
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
}
assertContains("crates/codec/wasm-package/src/proto_output.rs", "process_proto");
assertContains("crates/codec/wasm-package/src/proto_output.rs", "process_proto_json");
for (const staleWasmProtoExport of [
  "multicodec_prefix_for_name_proto",
  "multicodec_lookup_prefix_proto",
  "multicodec_table_proto",
  "multikey_parse_proto",
  "dag_cbor_verify_cid_proto",
  "pem_decode_proto",
]) {
  assertNotContains(
    "crates/codec/wasm-package/src/proto_output.rs",
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
  "public func processProto(_ request: [UInt8])",
);
assertContains(
  "packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift",
  "public func processProtoJson(_ requestJson: [UInt8])",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt",
  "public fun processProto(request: ByteArray)",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt",
  "public fun processProtoJson(requestJson: ByteArray)",
);
assertContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt",
  "public fun decodePem(pem: ByteArray",
);
assertNotContains(
  "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt",
  "fun decodePemProto(",
);
assertContains(
  "packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift",
  "public func decodePem(_ pem: [UInt8]",
);
assertNotContains(
  "packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift",
  "func decodePemProto(",
);
assertNotContains("crates/codec/multikey/src/error.rs", "&'static str");
assertContains(
  "scripts/build_swift_xcframework.sh",
  "rm_codec_process_proto_json",
);
assertContains(".github/workflows/code-checks.yml", "cargo-deny@0.19.6");
assertContains(
  ".github/workflows/code-checks.yml",
  "node --test scripts/release-readiness/cli.test.mjs",
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
  ".github/workflows/code-checks.yml",
  "gradle/actions/wrapper-validation@0723195856401067f7a2779048b490ace7a47d7c",
);
assertContains(".github/workflows/code-checks.yml", "node-version: '24'");
assertMinOccurrences(".github/workflows/crates-release.yml", "node-version: '24'", 2);

const codecProtoCargo = readText("crates/proto/codec/Cargo.toml");
if (!codecProtoCargo.includes(`version = "${codecProtoPackageVersion}"`)) {
  fail(`crates/proto/codec/Cargo.toml is not versioned ${codecProtoPackageVersion}`);
}
assertContains("crates/proto/codec/Cargo.toml", 'name = "reallyme-codec-proto"');
assertContains(
  "crates/proto/codec/Cargo.toml",
  '"/proto/**/*.proto"',
);
assertContains(
  "crates/proto/codec/Cargo.toml",
  '"/tests/**/*.rs"',
);
assertContains(
  "crates/proto/codec/README.md",
  `reallyme-codec-proto = { version = "${codecProtoPackageVersion}", features = ["generated"] }`,
);
assertProtoContract("crates/proto/codec/proto/reallyme/codec/v1/codec.proto");
assertReallyMeProtoBoundaryContract({
  protoPath: "crates/proto/codec/proto/reallyme/codec/v1/codec.proto",
  operationRequest: "CodecOperationRequest",
  resultEnvelope: "CodecProtoResultEnvelope",
  resultStatus: "CodecProtoResultStatus",
  protoReadme: "crates/proto/codec/README.md",
  protoCargo: "crates/proto/codec/Cargo.toml",
  wirePath: "crates/codec/src/proto_process.rs",
  codecPath: "crates/proto/codec/src/lib.rs",
  binaryEnvelopeNeedle: "encode_proto_result_envelope_or_error",
  sdkAdapters: [
    {
      path: "crates/codec/wasm-package/src/proto_output.rs",
      processProtoNeedle: "pub fn process_proto(",
      processProtoJsonNeedle: "pub fn process_proto_json(",
      requiredNeedles: [
        "let output: Zeroizing<Vec<u8>> = process_proto_request(request.as_slice());",
        "let output: Zeroizing<Vec<u8>> = process_proto_json_request(request_json.as_slice());",
      ],
    },
    {
      path: "packages/ts/src/protoProcess.ts",
      processProtoNeedle: "export const processProto =",
      processProtoJsonNeedle: "export const processProtoJson =",
    },
    {
      path: "packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift",
      processProtoNeedle: "public func processProto(_ request: [UInt8])",
      processProtoJsonNeedle:
        "public func processProtoJson(_ requestJson: [UInt8])",
    },
    {
      path: "packages/swift/Sources/ReallyMeCodec/CallCodecWithRustCAbi.swift",
      processProtoNeedle: "func processProtoEnvelope(request: [UInt8])",
      processProtoJsonNeedle:
        "func processProtoJsonEnvelope(requestJson: [UInt8])",
      binaryEnvelopeNeedle: "ReallyMeProtoCodecProtoResultEnvelope",
      requiredNeedles: [
        "private static let maxFfiOutputLength = 67_108_864",
        "private typealias CodecProtoResultLimitFunction = @convention(c) () -> Int",
        '@_silgen_name("rm_codec_max_proto_result_envelope_bytes")',
        "private let maxProtoResultEnvelopeLength: Int",
        "rmCodecMaxProtoResultEnvelopeBytesLinked()",
        '"rm_codec_max_proto_result_envelope_bytes"',
        "static func requireValidProtoResultEnvelopeLimit(_ limit: Int) throws -> Int",
        "guard firstStatus == ReallyMeCodecRustCAbiStatus.bufferTooSmall else",
        "producedLength <= maxProtoResultEnvelopeLength",
        "guard producedLength == output.count else",
        "ReallyMeCodecMemory.clearOwned(&envelope.payload)",
        '"rm_codec_abi_version"',
        "decodeProtoResultEnvelope",
      ],
    },
    {
      path: "packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt",
      processProtoNeedle: "public fun processProto(request: ByteArray)",
      processProtoJsonNeedle:
        "public fun processProtoJson(requestJson: ByteArray)",
    },
  ],
});

assertContains("buf.gen.yaml", "out: crates/proto/codec/src/generated/buffa");
assertContains("buf.gen.yaml", "out: packages/ts/src/proto/generated");
assertContains("buf.gen.yaml", "buf.build/bufbuild/es:v2.12.1");
assertContains("buf.gen.yaml", "buf.build/apple/swift:v1.38.1");
assertContains("buf.gen.yaml", "buf.build/protocolbuffers/java:v35.1");
assertContains("buf.gen.yaml", "buf.build/protocolbuffers/kotlin:v35.1");
assertContains("crates/proto/codec/src/generated/buffa/mod.rs", "pub mod codec");
assertContains("crates/proto/codec/src/lib.rs", "pub struct CodecWireError");
assertContains("crates/proto/codec/src/lib.rs", "pub fn try_new");
assertContains("crates/proto/codec/src/lib.rs", "decode_codec_error_payload");
assertContains("crates/proto/codec/src/lib.rs", "MAX_CODEC_PROTO_MESSAGE_BYTES");
assertContains("crates/proto/codec/src/lib.rs", "MAX_CODEC_PROTO_ERROR_ENVELOPE_BYTES");
assertContains("crates/proto/codec/src/lib.rs", "MAX_CODEC_PROTO_RESULT_ENVELOPE_BYTES");
assertContains(
  "crates/codec/ffi/src/kotlin_codec.rs",
  "MAX_CODEC_PROTO_MESSAGE_BYTES, MAX_CODEC_PROTO_RESULT_ENVELOPE_BYTES,",
);
assertContains(
  "crates/codec/ffi/src/kotlin_codec.rs",
  "max_request_len.checked_add(1)",
);
assertContains(
  "crates/codec/ffi/src/kotlin_codec.rs",
  "produced_len > MAX_CODEC_PROTO_RESULT_ENVELOPE_BYTES",
);
assertContains(
  "packages/ts/src/protoProcess.ts",
  "const MAX_CODEC_PROTO_RESULT_ENVELOPE_BYTES = 1_048_592;",
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
  "crates/proto/codec/src/lib.rs",
  "pub fn encode_protobuf<M: Message>(message: &M) -> Zeroizing<Vec<u8>>",
);
assertContains(
  "crates/proto/codec/src/lib.rs",
  ") -> Result<Zeroizing<Vec<u8>>, CodecProtoResult>",
);
assertContains("crates/proto/codec/tests/generated_tests.rs", "malformed_codec_error_payloads_decode_as_boundary_malformed_protobuf");
assertContains("crates/proto/codec/tests/generated_tests.rs", "codec_error_payload_decode_rejects_oversized_envelopes");
assertContains("crates/proto/codec/tests/generated_tests.rs", "json_decode_rejects_inputs_that_expand_past_binary_cap");
assertContains(".github/workflows/protobuf-ci.yml", "BUFFA_VERSION: 0.8.1");
assertContains(".github/workflows/protobuf-ci.yml", "BUF_VERSION: 1.71.0");
assertContains(".github/workflows/protobuf-ci.yml", "scripts/release-readiness/core.mjs");
assertContains(".github/workflows/protobuf-ci.yml", "scripts/run_pinned_release_readiness.mjs");
assertContains(".github/workflows/protobuf-ci.yml", "node-version: '24'");
assertContains(".github/workflows/protobuf-ci.yml", "cargo install protoc-gen-buffa-packaging");
assertContains(".github/workflows/protobuf-ci.yml", "buf breaking --against '.git#branch=origin/main'");
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
assertContains(".github/workflows/package-release.yml", "Write native checksum manifest");
assertContains(".github/workflows/release-preflight.yml", "Write native checksum manifest");
assertContains(".github/workflows/jvm-native-resources.yml", "jvm native resources");
assertContains(".github/workflows/jvm-native-resources.yml", "kotlin-native-");
assertContains(".github/workflows/jvm-native-resources.yml", "build_kotlin_native_resource.sh");
assertContains(".github/workflows/jvm-native-resources.yml", "Test host native loader");
assertContains(".github/workflows/package-release.yml", "Test host native loader");
assertContains(".github/workflows/release-preflight.yml", "Test host native loader");
assertContains(
  ".github/workflows/jvm-native-resources.yml",
  "actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a",
);
assertWorkflowPermissionsPolicy({
  path: ".github/workflows/package-release.yml",
  workflow: { contents: "read" },
  jobs: {
    "verify-release-sha": { actions: "read", contents: "read" },
    "maven-package": { actions: "read", contents: "read" },
    "android-aar": { actions: "read", contents: "read" },
    "swift-release": { actions: "read", contents: "write" },
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
assertContains(".github/workflows/package-release.yml", "publish_swift:");
assertContains(".github/workflows/package-release.yml", "publish_maven:");
assertContains(".github/workflows/package-release.yml", "release_sha:");
assertContains(".github/workflows/package-release.yml", "Leave blank to use the current origin/main SHA");
assertContains(".github/workflows/package-release.yml", "Resolve release SHA");
assertContains(".github/workflows/package-release.yml", "default: 0.1.22");
assertContains(".github/workflows/package-release.yml", "Verify audited release SHA");
assertContains(".github/workflows/crates-release.yml", "Verify audited release SHA");
assertContains(".github/workflows/crates-release.yml", "Resolve release SHA");
assertContains(".github/workflows/release-preflight.yml", "Resolve release SHA");
assertContains(".github/workflows/release-preflight.yml", "default: 0.1.22");
assertContains(".github/workflows/release-preflight.yml", "needs: [verify-source-sha, jvm-native]");
assertContains(".github/workflows/package-release.yml", "needs: [verify-release-sha, jvm-native]");
assertContains(".github/workflows/package-release.yml", "needs: [verify-release-sha, swift-artifact]");
assertContains(".github/workflows/package-release.yml", "needs: [verify-release-sha, swift-artifact, swift-verify]");
assertContains(".github/workflows/crates-release.yml", "needs: [verify-release-sha, dry-run]");
assertWorkflowRunStep(
  ".github/workflows/package-release.yml",
  "Require current main and successful checks for exact SHA",
  "node scripts/verify_release_attestation.mjs",
);
assertWorkflowRunStep(
  ".github/workflows/crates-release.yml",
  "Require current main and successful checks for exact SHA",
  "node scripts/verify_release_attestation.mjs",
);
assertWorkflowRunStep(
  ".github/workflows/package-release.yml",
  "Publish Maven artifact",
  `node ../../scripts/verify_release_attestation.mjs
./gradlew publish -Preallyme.codec.nativeResourcesDir=\${{ github.workspace }}/build/kotlin-native-resources`,
);
assertWorkflowRunStep(
  ".github/workflows/package-release.yml",
  "Publish Android AAR",
  `node scripts/verify_release_attestation.mjs
packages/kotlin/gradlew -p packages/kotlin-android publish -Preallyme.codec.androidJniLibsDir=\${{ github.workspace }}/build/android-jniLibs -Preallyme.codec.androidNativeAssetsDir=\${{ github.workspace }}/build/android-native-assets -Preallyme.codec.requireAndroidJniLibs=true`,
);
assertMinOccurrences(
  ".github/workflows/package-release.yml",
  "steps.maven_remote.outputs.configured == 'true'",
  2,
);
assertContains(".github/workflows/package-release.yml", "configured=false");
assertContains(
  ".github/workflows/package-release.yml",
  "remote Maven credentials are incomplete; packaged artifacts were verified locally and remote publish is skipped",
);
assertWorkflowRunStep(
  ".github/workflows/crates-release.yml",
  "Publish crates in dependency order",
  `node scripts/verify_release_attestation.mjs
node scripts/publish_crates_in_order.mjs publish`,
);
assertWorkflowRunStep(
  ".github/workflows/package-release.yml",
  "Verify SwiftPM manifest",
  `node scripts/verify_swift_release_artifact.mjs build/swift/ReallyMeCodecFFI.xcframework.zip build/swift/ReallyMeCodecFFI.xcframework.checksum Package.swift "\${RELEASE_VERSION}"
node scripts/run_pinned_release_readiness.mjs --release-packages`,
);
assertContains(".github/workflows/package-release.yml", "SwiftPM artifact verification");
assertWorkflowRunStep(
  ".github/workflows/package-release.yml",
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
assertMinOccurrences(".github/workflows/package-release.yml", "node-version: '24'", 3);
assertMinOccurrences(".github/workflows/release-preflight.yml", "node-version: '24'", 5);
for (const workflowPath of [
  ".github/workflows/fuzz.yml",
  ".github/workflows/jvm-native-resources.yml",
  ".github/workflows/package-release.yml",
  ".github/workflows/release-preflight.yml",
]) {
  assertNotContains(workflowPath, "actions/upload-artifact@330a01c490aca151604b8cf639adc76d48f6c5d4");
  assertNotContains(workflowPath, "actions/download-artifact@634f93cb2916e3fdff6788551b99b062d0335ce0");
}
assertContains(
  ".github/workflows/package-release.yml",
  "actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a # v7.0.1",
);
assertMinOccurrences(".github/workflows/fuzz.yml", "toolchain: nightly-2026-07-01", 2);
assertNotContains(".github/workflows/fuzz.yml", "cargo +nightly fuzz");
assertContains(".github/workflows/fuzz.yml", "- proto_process");
assertContains("fuzz/Cargo.toml", 'name = "proto_process"');
assertContains("fuzz/README.md", "`proto_process`");
assertContains("fuzz/fuzz_targets/proto_process.rs", "process_proto(data)");
assertContains("fuzz/fuzz_targets/proto_process.rs", "process_proto_json(data)");
assertContains("fuzz/Cargo.toml", 'name = "jcs_text"');
assertContains("fuzz/fuzz_targets/jcs_text.rs", "canonicalize_json_text(json)");
assertContains("fuzz/README.md", "`jcs_text`");
assertContains(".github/workflows/fuzz.yml", "- jcs_text");
assertContains(
  ".github/workflows/package-release.yml",
  "actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c # v8.0.1",
);
assertContains(".github/workflows/package-release.yml", "requireFullNativeResources=true");
assertContains(".github/workflows/release-preflight.yml", "requireFullNativeResources=true");
assertContains("packages/kotlin/settings.gradle.kts", 'rootProject.name = "reallyme-codec"');
assertContains("packages/kotlin/README.md", "me.really:codec:0.1.22");
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
assertNotContains(".github/workflows/package-release.yml", "-Preallyme.maven.requireRemote=true");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "public fun tryParseCid(cid: String): String?");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "public fun dagCborCodecCode(): Int");
assertContains("crates/codec/ffi/src/codec.rs", "MAX_CODEC_FFI_OUTPUT_BYTES");
assertContains("crates/codec/ffi/src/kotlin_codec.rs", "probed_output_capacity");
assertContains("crates/codec/ffi/src/kotlin_codec.rs", "produced_len != output.len()");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "CodingErrorAction.REPORT");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "bytes.fill(0)");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "boundaryResourceLimitResult");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "exceptionForCodecErrorPayload");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "CODEC_ERROR_REASON_CANONICAL_INTERNAL");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED");
assertContains("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "withTextBytes");
assertContains(
  "packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt",
  "nativeDigestMetadataAndTempPermissionsFailClosed",
);
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", 'assertNull(codec.tryParseCid("not-a-cid"))');
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", "assertEquals(0x71, codec.dagCborCodecCode())");
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", 'codec.base58btcDecode("")');
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", "codec.base58btcEncode(oversizedBase58Input)");
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", 'codec.multicodecPrefixForNameProto("not-a-codec")');
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", "codec.multicodecLookupPrefixProto(byteArrayOf(0, 0, 7))");
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", 'codec.dagCborVerifyCid("", encoded)');
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", "throwingProtoApisPreserveCallerVersusProviderAttribution");
assertContains("crates/codec/multibase/src/base58btc.rs", "bytes.len() > MAX_BASE58BTC_INPUT_LEN");
assertContains("crates/codec/multibase/tests/base58btc_tests.rs", "rejects_inputs_above_encode_cap_before_base58_conversion");
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
const codecVectorManifest = readJson("test-vectors/codec-vectors.json");
if (codecVectorManifest.schemaVersion !== 2) {
  fail("test-vectors/codec-vectors.json must use schemaVersion 2");
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
    fail(`test-vectors/codec-vectors.json is missing rejection vector ${key}`);
  }
}
for (const provenance of ["official", "trusted-upstream", "reallyme-pinned"]) {
  if (!codecVectorManifest.sources.some((source) => source.provenance === provenance)) {
    fail(`test-vectors/codec-vectors.json is missing ${provenance} provenance`);
  }
}
assertContains("test-vectors/README.md", "official");
assertContains("test-vectors/README.md", "trusted-upstream");
assertContains("test-vectors/README.md", "reallyme-pinned");
assertContains("crates/codec/tests/vector_suite.rs", "shared_vector_suite_covers_core_codec_methods");
assertContains("crates/codec/tests/vector_suite.rs", "shared_vector_suite_rejects_non_canonical_inputs");
assertContains("packages/ts/test/reallyme-codec.test.mjs", "shared codec vector suite covers TypeScript public methods");
assertContains("packages/ts/test/reallyme-codec.test.mjs", "shared codec vector suite rejects non-canonical inputs in TypeScript");
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", "sharedVectorSuiteCoversKotlinPublicMethods");
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", "sharedVectorSuiteRejectsNonCanonicalInputs");
assertContains("packages/kotlin/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", "codec.processProtoJson");
assertContains("packages/kotlin/src/test/java/me/really/codec/ReallyMeCodecJavaTest.java", "ReallyMeCodec.processProtoJson");

assertContains("Package.swift", 'name: "reallyme-codec"');
assertContains("Package.swift", 'name: "ReallyMeCodec"');
assertContains("Package.swift", 'name: "ReallyMeCodecProto"');
assertContains("Package.swift", 'name: "ReallyMeCodecFFI"');
assertContains("Package.swift", 'from: "1.38.1"');
assertContains("Package.swift", "ReallyMeCodecFFI.xcframework.zip");
assertContains("Package.swift", 'let ffiArtifactLocalPathOverride = ""');
assertNotContains("Package.swift", "build/swift/ReallyMeCodecFFI.xcframework");
assertNotContains("Package.swift", "FileManager.default.fileExists");
assertContains("Package.swift", "REALLYME_CODEC_SWIFTPM_RUNTIME_FFI");
assertContains("packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift", "public func tryParseCid(_ cid: String) throws -> String?");
assertContains("packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift", "public func dagCborCodecCode() throws -> UInt32");
assertContains("packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift", "consuming [UInt8]");
assertContains("packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift", "boundaryResourceLimitResult");
assertContains("packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift", "withTextBytes");
assertContains("packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift", "withOwnedBytes");
assertContains("packages/swift/Sources/ReallyMeCodec/MemoryHygiene.swift", "memset_s");
assertContains("packages/swift/Sources/ReallyMeCodec/MemoryHygiene.swift", "explicit_bzero");
assertContains("packages/swift/Sources/ReallyMeCodec/MemoryHygiene.swift", "clearOwned");
assertNotContains("packages/swift/Sources/ReallyMeCodec/MemoryHygiene.swift", "resetBytes");
assertNotContains("packages/swift/Sources/ReallyMeCodec/MemoryHygiene.swift", "initialize(repeating:");
assertContains("packages/swift/Sources/ReallyMeCodec/CallCodecWithRustCAbi.swift", "expectedCodecAbiVersion");
assertContains("packages/swift/Sources/ReallyMeCodec/CallCodecWithRustCAbi.swift", "throw ReallyMeCodecError.providerFailure");
assertContains("packages/swift/Sources/ReallyMeCodec/CallCodecWithRustCAbi.swift", "errorForCodecErrorPayload");
assertContains("packages/swift/Sources/ReallyMeCodec/CallCodecWithRustCAbi.swift", ".canonicalInternal");
assertContains("packages/swift/Sources/ReallyMeCodec/CallCodecWithRustCAbi.swift", ".boundaryResourceLimitExceeded");
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", 'XCTAssertNil(try codec.tryParseCid("not-a-cid"))');
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", "XCTAssertEqual(try codec.dagCborCodecCode(), 0x71)");
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", 'codec.base58btcDecode("")');
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", "codec.base58btcEncode(oversizedBase58Input)");
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", 'codec.multicodecPrefixForNameProto("not-a-codec")');
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", "codec.multicodecLookupPrefixProto([0, 0, 7])");
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", 'codec.dagCborVerifyCid(cid: "", bytes: encoded)');
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", "testSharedVectorSuiteCoversSwiftPublicMethods");
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", "testSharedVectorSuiteRejectsNonCanonicalInputs");
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", "codec.processProtoJson");
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", "testMalformedProviderEnvelopeMapsToTypedFailure");
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", "testAbiVersionMismatchFailsClosed");
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", "testSensitiveGeneratedMultikeyRequestFormattingIsRedacted");
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", "testThrowingProtoApisPreserveCallerVersusProviderAttribution");
assertContains("scripts/build_swift_xcframework.sh", "xcodebuild -create-xcframework");
assertContains("scripts/build_swift_xcframework.sh", "-C panic=unwind");
assertContains("scripts/build_swift_xcframework.sh", "cargo build --locked");
assertContains("scripts/build_swift_xcframework.sh", "rm_codec_abi_version");
assertContains("scripts/build_swift_xcframework.sh", "rm_codec_max_proto_result_envelope_bytes");
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
assertContains(".github/workflows/release-preflight.yml", "REALLYME_CODEC_SWIFTPM_RUNTIME_FFI");
assertContains(".github/workflows/release-preflight.yml", "cargo build --locked -p reallyme-codec-ffi");
assertContains(".github/workflows/code-checks.yml", "cargo build --locked -p reallyme-codec-ffi");
assertContains(".github/workflows/release-preflight.yml", "Build SwiftPM binary artifact");
assertContains(".github/workflows/release-preflight.yml", "Prepare local SwiftPM binary manifest");
assertContains(".github/workflows/release-preflight.yml", "--local-artifact-path build/swift/ReallyMeCodecFFI.xcframework");
assertContains(".github/workflows/release-preflight.yml", "Reset SwiftPM package state");
assertContains(".github/workflows/release-preflight.yml", "swift package reset");
assertContains(".github/workflows/release-preflight.yml", "Test Swift package with linked binary target");
assertContains(
  ".github/workflows/release-preflight.yml",
  "node scripts/run_pinned_release_readiness.mjs --release-packages",
);
assertContains("packages/ts/src/multiformat.ts", "ensureStringValue(encoded)");
assertContains("packages/ts/src/cbor.ts", "ensureStringValue(cid)");
assertContains("packages/ts/src/cbor.ts", "readDataProperty");
assertContains("packages/ts/src/cbor.ts", 'readDataProperty(value, "length")');
assertContains("packages/ts/src/cbor.ts", 'throw new ReallyMeCodecError("provider-failure")');
assertContains("packages/ts/src/boundary.ts", "MAX_CODEC_BOUNDARY_NODES = MAX_CODEC_FFI_INPUT_BYTES");
assertNotContains("packages/ts/src/boundary.ts", "MAX_CODEC_BOUNDARY_NODES = 65_536");
assertContains("packages/ts/src/boundary.ts", 'readOwnDataProperty(value, "length")');
assertContains("packages/ts/src/jcs.ts", "stringifyBoundaryJson");
assertContains("packages/ts/src/multiformat.ts", "MAX_MULTICODEC_TABLE_ENTRIES");
assertContains("packages/ts/src/pem.ts", "snapshotDecodePolicy");
assertContains("packages/ts/src/protoProcess.ts", "envelope.payload.fill(0)");
assertContains("packages/ts/src/protoProcess.ts", "boundaryResourceLimitResult");
assertContains("packages/ts/src/protoProcess.ts", "errorCodeForCodecError");
assertContains("packages/ts/src/protoProcess.ts", "MAX_CODEC_PROTO_JSON_BYTES");
assertContains("packages/ts/src/protoProcess.ts", "CodecErrorReason.CANONICAL_INTERNAL");
assertContains("packages/ts/src/protoProcess.ts", "CodecErrorReason.BOUNDARY_RESOURCE_LIMIT_EXCEEDED");
assertNotContains("packages/ts/tsconfig.json", '"DOM"');
assertContains("packages/ts/src/readOutput.ts", "MAX_CODEC_FFI_OUTPUT_BYTES");
assertContains("packages/ts/src/wasmProvider.ts", "Object.getOwnPropertyDescriptor(module, name)");
assertContains("packages/ts/src/proto.ts", "CodecBackendErrorSchema");
assertContains("packages/ts/test/reallyme-codec.test.mjs", 'assert.deepEqual(base58btcDecode(""), bytes())');
assertContains("packages/ts/test/reallyme-codec.test.mjs", 'dagCborVerifyCid("", encoded)');
assertContains("packages/ts/test/reallyme-codec.test.mjs", "throwing protobuf APIs preserve caller-versus-provider attribution");
assertContains("packages/ts/test/reallyme-codec.test.mjs", "array metadata is snapshotted once without invoking proxy getters");
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
assertContains("packages/kotlin-android/build.gradle.kts", 'id("com.android.library")');
assertContains("packages/kotlin-android/build.gradle.kts", 'id("com.android.library") version "9.3.0"');
assertContains("packages/kotlin-android/build.gradle.kts", 'artifactId = "codec-android"');
assertContains("packages/kotlin-android/build.gradle.kts", "com.google.protobuf:protobuf-javalite:4.35.1");
assertContains("packages/kotlin-android/build.gradle.kts", "com.google.protobuf:protobuf-kotlin-lite:4.35.1");
assertContains("packages/kotlin-android/build.gradle.kts", "jniLibs.directories");
assertContains("packages/kotlin-android/build.gradle.kts", "assets.directories");
assertContains("packages/kotlin-android/build.gradle.kts", "reallyme-codec/native-manifest.json");
assertContains("packages/kotlin-android/build.gradle.kts", "inputs.dir(jniLibsDir).optional()");
assertContains("packages/kotlin-android/build.gradle.kts", "verifyAndroidJniLibs");
assertContains("packages/kotlin-android/build.gradle.kts", "verifyReleaseAarContainsJniLibs");
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
assertContains("packages/kotlin-android/consumer-rules.pro", "ReallyMeCodecProtoStatus");
assertContains("packages/kotlin-android/consumer-rules.pro", "ReallyMeCodecProtoResult");
assertContains("packages/kotlin-android/README.md", "me.really:codec-android:0.1.22");
assertContains("packages/kotlin-android/README.md", "never sourced from the Git worktree");
for (const trackedAndroidFile of listFiles("packages/kotlin-android")) {
  if (trackedAndroidFile.endsWith(".so")) {
    fail(`${trackedAndroidFile} is a tracked prebuilt Android native library`);
  }
}
assertContains("scripts/build_android_native_resources.sh", "aarch64-linux-android");
assertContains("scripts/build_android_native_resources.sh", "-C panic=unwind");
assertContains("scripts/build_android_native_resources.sh", "cargo build --locked");
assertContains("scripts/build_android_native_resources.sh", "armv7-linux-androideabi");
assertContains("scripts/build_android_native_resources.sh", "x86_64-linux-android");
assertContains("scripts/build_android_native_resources.sh", "i686-linux-android");
assertContains("scripts/build_kotlin_native_resource.sh", "-C panic=unwind");
assertContains("scripts/build_kotlin_native_resource.sh", "cargo build --locked");
assertContains("packages/kotlin/build.gradle.kts", "-C panic=unwind");
assertContains(".github/workflows/package-release.yml", "ANDROID_NDK_HOME=${ANDROID_HOME}/ndk/29.0.14206865");
assertNotContains(".github/workflows/package-release.yml", "ANDROID_NDK_HOME: ${{ env.ANDROID_HOME }}/ndk/29.0.14206865");
assertContains(".github/workflows/release-preflight.yml", "ANDROID_NDK_HOME=${ANDROID_HOME}/ndk/29.0.14206865");
assertContains(".github/workflows/package-release.yml", "android-aar:");
assertContains(".github/workflows/package-release.yml", "Write Android native checksum manifest");
assertContains(".github/workflows/release-preflight.yml", "Write Android native checksum manifest");
assertContains(".github/workflows/package-release.yml", "verifyReleaseAarContainsJniLibs");
assertContains(".github/workflows/package-release.yml", "RELEASE_VERSION");
assertContains(".github/workflows/package-release.yml", "needs: [verify-release-sha, swift-artifact]");
assertContains(".github/workflows/package-release.yml", "if: inputs.publish_swift == true");
assertContains(".github/workflows/package-release.yml", "if: inputs.publish_maven == true");
assertNotContains(".github/workflows/package-release.yml", "if: inputs.publish == true");
assertContains(".github/workflows/release-preflight.yml", "android aar preflight");
assertContains(".github/workflows/release-preflight.yml", "requireAndroidJniLibs=true");

assertContains("README.md", "https://github.com/reallyme/codec");
assertContains("README.md", "https://www.npmjs.com/package/@reallyme/codec");
assertContains("README.md", "me.really:codec:0.1.22");
assertContains("README.md", "reallyme-codec-proto");
assertContains("README.md", "## Published Surfaces");
assertContains("README.md", "`me.really:codec-android` AAR");
assertContains("README.md", "## Source Map");
assertContains("CONTRACT.md", "reallyme/codec");
assertContains("SECURITY.md", "reallyme-codec");
assertContains("SECURITY_MEMORY_MODEL.md", "reallyme-codec");
assertContains("buf.yaml", "modules:");
assertContains("buf.yaml", "- path: crates/proto/codec/proto");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "package reallyme.codec.v1;");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "message CodecError");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "message CodecOperationRequest");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "reserved 1 to 999;");
assertContains(
  "crates/proto/codec/proto/reallyme/codec/v1/codec.proto",
  "CodecMulticodecPrefixForNameRequest multicodec_prefix_for_name = 1000;",
);
assertContains(
  "crates/proto/codec/proto/reallyme/codec/v1/codec.proto",
  "CodecMultikeyParseRequest multikey_parse = 2000;",
);
assertContains(
  "crates/proto/codec/proto/reallyme/codec/v1/codec.proto",
  "CodecDagCborVerifyCidRequest dag_cbor_verify_cid = 3000;",
);
assertContains(
  "crates/proto/codec/proto/reallyme/codec/v1/codec.proto",
  "CodecPemDecodeRequest pem_decode = 4000;",
);
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "message CodecBaseEncodingError");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "message CodecPemError");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "message CodecMultiformatError");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "message CodecCanonicalizationError");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "message CodecBackendError");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "message CodecMulticodecSpec");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "message CodecMultikeyParseResult");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "message CodecPemDecodeResult");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "message CodecDagCborVerifyCidResult");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "enum CodecErrorReason");
assertContains(".github/workflows/protobuf-ci.yml", "redact_codec_proto_debug.mjs");
assertContains("crates/proto/codec/Cargo.toml", '"buffa/json"');
assertContains("scripts/redact_codec_proto_debug.mjs", "validateScalarFieldClassifications");
assertContains("scripts/redact_codec_proto_debug.mjs", "validateSensitiveRustHardening");
assertContains("scripts/redact_codec_proto_debug.mjs", "replaceAllRequired");
assertContains("scripts/redact_codec_proto_debug.mjs", "impl ::core::ops::Drop for");
assertContains("scripts/redact_codec_proto_debug.mjs", "deserialize_zeroizing_bytes");
assertContains("scripts/redact_codec_proto_debug.mjs", "Zeroize::zeroize");
assertContains("scripts/redact_codec_proto_debug.mjs", "__reallyme_zeroize_unknown_fields");
assertContains("scripts/redact_codec_proto_debug.mjs", "deny_unknown_fields");
assertContains("scripts/redact_codec_proto_debug.mjs", "--check-idempotent");
assertContains("scripts/codec_proto_sensitivity.mjs", "codecProtoScalarFieldClassifications");
assertContains("scripts/codec_proto_sensitivity.mjs", "codecProtoSensitiveOwnerMessages");
assertReallyMeProtobufReleasePolicy({
  generatedFreshnessMode,
  workflowMode: "delegated",
  generatedFreshnessStepRun:
    "node scripts/run_pinned_release_readiness.mjs --generated-freshness",
  installBufUses:
    "bufbuild/buf-setup-action@a47c93e0b1648d5651a065437926377d060baa99",
  hardeningPolicy: {
    hardeningScript: "scripts/redact_codec_proto_debug.mjs",
    protoSchema: "crates/proto/codec/proto/reallyme/codec/v1/codec.proto",
    generatedRust: "crates/proto/codec/src/generated/buffa/reallyme.codec.v1.codec.rs",
    generatedView: "crates/proto/codec/src/generated/buffa/reallyme.codec.v1.codec.__view.rs",
    protoCargo: "crates/proto/codec/Cargo.toml",
    requiredScriptNeedles: [
      "codecProtoScalarFieldClassifications",
      "validateScalarFieldClassifications",
      "validateSensitiveRustHardening",
      "replaceAllRequired",
      "missing a generated-path or Drop wipe",
      "still accepts ignored ProtoJSON fields",
      "impl ::core::ops::Drop for",
      "deserialize_zeroizing_bytes",
      "Zeroize::zeroize",
      "__reallyme_zeroize_unknown_fields",
      "deny_unknown_fields",
    ],
    requiredCargoNeedles: ['"buffa/json"'],
    scalarFieldClassifications: codecProtoScalarFieldClassifications,
    requiredGeneratedNeedles: [
      "fn __reallyme_zeroize_unknown_fields(",
      "impl ::core::ops::Drop for CodecOperationRequest",
      "Buffa's generated message contract requires Clone",
      "#[serde(default, deny_unknown_fields)]",
      '.field("payload", &"<redacted>")',
      '.field("value", &"<redacted>")',
      '.field("pem", &"<redacted>")',
      '.field("public_key", &"<redacted>")',
      '.field("der", &"<redacted>")',
      '.field("multikey", &"<redacted>")',
      "impl ::core::ops::Drop for CodecMultikeyParseRequest",
      ...codecProtoSensitiveOwnerMessages.map(
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
      "crates/proto/codec/src/generated",
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
  "crates/proto/codec/src/generated/buffa/reallyme.codec.v1.codec.rs",
  '.field("der", &"<redacted>")',
);
assertContains(
  "crates/proto/codec/src/generated/buffa/reallyme.codec.v1.codec.rs",
  "fn __reallyme_zeroize_unknown_fields(",
);
assertContains(
  "crates/proto/codec/src/generated/buffa/reallyme.codec.v1.codec.rs",
  "#[serde(default, deny_unknown_fields)]",
);
assertNotContains(
  "crates/proto/codec/src/generated/buffa/reallyme.codec.v1.codec.rs",
  '.field("der", &self.der)',
);
assertContains(
  "crates/proto/codec/src/generated/buffa/reallyme.codec.v1.codec.__view.rs",
  "impl<'a> ::core::fmt::Debug for CodecPemDecodeResultView<'a>",
);
assertContains(
  "crates/proto/codec/src/generated/buffa/reallyme.codec.v1.codec.rs",
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
    "crates/proto/codec/src/generated/buffa/reallyme.codec.v1.codec.__view.rs",
    sensitiveOwnedView,
  );
  assertNotContains(
    "crates/proto/codec/src/generated/buffa/reallyme.codec.v1.mod.rs",
    sensitiveOwnedView,
  );
}
assertContains("scripts/redact_codec_proto_debug.mjs", "removeSensitiveRustOwnedViews");
assertContains(
  "crates/proto/codec/tests/generated_tests.rs",
  "pem_decode_result_debug_redacts_der",
);
assertContains(
  "crates/proto/codec/tests/generated_tests.rs",
  "generated_proto_json_rejects_unknown_fields",
);
assertContains(
  "crates/proto/codec/tests/generated_tests.rs",
  "operation_request_wire_tags_use_sparse_family_bands",
);

console.log("codec release readiness checks passed");
