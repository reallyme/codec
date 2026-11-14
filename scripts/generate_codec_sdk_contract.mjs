#!/usr/bin/env node
// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fail, findMatchingBrace, protoSource, root, scrubProtoCommentsAndStrings } from "./codec_proto_schema.mjs";

const check = process.argv.length === 3 && process.argv[2] === "--check";
if (process.argv.length !== 2 && !check) fail("use --check or no arguments");
const header = `// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

// Generated from reallyme/codec/v1/codec.proto. Do not edit.
`;
const schema = scrubProtoCommentsAndStrings(protoSource);
const words = (value) => value.toLowerCase().split("_");
const upperCamel = (value) => words(value).map((word) => word[0].toUpperCase() + word.slice(1)).join("");
const lowerCamel = (value) => {
  const name = upperCamel(value);
  return name[0].toLowerCase() + name.slice(1);
};
const kebab = (value) => words(value).join("-");
const quote = JSON.stringify;

function members(name, prefix) {
  const declaration = new RegExp(`^enum ${name}\\s*\\{`, "mu").exec(schema);
  if (!declaration) fail(`missing SDK enum ${name}`);
  const open = declaration.index + declaration[0].length - 1;
  const close = findMatchingBrace(schema, open);
  const body = protoSource.slice(open + 1, close);
  const fields = [...body.matchAll(/((?:^[ \t]*\/\/[^\n]*\n)*)^[ \t]*([A-Z][A-Z0-9_]*)\s*=\s*(\d+)\s*;/gmu)];
  if (fields.length < 2 || fields[0][2] !== `${prefix}UNSPECIFIED` || fields[0][3] !== "0") fail(`invalid SDK enum ${name}`);
  const values = fields.slice(1).map(([, metadata, identifier, number]) => {
    if (!identifier.startsWith(prefix) || number === "0") fail(`invalid SDK enum member in ${name}`);
    const order = /sdk-order: (\d+)/u.exec(metadata)?.[1];
    return { token: identifier.slice(prefix.length), identifier, metadata, order: order === undefined ? undefined : Number(order) };
  });
  if (values.some((value) => value.order !== undefined)) {
    const orders = new Set(values.map((value) => value.order));
    if (orders.size !== values.length || values.some(({ order }) => !Number.isSafeInteger(order) || order < 0 || order >= values.length)) {
      fail(`incomplete SDK presentation order for ${name}`);
    }
    values.sort((left, right) => left.order - right.order);
  }
  return values;
}

const projections = [
  { proto: "CodecTag", sdk: "ReallyMeMulticodecTag", prefix: "CODEC_TAG_", swift: "sdkMulticodecTag", ts: "projectMulticodecTag", parameter: "tag" },
  { proto: "CodecKeyMaterialKind", sdk: "ReallyMeKeyMaterialKind", prefix: "CODEC_KEY_MATERIAL_KIND_", swift: "sdkKeyMaterialKind", ts: "projectKeyMaterialKind", parameter: "kind" },
  { proto: "CodecPemLabel", sdk: "ReallyMePemLabel", prefix: "CODEC_PEM_LABEL_", string: "label", text: (token) => token.replaceAll("_", " ") },
  { proto: "CodecPemLineEnding", sdk: "ReallyMePemLineEnding", prefix: "CODEC_PEM_LINE_ENDING_", string: "lineEnding", text: (token) => token.toLowerCase() },
].map((projection) => ({ ...projection, values: members(projection.proto, projection.prefix) }));
const categories = members("CodecSdkErrorCategory", "CODEC_SDK_ERROR_CATEGORY_").map((value) => {
  const targets = /sdk: (swift(?:,typescript,kotlin)?)(?:;|\s*$)/u.exec(value.metadata)?.[1].split(",");
  if (targets === undefined) fail(`missing SDK targets for ${value.identifier}`);
  const parent = /kotlin-parent: ([A-Z0-9_]+)/u.exec(value.metadata)?.[1];
  const base = /kotlin-base: true/u.test(value.metadata);
  if ((base || parent !== undefined) && !targets.includes("kotlin")) fail("invalid Kotlin error inheritance metadata");
  return { ...value, targets, parent, base };
});
const kotlinErrors = categories.filter(({ targets }) => targets.includes("kotlin"));
for (const category of kotlinErrors) {
  if (category.parent !== undefined && !kotlinErrors.some((parent) => parent.token === category.parent && parent.base && parent.parent === undefined)) {
    fail("SDK error category has no open Kotlin parent");
  }
}

