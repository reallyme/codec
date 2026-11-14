// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

// Release Readiness 0.7.0's aggregate protobuf policy requires upstream tool
// names and Rust source patches. Reuse its granular checks while covering this
// specific native-generation gap, pending support in the shared framework.
export function assertNativeOperationBoundary(context, policy) {
  const { assertContains, assertNotContains, assertProtoContract, extractProtoBlocks,
    stripProtoLineComments, readText, fail } = context;
  const { protoPath, operationRequest, operationResponse, protoReadme, protoCargo,
    wirePath, codecPath, binaryResponseNeedle, sdkAdapters } = policy;
  assertProtoContract(protoPath);
  const source = stripProtoLineComments(readText(protoPath));
  if (extractProtoBlocks(source, "service").length !== 0) fail("codec contract must remain message-only");
  const messages = new Map(extractProtoBlocks(source, "message").map((block) => [block.name, block.body]));
  for (const [message, oneof] of [[operationRequest, "operation"], [operationResponse, "outcome"], ["CodecOperationResult", "result"]]) {
    if (!new RegExp(`\\boneof\\s+${oneof}\\s*\\{`, "u").test(messages.get(message) ?? "")) {
      fail("codec operation contract requires its generated discriminated envelopes");
    }
  }
  const response = messages.get(operationResponse) ?? "";
  for (const [type, field, number] of [["CodecOperationResult", "result", 1], ["CodecError", "error", 2]]) {
    if (!new RegExp(`^\\s*${type}\\s+${field}\\s*=\\s*${number}\\s*;`, "mu").test(response)) {
      fail("codec operation response must retain its typed result/error fields");
    }
  }
  assertContains(protoReadme, "JSON is a generated ProtoJSON request convenience. Results remain one fully");
  for (const needle of ["local: protoc-gen-reallyme-buffa", "views=true", "json=true"]) assertContains("buf.gen.yaml", needle);
  for (const needle of ['"buffa/json"', "zeroize"]) assertContains(protoCargo, needle);
  for (const needle of [operationRequest, operationResponse, "Zeroizing<Vec<u8>>",
    "pub fn process_operation_response(", "pub fn process_operation_response_json("]) assertContains(wirePath, needle);
  for (const needle of ["DecodeOptions::new()", binaryResponseNeedle]) assertContains(codecPath, needle);
  for (const needle of ["pub fn process_json(", "pub fn process_proto_with_operation",
    "pub fn process_proto_operation", "CodecProtoResultEnvelope"]) assertNotContains(wirePath, needle);
  for (const adapter of sdkAdapters) {
    for (const needle of [adapter.processOperationNeedle, adapter.processOperationJsonNeedle,
      ...(adapter.requiredNeedles ?? [])]) {
      assertContains(adapter.path, needle);
    }
    assertContains(adapter.binaryResponsePath ?? adapter.path, adapter.binaryResponseNeedle ?? operationResponse);
    for (const needle of adapter.forbiddenNeedles ?? []) assertNotContains(adapter.path, needle);
  }
}

