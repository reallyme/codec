#!/usr/bin/env node
// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import {
  codecProtoScalarFieldClassifications,
  codecProtoSensitiveOwnerMessages,
} from "./codec_proto_sensitivity.mjs";

const root = new URL("..", import.meta.url);

const protoPath = join(
  root.pathname,
  "crates/proto/codec/proto/reallyme/codec/v1/codec.proto",
);
const rustGeneratedPath = join(
  root.pathname,
  "crates/proto/codec/src/generated/buffa/reallyme.codec.v1.codec.rs",
);
const rustGeneratedViewPath = join(
  root.pathname,
  "crates/proto/codec/src/generated/buffa/reallyme.codec.v1.codec.__view.rs",
);
const rustGeneratedModulePath = join(
  root.pathname,
  "crates/proto/codec/src/generated/buffa/reallyme.codec.v1.mod.rs",
);
const protoSource = readFileSync(protoPath, "utf8");
const supportedArguments = new Set(["--check-idempotent"]);
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
const checkIdempotent = suppliedArguments.has("--check-idempotent");

const sensitiveFieldNames = new Map();
for (const classification of codecProtoScalarFieldClassifications) {
  if (classification.sensitivity !== "sensitive") {
    continue;
  }
  const fields = sensitiveFieldNames.get(classification.message) ?? [];
  fields.push({ name: classification.field, kind: classification.kind });
  sensitiveFieldNames.set(classification.message, fields);
}
for (const messageName of codecProtoSensitiveOwnerMessages) {
  if (!sensitiveFieldNames.has(messageName)) {
    sensitiveFieldNames.set(messageName, []);
  }
}
const sensitiveMessageNames = [...sensitiveFieldNames.keys()];
const sensitiveOwnedViewMessageNames = [...sensitiveMessageNames];
function fail(message) {
  console.error(`generated Codec proto hardening failed: ${message}`);
  process.exit(1);
}

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

function countOccurrences(text, needle) {
  return text.split(needle).length - 1;
}

function scrubProtoCommentsAndStrings(source) {
  let output = "";
  let state = "normal";
  for (let index = 0; index < source.length; index += 1) {
    const character = source[index];
    const next = source[index + 1];
    if (state === "normal") {
      if (character === "/" && next === "/") {
        output += "  ";
        index += 1;
        state = "line-comment";
      } else if (character === "/" && next === "*") {
        output += "  ";
        index += 1;
        state = "block-comment";
      } else if (character === '"') {
        output += " ";
        state = "string";
      } else {
        output += character;
      }
      continue;
    }
    if (state === "line-comment") {
      if (character === "\n") {
        output += "\n";
        state = "normal";
      } else {
        output += " ";
      }
      continue;
    }
    if (state === "block-comment") {
      if (character === "*" && next === "/") {
        output += "  ";
        index += 1;
        state = "normal";
      } else {
        output += character === "\n" ? "\n" : " ";
      }
      continue;
    }
    if (character === "\\" && next !== undefined) {
      output += next === "\n" ? " \n" : "  ";
      index += 1;
    } else if (character === '"') {
      output += " ";
      state = "normal";
    } else {
      output += character === "\n" ? "\n" : " ";
    }
  }
  return output;
}