function swiftProjection({ proto, sdk, values, swift, parameter }) {
  const isLabel = proto === "CodecPemLabel";
  const cases = values.map(({ token }) => `    case ${lowerCamel(token)}${isLabel ? ` = ${quote(token.replaceAll("_", " "))}` : ""}`).join("\n");
  let source = `public enum ${sdk}: ${isLabel ? "String, " : ""}Sendable {\n${cases}\n}\n`;
  if (swift !== undefined) {
    source += `\nfunc ${swift}(from ${parameter}: ReallyMeProto${proto}) throws(ReallyMeCodecError) -> ${sdk} {\n    switch ${parameter} {\n${values.map(({ token }) => `    case .${lowerCamel(token)}: return .${lowerCamel(token)}`).join("\n")}\n    case .unspecified, .UNRECOGNIZED: throw .providerFailure\n    }\n}\n`;
  } else {
    const func = isLabel ? "protoPemLabel" : "protoPemLineEnding";
    source += `\nfunc ${func}(from value: ${sdk}${isLabel ? "" : "?"}) -> ReallyMeProto${proto} {\n    switch value {\n${values.map(({ token }) => `    case .${lowerCamel(token)}: return .${lowerCamel(token)}`).join("\n")}\n${isLabel ? "" : "    case nil: return .unspecified\n"}    }\n}\n`;
  }
  return source;
}
function kotlinProjection({ proto, sdk, values, swift, parameter }) {
  const isLabel = proto === "CodecPemLabel";
  const cases = values.map(({ token }) => `    ${token}${isLabel ? `(${quote(token.replaceAll("_", " "))})` : ""},`).join("\n");
  let source = `public enum class ${sdk}${isLabel ? "(public val label: String)" : ""} {\n${cases}\n}\n`;
  if (swift !== undefined) {
    source += `\ninternal fun ${swift}(${parameter}: ${proto}): ${sdk} = when (${parameter}) {\n${values.map(({ token, identifier }) => `    ${proto}.${identifier} -> ${sdk}.${token}`).join("\n")}\n    ${proto}.${values[0].identifier.slice(0, -values[0].token.length)}UNSPECIFIED,\n    ${proto}.UNRECOGNIZED,\n    -> throw ReallyMeCodecException.ProviderFailure()\n}\n`;
  } else {
    source += `\ninternal fun ${isLabel ? "protoPemLabel" : "protoPemLineEnding"}(value: ${sdk}${isLabel ? "" : "?"}): ${proto} = when (value) {\n${values.map(({token, identifier}) => `    ${sdk}.${token} -> ${proto}.${identifier}`).join("\n")}\n${isLabel ? "" : `    null -> ${proto}.CODEC_PEM_LINE_ENDING_UNSPECIFIED\n`} }\n`;
    if (isLabel) source += `\ninternal fun sdkPemLabel(label: String): ${sdk} =\n    ${sdk}.entries.firstOrNull { it.label == label } ?: throw ReallyMeCodecException.ProviderFailure()\n`;
  }
  return source;
}
function typescriptProjection({ proto, sdk, values, ts, string, text }) {
  if (string !== undefined) {
    const map = `${string}ToProto`;
    const capital = string[0].toUpperCase() + string.slice(1);
    return `export const ${map} = {\n${values.map(({ token }) => `  ${quote(text(token))}: ${proto}.${token},`).join("\n")}\n} as const;\nexport type ${sdk} = keyof typeof ${map};\n\nexport function isPem${capital}(value: unknown): value is ${sdk} {\n  return typeof value === "string" && Object.hasOwn(${map}, value);\n}\n`;
  }
  const map = `${ts}Values`;
  return `const ${map} = {\n${values.map(({ token }) => `  [${proto}.${token}]: ${quote(kebab(token))},`).join("\n")}\n} as const;\nexport type ${sdk} = (typeof ${map})[keyof typeof ${map}];\n\nexport function ${ts}(value: ${proto}): ${sdk} | undefined {\n  switch (value) {\n${values.map(({ token }) => `    case ${proto}.${token}: return ${map}[value];`).join("\n")}\n    default: return undefined;\n  }\n}\n`;
}

