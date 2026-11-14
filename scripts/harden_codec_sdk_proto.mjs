#!/usr/bin/env node
// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { mkdirSync, readFileSync, readdirSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { join } from "node:path";
import { codecProtoProviderOutputMessages } from "./codec_proto_sensitivity.mjs";
import { root, protoSource, sensitiveFieldNames, sensitiveMessageNames,
  sensitiveOwnerMessageNames, snakeToLowerCamel, fail, validateScalarFieldClassifications,
} from "./codec_proto_schema.mjs";
function countOccurrences(text, needle) { return text.split(needle).length - 1; }
function replaceOnce(text, before, after, path) {
  const next = text.replace(before, after);
  if (next === text) {
    fail(`${path} did not contain expected generated fragment: ${before}`);
  }
  return next;
}


function replaceAllRequired(text, before, after, path) {
  const replacements = text.split(before).length - 1;
  const next = replacements === 0 ? text : text.replaceAll(before, after);
  if (!next.includes(after)) {
    fail(`${path} contained neither the generated nor hardened fragment: ${before}`);
  }
  return next;
}


function redactSwiftDebug() {
  const filePath = join(root, "gen/swift/reallyme/codec/v1/codec.pb.swift");
  let source = readFileSync(filePath, "utf8");
  for (const messageName of sensitiveMessageNames) {
    const swiftName = `ReallyMeProto${messageName}`;
    // SwiftProtobuf uses its own copy-on-write storage for recursive or large
    // messages. Preserve that generated conformance while adding redaction.
    const generatedUnchecked =
      source.includes(`nonisolated struct ${swiftName}: @unchecked Sendable`);
    const sendability = generatedUnchecked ? "@unchecked Sendable" : "Sendable";
    const originalDeclaration = `nonisolated struct ${swiftName}: ${sendability} {`;
    const declaration = `nonisolated struct ${swiftName}: ${sendability}, CustomReflectable {`;
    if (countOccurrences(source, originalDeclaration) === 1) {
      source = source.replace(originalDeclaration, declaration);
    }
    if (countOccurrences(source, declaration) !== 1) {
      fail(`generated Swift message ${swiftName} must have exactly one declaration`);
    }
    const additions = [];
    const mirrorComment = `// Mirror for ${swiftName} must not expose sensitive fields.`;
    if (!source.includes(mirrorComment)) {
      additions.push(`  ${mirrorComment}
  var customMirror: Mirror { Mirror(self, children: []) }`);
    }
    if (!source.includes(`var debugDescription: String { "${swiftName}(<redacted>)" }`)) {
      additions.push(`  // Security post-processing: protobuf fields can contain secrets or PII.
  var debugDescription: String { "${swiftName}(<redacted>)" }

  func hash(into hasher: inout Hasher) {
    hasher.combine("${swiftName}(<redacted>)")
  }`);
    }
    if (!source.includes(`func textFormatString() -> String { "${swiftName}(<redacted>)" }`)) {
      additions.push(`  // Concrete calls use redacted text; SwiftProtobuf's generic Message
  // extension still traverses fields, so generic text-format output is unsafe.
  func textFormatString() -> String { "${swiftName}(<redacted>)" }

  func textFormatString(
    options _: SwiftProtobuf.TextFormatEncodingOptions
  ) -> String { "${swiftName}(<redacted>)" }`);
    }
    if (additions.length !== 0) {
      source = source.replace(declaration, `${declaration}\n${additions.join("\n\n")}`);
    }
    for (const required of [
      `var debugDescription: String { "${swiftName}(<redacted>)" }`,
      mirrorComment,
      `func textFormatString() -> String { "${swiftName}(<redacted>)" }`,
      `) -> String { "${swiftName}(<redacted>)" }`,
    ]) {
      if (!source.includes(required)) {
        fail(`generated Swift message ${swiftName} is missing ${required}`);
      }
    }
  }
  // SwiftProtobuf formats through a generic Message extension. Public message
  // types would expose raw fields even when their concrete formatter is redacted.
  // The Swift SDK therefore keeps generated wire types internal to its module.
  writeFileSync(filePath, source);
  // SwiftPM compiles sources from one target directory. The generated wire
  // code is internal in both locations, so it cannot be imported separately.
  mkdirSync(join(root, "packages/swift/Sources/ReallyMeCodec/Generated"), { recursive: true });
  writeFileSync(
    join(root, "packages/swift/Sources/ReallyMeCodec/Generated/CodecProto.swift"),
    source,
  );
}