export function assertNativeProtoPolicy(context, { sensitiveMessages, generatedFreshnessMode }) {
  const {
    assertContains, assertNotContains, assertWorkflowRunStep,
    assertWorkflowUsesStepPinnedTo, assertGeneratedArtifactsFresh,
    readText, run, fail,
  } = context;
  const schema = "crates/proto/proto/reallyme/codec/v1/codec.proto";
  const rust = "crates/proto/src/generated/buffa/reallyme.codec.v1.codec.rs";
  const view = "crates/proto/src/generated/buffa/reallyme.codec.v1.codec.__view.rs";
  const exports = "crates/proto/src/generated/contract_exports.rs";
  const workflow = ".github/workflows/protobuf-ci.yml";

  assertContains("Cargo.toml", 'buffa = { package = "reallyme-buffa", version = "=0.1.1", features = ["json", "zeroize"] }');
  assertContains("crates/proto/Cargo.toml", '"buffa/zeroize"');
  assertContains(workflow, "BUFFA_VERSION: 0.1.1");
  assertContains(workflow, "BUF_VERSION: 1.75.0");
  assertContains(workflow, "key: buffa-${{ env.BUFFA_VERSION }}");
  assertWorkflowUsesStepPinnedTo(workflow, "Install buf", "bufbuild/buf-action");
  assertWorkflowRunStep(workflow, "Install pinned Buffa generators", `cargo install reallyme-protoc-gen-buffa --version "=$BUFFA_VERSION" --locked
cargo install reallyme-protoc-gen-buffa-packaging --version "=$BUFFA_VERSION" --locked
node scripts/generate_codec_proto.mjs --check-tools`);
  assertWorkflowRunStep(workflow, "Check release readiness generated freshness",
    "node scripts/run_pinned_release_readiness.mjs --generated-freshness");

  for (const option of [
    "local: protoc-gen-reallyme-buffa", "local: protoc-gen-reallyme-buffa-packaging",
    "strategy: all", "views=true", "json=true", "zeroize=true",
    "unknown_fields=false", "deny_unknown_json_fields=true", "allow_unmatched_message_rules=false",
    "bytes_type_custom_in=.reallyme.codec.v1=crate::guarded::SecretBytes",
    "string_type_custom_in=.reallyme.codec.v1=crate::guarded::SecretString",
    "json_policy_type=crate::guarded::SecretBytes", "json_policy_type=crate::guarded::SecretString",
  ]) assertContains("buf.gen.yaml", option);
  for (const path of [rust, view]) {
    if (!readText(path).startsWith("// @generated by reallyme-buffa-codegen 0.1.1.")) fail("Rust output must come from the native pinned generator");
    assertNotContains(path, "__buffa_unknown_fields");
    assertNotContains(path, "__reallyme_");
  }
  assertContains("crates/proto/src/generated.rs", "mod raw_proto;");
  assertNotContains("crates/proto/src/generated.rs", "pub mod raw_proto;");
  for (const message of [...readText(schema).matchAll(/^message\s+(Codec\w+)\s*\{/gmu)].map((match) => match[1])) {
    assertContains(rust, `impl ::core::ops::Drop for ${message} {`);
    assertContains(rust, `impl ::buffa::sensitive::ZeroizeOnDrop for ${message} {}`);
  }
  const borrowedSelectors = [...readText("buf.gen.yaml").matchAll(
    /^\s*- borrowed_views_only_in=\.reallyme\.codec\.v1\.(Codec\w+)\s*$/gmu,
  )].map((match) => match[1]);
  if (borrowedSelectors.length !== sensitiveMessages.length ||
      new Set(borrowedSelectors).size !== sensitiveMessages.length ||
      sensitiveMessages.some((message) => !borrowedSelectors.includes(message))) {
    fail("Buffa borrowed-only selectors must match every classified sensitive message");
  }
  const generatedViews = readText(view);
  for (const message of sensitiveMessages) {
    assertNotContains(exports, `${message}OwnedView`);
    if (generatedViews.includes(`struct ${message}OwnedView(`) ||
        generatedViews.includes(`impl ::buffa::HasMessageView for super::super::${message} {`)) {
      fail(`generated sensitive message ${message} exposes a standard owned view`);
    }
    assertContains(view, `impl<'a> ::core::fmt::Debug for ${message}View<'a>`);
    assertContains("gen/swift/reallyme/codec/v1/codec.pb.swift", `ReallyMeProto${message}(<redacted>)`);
    assertContains(`gen/java/me/really/codec/v1/${message}.java`, `return "${message}{<redacted>}";`);
    assertContains(`gen/java/me/really/codec/v1/${message}.java`, "return 0x524d;");
  }
  for (const needle of [".reject_unknown_fields()", ".with_recursion_limit(", ".with_element_memory_limit("]) {
    assertContains("crates/proto/src/wire.rs", needle);
  }
  assertContains("crates/proto/src/guarded/decode.rs", "fn decode_length_delimited(payload: &mut FieldPayload<impl Buf>)");
  assertContains("crates/proto/src/guarded/json_bytes.rs", "impl ProtoBytesJson for SecretBytes");
  assertContains("crates/proto/src/guarded/json_string.rs", "impl ProtoStringJson for SecretString");
  assertContains("crates/proto/src/guarded/decode.rs", "try_reserve_exact(payload.remaining())");
  assertContains(rust, "::buffa::sensitive::List<");
  // Standalone generated enums must use the guarded runtime visitor.
  assertContains(rust, "::buffa::json_helpers::closed_enum::deserialize_strict(d)");
  for (const path of [rust, view]) assertNotContains(path, "::buffa::alloc::format!(");
  if (/pub\s+\w+\s*:\s*(?:::buffa::alloc::)?(?:vec::Vec\s*<\s*u8\s*>|string::String)/u.test(readText(rust))) {
    fail("native generated bytes/string fields must have guarded owners");
  }
  // The classifier is closed-world and these artifacts never modify raw Rust.
  run(process.execPath, ["scripts/generate_codec_proto_support.mjs", "--check"]);
  run(process.execPath, ["scripts/generate_codec_sdk_contract.mjs", "--check"]);
  if (generatedFreshnessMode) {
    assertGeneratedArtifactsFresh({
      generatedPaths: [
        "crates/proto/src/generated", "packages/ts/src/proto/generated", "gen",
        "packages/swift/Sources/ReallyMeCodec/Generated",
        "packages/kotlin/src/main/kotlin/me/really/codec/generated",
      ],
      commands: [["node", ["scripts/generate_codec_proto.mjs"]]],
    });
  }
}