const swiftErrors = `/// Typed SDK failures contain no raw input or backend diagnostics.\npublic enum ReallyMeCodecError: Error, Equatable, Sendable {\n${categories.filter(({targets}) => targets.includes("swift")).map(({ token }) => `    case ${lowerCamel(token)}`).join("\n")}\n}\n`;
const kotlinErrorClasses = kotlinErrors.map(({ token, identifier, parent, base }) =>
  `    public ${base ? "open " : ""}class ${upperCamel(token)} : ${parent === undefined ? `ReallyMeCodecException(CodecSdkErrorCategory.${identifier})` : `${upperCamel(parent)}()`}`,
).join("\n\n");
const kotlinMessages = kotlinErrors.filter(({parent}) => parent === undefined).map(({token, identifier}) =>
  `        CodecSdkErrorCategory.${identifier} -> ${quote(words(token).join(" "))}`,
).join("\n");
const kotlinErrorsSource = `import me.really.codec.v1.CodecSdkErrorCategory\n\n/** Typed SDK failures contain no raw input or backend diagnostics. */\npublic sealed class ReallyMeCodecException private constructor(category: CodecSdkErrorCategory) :\n    RuntimeException(codecSdkErrorMessage(category)) {\n${kotlinErrorClasses}\n}\n\nprivate fun codecSdkErrorMessage(category: CodecSdkErrorCategory): String =\n    when (category) {\n${kotlinMessages}\n        else -> "provider failure"\n    }\n`;
const typescriptErrors = `const sdkErrorCodes = {\n${categories.filter(({ targets }) => targets.includes("typescript")).map(({ token }) => `  [CodecSdkErrorCategory.${token}]: ${quote(kebab(token))},`).join("\n")}\n} as const;\nexport type ReallyMeCodecErrorCode = (typeof sdkErrorCodes)[keyof typeof sdkErrorCodes];\n\nconst codeValues: ReadonlySet<unknown> = new Set(Object.values(sdkErrorCodes));\nexport function isReallyMeCodecErrorCode(value: unknown): value is ReallyMeCodecErrorCode {\n  return codeValues.has(value);\n}\n`;
const protoImports = projections.map(({ proto }) => proto).sort();
const outputs = new Map([
  ["packages/swift/Sources/ReallyMeCodec/Generated/CodecSdkContract.swift", `${swiftErrors}\n${projections.map(swiftProjection).join("\n")}`],
  ["packages/kotlin/src/main/kotlin/me/really/codec/generated/CodecSdkEnums.kt", `package me.really.codec\n\n${protoImports.map((name) => `import me.really.codec.v1.${name}`).join("\n")}\n\n${projections.map(kotlinProjection).join("\n")}`],
  ["packages/kotlin/src/main/kotlin/me/really/codec/generated/ReallyMeCodecException.kt", `package me.really.codec\n\n${kotlinErrorsSource}`],
  ["packages/ts/src/proto/generated/sdkContract.ts", `import {\n  ${[...protoImports, "CodecSdkErrorCategory"].sort().join(",\n  ")},\n} from "./reallyme/codec/v1/codec_pb.js";\n\n${projections.map(typescriptProjection).join("\n")}\n${typescriptErrors}`],
]);
for (const [relative, body] of outputs) {
  const path = join(root, relative);
  const source = `${header}\n${body}`;
  if (check) {
    if (readFileSync(path, "utf8") !== source) fail(`stale SDK projection ${relative}`);
  } else {
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, source);
  }
}