function redactJavaDebug() {
  for (const messageName of sensitiveMessageNames) {
    const filePath = join(root, `gen/java/me/really/codec/v1/${messageName}.java`);
    let source = readFileSync(filePath, "utf8");
    const declaration = new RegExp(`public\\s+final\\s+class\\s+${messageName}\\s+extends`, "gu");
    if ([...source.matchAll(declaration)].length !== 1) {
      fail(`generated Java message ${messageName} must have exactly one declaration`);
    }
    if (!source.includes(`${messageName}{<redacted>}`)) {
      const declarationMatch = new RegExp(
        `public\\s+final\\s+class\\s+${messageName}\\s+extends`,
        "u",
      ).exec(source);
      const declarationStart = declarationMatch?.index ?? -1;
      const bodyStart = source.indexOf("{", declarationStart);
      if (declarationStart < 0 || bodyStart < 0) {
        throw new Error(`unable to locate generated Java message ${messageName}`);
      }
      const redaction = `
  // Security post-processing: protobuf fields can contain secrets or PII.
  @java.lang.Override
  public java.lang.String toString() {
    return "${messageName}{<redacted>}";
  }

  @java.lang.Override
  public int hashCode() {
    return 0x524d;
  }
`;
      source = `${source.slice(0, bodyStart + 1)}${redaction}${source.slice(bodyStart + 1)}`;
    }
    source = ensureJavaUnknownFieldInspection(source, messageName).replace(
      "skew or corrupt-provider additions before mapping provider output.",
      "skew or corrupt-provider additions before copying a sensitive owner tree.",
    );
    if (
      countOccurrences(source, `return "${messageName}{<redacted>}";`) !== 1 ||
      countOccurrences(source, "return 0x524d;") < 1 ||
      countOccurrences(
        source,
        "public boolean reallyMeHasUnknownFieldsForValidation()",
      ) !== 1
    ) {
      fail(`generated Java message ${messageName} is missing redaction overrides`);
    }
    // The Java lite generator currently emits unstable whitespace around the
    // bytes-field setter. Normalize it so checked-in generation is byte-for-
    // byte reproducible across clean CI and local regeneration.
    source = source.replace(
      "   */\n  private void setPem(com.google.protobuf.ByteString value)",
      "  */\n  private void setPem(com.google.protobuf.ByteString value)",
    );
    source = source.replace("value.getClass();\n  \n    pem_", "value.getClass();\n\n    pem_");
    const trailingNewlines = messageName === "CodecPemDecodeRequest" ? "\n" : "\n\n";
    source = source.replace(/\n+$/u, trailingNewlines);
    writeFileSync(filePath, source);
  }

  for (const messageName of codecProtoProviderOutputMessages) {
    const filePath = join(
      root,
      `gen/java/me/really/codec/v1/${messageName}.java`,
    );
    const source = ensureJavaUnknownFieldInspection(
      readFileSync(filePath, "utf8"),
      messageName,
    );
    writeFileSync(filePath, source);
  }
}


function ensureJavaUnknownFieldInspection(source, messageName) {
  let hardened = source;
  if (
    !hardened.includes("public boolean reallyMeHasUnknownFieldsForValidation()")
  ) {
    const constructor = `  private ${messageName}()`;
    const constructorStart = hardened.indexOf(constructor);
    if (constructorStart < 0) {
      fail(`unable to locate generated Java constructor for ${messageName}`);
    }
    const unknownFieldInspection = `  // Java Lite deliberately omits public unknown-field access. This generated
  // boolean exposes no field content, but lets SDK adapters reject schema
  // skew or corrupt-provider additions before mapping provider output.
  public boolean reallyMeHasUnknownFieldsForValidation() {
    return unknownFields != com.google.protobuf.UnknownFieldSetLite.getDefaultInstance();
  }

`;
    hardened = `${hardened.slice(0, constructorStart)}${unknownFieldInspection}${hardened.slice(constructorStart)}`;
  }
  if (
    countOccurrences(
      hardened,
      "public boolean reallyMeHasUnknownFieldsForValidation()",
    ) !== 1
  ) {
    fail(
      `generated Java message ${messageName} has an invalid unknown-field predicate`,
    );
  }
  return hardened;
}


function normalizeGeneratedWhitespace(directoryPath, extension) {
  for (const fileName of readdirSync(directoryPath)) {
    if (!fileName.endsWith(extension)) {
      continue;
    }
    const filePath = join(directoryPath, fileName);
    const source = readFileSync(filePath, "utf8");
    const normalized = source.replace(/[ \t]+$/gmu, "").replace(/\n+$/u, "\n");
    writeFileSync(filePath, normalized);
  }
}

// Preserve fixed native error diagnostics using metadata owned by the schema.
// Buffa owns the enum definitions, numeric values, and idiomatic aliases.

const checkIdempotent = process.argv.length === 3 && process.argv[2] === "--check-idempotent";
if (process.argv.length !== 2 && !checkIdempotent) fail("use --check-idempotent or no arguments");
const outputPaths = [
  join(root, "gen/swift/reallyme/codec/v1/codec.pb.swift"),
  join(root, "packages/swift/Sources/ReallyMeCodec/Generated/CodecProto.swift"),
  ...["gen/java/me/really/codec/v1", "gen/kotlin/me/really/codec/v1", "gen/es/reallyme/codec/v1", "packages/ts/src/proto/generated/reallyme/codec/v1"].flatMap((relative) => readdirSync(join(root, relative)).map((file) => join(root, relative, file))),
];
const before = checkIdempotent ? new Map(outputPaths.map((path) => [path, readFileSync(path)])) : undefined;
validateScalarFieldClassifications();
redactSwiftDebug();
redactJavaDebug();
for (const [path, extension] of [["gen/java/me/really/codec/v1", ".java"], ["gen/kotlin/me/really/codec/v1", ".kt"], ["gen/es/reallyme/codec/v1", ".ts"], ["packages/ts/src/proto/generated/reallyme/codec/v1", ".ts"]]) normalizeGeneratedWhitespace(join(root, path), extension);
if (before !== undefined) for (const [path, bytes] of before) if (!bytes.equals(readFileSync(path))) fail("SDK generation safeguards are not idempotent");