function generatedRustMessageRegion(source, messageName) {
  const startNeedle = `pub struct ${messageName} {`;
  const start = source.indexOf(startNeedle);
  if (start < 0) {
    fail(`missing generated Rust message ${messageName}`);
  }
  const remainder = source.slice(start + startNeedle.length);
  const nextMessage = /^pub struct [A-Z][A-Za-z0-9]* \{/gmu.exec(remainder);
  const end = nextMessage === null
    ? source.length
    : start + startNeedle.length + nextMessage.index;
  return source.slice(start, end);
}

function validateSensitiveRustHardening(source) {
  for (const [messageName, fields] of sensitiveFieldNames) {
    const region = generatedRustMessageRegion(source, messageName);
    for (const field of fields) {
      const zeroizeNeedle =
        `::zeroize::Zeroize::zeroize(&mut self.${field.name});`;
      // One wipe belongs to the generated clear/merge path and one to Drop.
      // Requiring both prevents the Drop implementation from masking a failed
      // rewrite when generator formatting changes.
      if (countOccurrences(region, zeroizeNeedle) < 2) {
        fail(`${messageName}.${field.name} is missing a generated-path or Drop wipe`);
      }
      if (region.includes(`self.${field.name}.clear();`)) {
        fail(`${messageName}.${field.name} still uses a non-zeroizing clear path`);
      }
    }
    const unknownWipe =
      "__reallyme_zeroize_unknown_fields(&mut self.__buffa_unknown_fields);";
    if (countOccurrences(region, unknownWipe) < 2) {
      fail(`${messageName} is missing a generated-path or Drop unknown-field wipe`);
    }
    if (region.includes("self.__buffa_unknown_fields.clear();")) {
      fail(`${messageName} still clears unknown fields without zeroizing them`);
    }
  }
  if (source.includes("serde::de::IgnoredAny")) {
    fail(`${rustGeneratedPath} still accepts ignored ProtoJSON fields`);
  }
}

function validateScalarFieldClassifications() {
  const structuralProtoSource = scrubProtoCommentsAndStrings(protoSource);
  const schemaFields = [];
  const schemaMessageNames = new Set();
  const messagePattern = /^message\s+([A-Z][A-Za-z0-9]*)\s*\{/gmu;
  for (const messageMatch of structuralProtoSource.matchAll(messagePattern)) {
    const message = messageMatch[1];
    schemaMessageNames.add(message);
    const openIndex = messageMatch.index + messageMatch[0].lastIndexOf("{");
    const closeIndex = findMatchingBrace(structuralProtoSource, openIndex);
    const body = structuralProtoSource.slice(openIndex + 1, closeIndex);
    if (/^\s+message\s+[A-Z][A-Za-z0-9]*\s*\{/mu.test(body)) {
      fail(`nested protobuf messages require an explicit classifier extension in ${message}`);
    }
    if (/\bmap\s*<[^>]*(?:bytes|string)[^>]*>/u.test(body)) {
      fail(`protobuf maps with bytes/string members require an explicit classifier extension in ${message}`);
    }
    const scalarFieldPattern =
      /(?:^|[;{}])\s*(?:(?:optional|required|repeated)\s+)?(bytes|string)\s+([a-z][a-z0-9_]*)\s*=\s*\d+(?:\s*\[[^\]]*\])?\s*(?=;)/gmu;
    for (const fieldMatch of body.matchAll(scalarFieldPattern)) {
      schemaFields.push({ message, kind: fieldMatch[1], field: fieldMatch[2] });
    }
  }
  const classificationKeys = new Set();
  for (const classification of codecProtoScalarFieldClassifications) {
    if (
      classification === null ||
      typeof classification !== "object" ||
      !/^[A-Z][A-Za-z0-9]*$/u.test(classification.message) ||
      !/^[a-z][a-z0-9_]*$/u.test(classification.field) ||
      !["bytes", "string"].includes(classification.kind) ||
      !["public", "sensitive"].includes(classification.sensitivity)
    ) {
      fail("invalid protobuf scalar sensitivity classification");
    }
    const key = `${classification.message}.${classification.field}:${classification.kind}`;
    if (classificationKeys.has(key)) {
      fail(`duplicate protobuf scalar sensitivity classification ${key}`);
    }
    classificationKeys.add(key);
  }
  const schemaKeys = new Set(
    schemaFields.map((field) => `${field.message}.${field.field}:${field.kind}`),
  );
  for (const key of schemaKeys) {
    if (!classificationKeys.has(key)) {
      fail(`unclassified protobuf bytes/string field ${key}`);
    }
  }
  for (const key of classificationKeys) {
    if (!schemaKeys.has(key)) {
      fail(`stale protobuf scalar sensitivity classification ${key}`);
    }
  }
  for (const messageName of codecProtoSensitiveOwnerMessages) {
    if (!schemaMessageNames.has(messageName)) {
      fail(`sensitive protobuf owner ${messageName} does not name a schema message`);
    }
  }
}

function escapeRegExp(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
}

function findMatchingBrace(source, openIndex) {
  let depth = 0;
  for (let index = openIndex; index < source.length; index += 1) {
    const char = source[index];
    if (char === "{") {
      depth += 1;
    } else if (char === "}") {
      depth -= 1;
      if (depth === 0) {
        return index;
      }
    }
  }
  fail(`missing matching brace after byte offset ${openIndex}`);
}

function redactOwnedRustDebugAndMemory() {
  let source = readFileSync(rustGeneratedPath, "utf8");
  source = hardenUnknownFields(source);
  for (const [messageName, fields] of sensitiveFieldNames.entries()) {
    for (const fieldName of fields) {
      const name = typeof fieldName === "string" ? fieldName : fieldName.name;
      source = replaceAllRequired(
        source,
        `.field("${name}", &self.${name})`,
        `.field("${name}", &"<redacted>")`,
        rustGeneratedPath,
      );
      source = replaceAllRequired(
        source,
        `        self.${name}.clear();`,
        `        ::zeroize::Zeroize::zeroize(&mut self.${name});`,
        rustGeneratedPath,
      );
    }

    const structMarker = `pub struct ${messageName} {`;
    const structStart = source.indexOf(structMarker);
    if (structStart < 0) {
      fail(`missing generated Rust message ${messageName}`);
    }

    if (fields.length > 0) {
      const serdeDerive = "#[derive(::serde::Serialize, ::serde::Deserialize)]";
      const structHeaderStart = Math.max(0, structStart - 512);
      const structHeader = source.slice(structHeaderStart, structStart);
      const serdeIndex = structHeader.lastIndexOf(serdeDerive);
      if (serdeIndex >= 0) {
        const absoluteSerdeIndex = structHeaderStart + serdeIndex;
        source =
          source.slice(0, absoluteSerdeIndex) +
          "#[derive(::serde::Serialize)]" +
          source.slice(absoluteSerdeIndex + serdeDerive.length);
      } else if (
        !source.includes(`impl<'de> ::serde::Deserialize<'de> for ${messageName}`) &&
        !source.includes(`impl<'de> serde::Deserialize<'de> for ${messageName}`)
      ) {
        fail(`${messageName} is missing generated serde Deserialize derive`);
      }
    }

    source = hardenSensitiveDrop(source, messageName, fields);
    if (fields.length > 0) {
      source = hardenSensitiveDeserialize(source, messageName, fields);
    }
  }
  source = replaceAllRequired(
    source,
    "        self.__buffa_unknown_fields.clear();",
    "        __reallyme_zeroize_unknown_fields(&mut self.__buffa_unknown_fields);",
    rustGeneratedPath,
  );
  source = replaceAllRequired(
    source,
    "#[serde(default)]",
    "#[serde(default, deny_unknown_fields)]",
    rustGeneratedPath,
  );
  source = replaceAllRequired(
    source,
    `                        _ => {
                            map.next_value::<serde::de::IgnoredAny>()?;
                        }`,
    `                        _ => {
                            return Err(serde::de::Error::custom("unknown field"));
                        }`,
    rustGeneratedPath,
  );
  // Buffa's enum visitors otherwise reflect attacker-controlled numeric values
  // into allocated error strings. Fixed diagnostics keep boundary failures
  // deterministic and avoid carrying untrusted input into logs.
  source = source.replaceAll(
    `::serde::de::Error::custom(
                            ::buffa::alloc::format!("enum value {v} out of i32 range"),
                        )`,
    `::serde::de::Error::custom("enum value out of i32 range")`,
  );
  source = source.replaceAll(
    `::serde::de::Error::custom(
                            ::buffa::alloc::format!("unknown enum value {v32}"),
                        )`,
    `::serde::de::Error::custom("unknown enum value")`,
  );
  if (source.includes("::buffa::alloc::format!(")) {
    fail(`${rustGeneratedPath} still contains formatted ProtoJSON errors`);
  }
  validateSensitiveRustHardening(source);
  writeFileSync(rustGeneratedPath, source);
}

function hardenUnknownFields(source) {
  if (source.includes("fn __reallyme_zeroize_unknown_fields(")) {
    return source;
  }
  const generatedHeader = `// @generated by buffa-codegen. DO NOT EDIT.
// source: reallyme/codec/v1/codec.proto
`;
  const helpers = `
fn __reallyme_zeroize_unknown_fields(fields: &mut ::buffa::UnknownFields) {
    for mut field in ::core::mem::take(fields) {
        __reallyme_zeroize_unknown_field_data(&mut field.data);
    }
}

fn __reallyme_zeroize_unknown_field_data(data: &mut ::buffa::UnknownFieldData) {
    match data {
        ::buffa::UnknownFieldData::LengthDelimited(bytes) => {
            ::zeroize::Zeroize::zeroize(bytes);
        }
        ::buffa::UnknownFieldData::Group(fields) => {
            __reallyme_zeroize_unknown_fields(fields);
        }
        ::buffa::UnknownFieldData::Varint(_)
        | ::buffa::UnknownFieldData::Fixed64(_)
        | ::buffa::UnknownFieldData::Fixed32(_) => {}
    }
}
`;
  return replaceOnce(source, generatedHeader, `${generatedHeader}${helpers}`, rustGeneratedPath);
}

function hardenSensitiveDrop(source, messageName, fields) {
  const body = fields
    .map((field) => `        ::zeroize::Zeroize::zeroize(&mut self.${field.name});`)
    .concat(["        __reallyme_zeroize_unknown_fields(&mut self.__buffa_unknown_fields);"])
    .join("\n");
  const securityComment = `// SECURITY: Buffa's generated message contract requires Clone. Every clone
// remains an owning value, and this Drop implementation wipes its sensitive
// fields and unknown-field storage when that owner leaves scope.
`;
  const dropImpl = `${securityComment}impl ::core::ops::Drop for ${messageName} {
    fn drop(&mut self) {
${body}
    }
}
`;
  const existingDropPattern = new RegExp(
    `(?:${escapeRegExp(securityComment)})*impl ::core::ops::Drop for ${messageName} \\{\\n    fn drop\\(&mut self\\) \\{\\n[\\s\\S]*?    \\}\\n\\}\\n`,
    "u",
  );
  if (existingDropPattern.test(source)) {
    return source.replace(existingDropPattern, dropImpl);
  }

  const implMarker = `impl ${messageName} {`;
  const implIndex = source.indexOf(implMarker);
  if (implIndex < 0) {
    fail(`missing generated inherent impl for ${messageName}`);
  }
  return `${source.slice(0, implIndex)}${dropImpl}${source.slice(implIndex)}`;
}

function hardenSensitiveDeserialize(source, messageName, fields) {
  const qualifiedMarker = `impl<'de> ::serde::Deserialize<'de> for ${messageName}`;
  const unqualifiedMarker = `impl<'de> serde::Deserialize<'de> for ${messageName}`;
  if (!source.includes(qualifiedMarker) && !source.includes(unqualifiedMarker)) {
    const implMarker = `impl ${messageName} {`;
    const implIndex = source.indexOf(implMarker);
    if (implIndex < 0) {
      fail(`missing generated inherent impl for ${messageName}`);
    }
    return `${source.slice(0, implIndex)}${sensitiveDeserializeImpl(messageName, fields)}${source.slice(implIndex)}`;
  }

  const implPattern = new RegExp(
    `impl<'de> (?:::)?serde::Deserialize<'de> for ${messageName} \\{\\n[\\s\\S]*?\\n\\}\\nimpl ${messageName} \\{`,
    "u",
  );
  if (!implPattern.test(source)) {
    fail(`unable to replace generated Deserialize impl for ${messageName}`);
  }
  return source.replace(
    implPattern,
    `${sensitiveDeserializeImpl(messageName, fields)}impl ${messageName} {`,
  );
}

function sensitiveDeserializeImpl(messageName, sensitiveFields) {
  const fields = messageFields(messageName);
  const sensitiveNames = new Map(sensitiveFields.map((field) => [field.name, field.kind]));
  const wireFields = fields
    .map((field) => wireFieldDeclaration(field, sensitiveNames.get(field.name)))
    .join("\n");
  const outputFields = fields
    .map((field) => outputFieldInitializer(field, sensitiveNames.get(field.name)))
    .join("\n");

  return `impl<'de> ::serde::Deserialize<'de> for ${messageName} {
    fn deserialize<D>(deserializer: D) -> ::core::result::Result<Self, D::Error>
    where
        D: ::serde::Deserializer<'de>,
    {
        fn deserialize_zeroizing_bytes<'de, D>(
            deserializer: D,
        ) -> ::core::result::Result<::zeroize::Zeroizing<::buffa::alloc::vec::Vec<u8>>, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::buffa::json_helpers::bytes::deserialize(deserializer)
                .map(::zeroize::Zeroizing::new)
        }

        fn deserialize_zeroizing_string<'de, D>(
            deserializer: D,
        ) -> ::core::result::Result<::zeroize::Zeroizing<::buffa::alloc::string::String>, D::Error>
        where
            D: ::serde::Deserializer<'de>,
        {
            ::serde::Deserialize::deserialize(deserializer).map(::zeroize::Zeroizing::new)
        }

        #[derive(Default, ::serde::Deserialize)]
        #[serde(default, deny_unknown_fields)]
        struct Wire {
${wireFields}
        }

        let mut wire = Wire::deserialize(deserializer)?;
        Ok(Self {
${outputFields}
            __buffa_unknown_fields: Default::default(),
        })
    }
}
`;
}

function messageFields(messageName) {
  const structuralProtoSource = scrubProtoCommentsAndStrings(protoSource);
  const messagePattern = new RegExp(`message\\s+${messageName}\\s*\\{`, "u");
  const match = messagePattern.exec(structuralProtoSource);
  if (!match) {
    fail(`missing proto message ${messageName}`);
  }
  const openIndex = match.index + match[0].lastIndexOf("{");
  const closeIndex = findMatchingBrace(structuralProtoSource, openIndex);
  const body = structuralProtoSource.slice(openIndex + 1, closeIndex);
  const fieldPattern =
    /(?:^|[;{}])\s*(?:(optional|required|repeated)\s+)?(\w+)\s+(\w+)\s*=\s*\d+(?:\s*\[[^\]]*\])?\s*(?=;)/gmu;
  return [...body.matchAll(fieldPattern)].map(
    ([, label, typeName, name]) => ({
      repeated: label === "repeated",
      typeName,
      name,
      jsonName: snakeToLowerCamel(name),
    }),
  );
}

function wireFieldDeclaration(field, sensitiveKind) {
  const serdeParts = [`rename = "${field.jsonName}"`];
  if (field.jsonName !== field.name) {
    serdeParts.push(`alias = "${field.name}"`);
  }
  const rustType = rustFieldType(field, sensitiveKind);
  if (sensitiveKind === "bytes") {
    serdeParts.push("deserialize_with = \"deserialize_zeroizing_bytes\"");
  } else if (sensitiveKind === "string") {
    serdeParts.push("deserialize_with = \"deserialize_zeroizing_string\"");
  } else if (field.typeName.startsWith("Codec") && field.typeName !== "CodecPemDecodeOptions") {
    serdeParts.push(`with = "::buffa::json_helpers::proto_enum"`);
  }
  return `            #[serde(${serdeParts.join(", ")})]
            ${field.name}: ${rustType},`;
}

function rustFieldType(field, sensitiveKind) {
  if (sensitiveKind === "bytes") {
    return "::zeroize::Zeroizing<::buffa::alloc::vec::Vec<u8>>";
  }
  if (sensitiveKind === "string") {
    return "::zeroize::Zeroizing<::buffa::alloc::string::String>";
  }
  if (field.repeated && field.typeName.startsWith("Codec")) {
    return `::buffa::alloc::vec::Vec<::buffa::EnumValue<${field.typeName}>>`;
  }
  if (field.typeName.startsWith("Codec") && field.typeName !== "CodecPemDecodeOptions") {
    return `::buffa::EnumValue<${field.typeName}>`;
  }
  if (field.typeName === "CodecPemDecodeOptions") {
    return "::buffa::MessageField<CodecPemDecodeOptions>";
  }
  if (field.typeName === "string") {
    return "::buffa::alloc::string::String";
  }
  if (field.typeName === "bytes") {
    return "::buffa::alloc::vec::Vec<u8>";
  }
  if (field.typeName === "uint32") {
    return "u32";
  }
  if (field.typeName === "bool") {
    return "bool";
  }
  fail(`unsupported generated field type ${field.typeName} on ${field.name}`);
}

function outputFieldInitializer(field, sensitiveKind) {
  if (sensitiveKind === "bytes" || sensitiveKind === "string") {
    return `            ${field.name}: ::core::mem::take(&mut *wire.${field.name}),`;
  }
  if (field.typeName === "CodecPemDecodeOptions") {
    return `            ${field.name}: ::core::mem::take(&mut wire.${field.name}),`;
  }
  if (field.typeName === "string" || field.typeName === "bytes" || field.repeated) {
    return `            ${field.name}: ::core::mem::take(&mut wire.${field.name}),`;
  }
  return `            ${field.name}: wire.${field.name},`;
}

function snakeToLowerCamel(name) {
  return name.replace(/_([a-z])/gu, (_match, char) => char.toUpperCase());
}

function redactViewRustDebug() {
  let source = readFileSync(rustGeneratedViewPath, "utf8");
  for (const [messageName, fieldEntries] of sensitiveFieldNames.entries()) {
    const fields = fieldEntries.map((field) => field.name);
    const viewName = `${messageName}View`;
    source = replaceAllRequired(
      source,
      `#[derive(Clone, Debug, Default)]\npub struct ${viewName}<'a> {`,
      `#[derive(Clone, Default)]\npub struct ${viewName}<'a> {`,
      rustGeneratedViewPath,
    );

    if (source.includes(`impl<'a> ::core::fmt::Debug for ${viewName}<'a>`)) {
      continue;
    }

    const structStart = source.indexOf(`pub struct ${viewName}<'a> {`);
    if (structStart < 0) {
      throw new Error(`unable to locate generated Rust view ${viewName}`);
    }
    const structEnd = source.indexOf("\n}", structStart);
    if (structEnd < 0) {
      throw new Error(`unable to locate end of generated Rust view ${viewName}`);
    }
    const body = source.slice(structStart, structEnd);
    const fieldNames = [...body.matchAll(/^\s+pub\s+(\w+):/gmu)].map((field) => field[1]);
    const debugFields = fieldNames
      .filter((fieldName) => fieldName !== "__buffa_unknown_fields")
      .map((fieldName) => {
        const fieldValue = fields.includes(fieldName) ? '"<redacted>"' : `self.${fieldName}`;
        return `            .field("${fieldName}", &${fieldValue})`;
      })
      .join("\n");
    const debugImpl = `
impl<'a> ::core::fmt::Debug for ${viewName}<'a> {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        f.debug_struct("${viewName}")
${debugFields}
            .finish()
    }
}
`;
    source = `${source.slice(0, structEnd + 2)}${debugImpl}${source.slice(structEnd + 2)}`;
  }
  writeFileSync(rustGeneratedViewPath, source);
}

function removeSensitiveRustOwnedViews() {
  let viewSource = readFileSync(rustGeneratedViewPath, "utf8");
  let moduleSource = readFileSync(rustGeneratedModulePath, "utf8");

  for (const messageName of sensitiveOwnedViewMessageNames) {
    const ownedViewName = `${messageName}OwnedView`;
    const startMarker = `/** Self-contained, \`'static\` owned view of a \`${messageName}\` message.`;
    const start = viewSource.indexOf(startMarker);
    if (start < 0) {
      if (!viewSource.includes(`pub struct ${ownedViewName}(`)) {
        continue;
      }
      fail(`unable to locate generated owned-view block for ${messageName}`);
    }

    const serializeMarker = `impl ::serde::Serialize for ${ownedViewName} {`;
    const serializeStart = viewSource.indexOf(serializeMarker, start);
    if (serializeStart < 0) {
      fail(`unable to locate generated owned-view Serialize impl for ${messageName}`);
    }
    const serializeOpen = viewSource.indexOf("{", serializeStart);
    const serializeEnd = findMatchingBrace(viewSource, serializeOpen);
    let removeEnd = serializeEnd + 1;
    if (viewSource[removeEnd] === "\n") {
      removeEnd += 1;
    }
    viewSource = `${viewSource.slice(0, start)}${viewSource.slice(removeEnd)}`;

    const reexport = `#[doc(inline)]\npub use self::__buffa::view::${ownedViewName};\n`;
    if (!moduleSource.includes(reexport)) {
      fail(`missing generated module re-export for ${ownedViewName}`);
    }
    moduleSource = moduleSource.replace(reexport, "");
  }

  writeFileSync(rustGeneratedViewPath, viewSource);
  writeFileSync(rustGeneratedModulePath, moduleSource);
}

function redactSwiftDebug() {
  const filePath = join(root.pathname, "gen/swift/reallyme/codec/v1/codec.pb.swift");
  let source = readFileSync(filePath, "utf8");
  for (const messageName of sensitiveMessageNames) {
    const swiftName = `ReallyMeProto${messageName}`;
    const declaration = `public nonisolated struct ${swiftName}: Sendable {`;
    if (countOccurrences(source, declaration) !== 1) {
      fail(`generated Swift message ${swiftName} must have exactly one declaration`);
    }
    const additions = [];
    if (!source.includes(`public var debugDescription: String { "${swiftName}(<redacted>)" }`)) {
      additions.push(`  // Security post-processing: protobuf fields can contain secrets or PII.
  public var debugDescription: String { "${swiftName}(<redacted>)" }

  public func hash(into hasher: inout Hasher) {
    hasher.combine("${swiftName}(<redacted>)")
  }`);
    }
    if (!source.includes(`public func textFormatString() -> String { "${swiftName}(<redacted>)" }`)) {
      additions.push(`  // SwiftProtobuf's protocol-extension implementation traverses every
  // field. Concrete sensitive messages shadow both public overloads so an
  // explicit text-format call cannot bypass debug redaction.
  public func textFormatString() -> String { "${swiftName}(<redacted>)" }

  public func textFormatString(
    options _: SwiftProtobuf.TextFormatEncodingOptions
  ) -> String { "${swiftName}(<redacted>)" }`);
    }
    if (additions.length !== 0) {
      source = source.replace(declaration, `${declaration}\n${additions.join("\n\n")}`);
    }
    for (const required of [
      `public var debugDescription: String { "${swiftName}(<redacted>)" }`,
      `public func textFormatString() -> String { "${swiftName}(<redacted>)" }`,
      `) -> String { "${swiftName}(<redacted>)" }`,
    ]) {
      if (!source.includes(required)) {
        fail(`generated Swift message ${swiftName} is missing ${required}`);
      }
    }
  }
  writeFileSync(filePath, source);
}

function redactJavaDebug() {
  for (const messageName of sensitiveMessageNames) {
    const filePath = join(root.pathname, `gen/java/me/really/codec/v1/${messageName}.java`);
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
    if (
      countOccurrences(source, `return "${messageName}{<redacted>}";`) !== 1 ||
      countOccurrences(source, "return 0x524d;") < 1
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

function generatedOutputPaths() {
  const javaDirectory = join(root.pathname, "gen/java/me/really/codec/v1");
  const kotlinDirectory = join(root.pathname, "gen/kotlin/me/really/codec/v1");
  return [
    rustGeneratedPath,
    rustGeneratedViewPath,
    rustGeneratedModulePath,
    join(root.pathname, "gen/swift/reallyme/codec/v1/codec.pb.swift"),
    ...readdirSync(javaDirectory)
      .filter((fileName) => fileName.endsWith(".java"))
      .map((fileName) => join(javaDirectory, fileName)),
    ...readdirSync(kotlinDirectory)
      .filter((fileName) => fileName.endsWith(".kt"))
      .map((fileName) => join(kotlinDirectory, fileName)),
  ];
}

const idempotencyBefore = checkIdempotent
  ? new Map(generatedOutputPaths().map((path) => [path, readFileSync(path)]))
  : null;

validateScalarFieldClassifications();
redactOwnedRustDebugAndMemory();
redactViewRustDebug();
removeSensitiveRustOwnedViews();
redactSwiftDebug();
redactJavaDebug();
normalizeGeneratedWhitespace(join(root.pathname, "gen/java/me/really/codec/v1"), ".java");
normalizeGeneratedWhitespace(join(root.pathname, "gen/kotlin/me/really/codec/v1"), ".kt");

if (idempotencyBefore !== null) {
  for (const [path, before] of idempotencyBefore) {
    if (!before.equals(readFileSync(path))) {
      fail("generated protobuf hardening is not idempotent");
    }
  }
}
