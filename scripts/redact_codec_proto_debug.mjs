#!/usr/bin/env node
// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

import {
  codecProtoProviderOutputMessages,
  codecProtoScalarFieldClassifications,
  codecProtoSensitiveNonTextFieldClassifications,
  codecProtoSensitiveOwnerMessages,
} from "./codec_proto_sensitivity.mjs";

const root = new URL("..", import.meta.url);

const protoPath = join(
  root.pathname,
  "crates/proto/proto/reallyme/codec/v1/codec.proto",
);
const rustGeneratedPath = join(
  root.pathname,
  "crates/proto/src/generated/buffa/reallyme.codec.v1.codec.rs",
);
const rustGeneratedViewPath = join(
  root.pathname,
  "crates/proto/src/generated/buffa/reallyme.codec.v1.codec.__view.rs",
);
const rustGeneratedModulePath = join(
  root.pathname,
  "crates/proto/src/generated/buffa/reallyme.codec.v1.mod.rs",
);
const esGeneratedPath = join(
  root.pathname,
  "gen/es/reallyme/codec/v1/codec_pb.ts",
);
const tsPackageGeneratedPath = join(
  root.pathname,
  "packages/ts/src/proto/generated/reallyme/codec/v1/codec_pb.ts",
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
for (const classification of [
  ...codecProtoScalarFieldClassifications,
  ...codecProtoSensitiveNonTextFieldClassifications,
]) {
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
const sensitiveOwnerMessageNames = new Set(codecProtoSensitiveOwnerMessages);
const codecEnumTypeNames = new Set([
  "CodecErrorReason",
  "CodecKeyMaterialKind",
  "CodecPemLabel",
  "CodecErrorOrigin",
  "CodecTag",
]);
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

function rustOwnedMessageFieldNames(source, messageName) {
  const region = generatedRustMessageRegion(source, messageName);
  return [...region.matchAll(/^\s+pub\s+([a-z][a-z0-9_]*):/gmu)]
    .map((field) => field[1])
    .filter((fieldName) => fieldName !== "__buffa_unknown_fields");
}

function hasDropImpl(region, messageName) {
  return region.includes(`impl ::core::ops::Drop for ${messageName}`);
}

function validateSensitiveRustHardening(source) {
  for (const [messageName, fields] of sensitiveFieldNames) {
    const region = generatedRustMessageRegion(source, messageName);
    for (const field of fields) {
      const zeroizeNeedle = sensitiveFieldWipe(field, "self");
      // One wipe belongs to the generated clear/merge path and one to Drop.
      // Requiring both prevents the Drop implementation from masking a failed
      // rewrite when generator formatting changes.
      if (countOccurrences(region, zeroizeNeedle) < 2) {
        fail(`${messageName}.${field.name} is missing a generated-path or Drop wipe`);
      }
      if (region.includes(generatedSensitiveClear(field))) {
        fail(`${messageName}.${field.name} still uses a non-zeroizing clear path`);
      }
    }
    const unknownWipe =
      "__reallyme_zeroize_unknown_fields(&mut self.__buffa_unknown_fields);";
    const requiredUnknownWipes = fields.length > 0 || hasDropImpl(region, messageName) ? 2 : 1;
    if (countOccurrences(region, unknownWipe) < requiredUnknownWipes) {
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

function sensitiveFieldWipe(field, owner) {
  if (field.kind === "repeated-message") {
    return `__reallyme_zeroize_message_vec(&mut ${owner}.${field.name});`;
  }
  if (field.kind === "message-field") {
    return `__reallyme_zeroize_message_field(&mut ${owner}.${field.name});`;
  }
  return `::zeroize::Zeroize::zeroize(&mut ${owner}.${field.name});`;
}

function hardenSensitiveSerialize(source, messageName, fields) {
  const structMarker = `pub struct ${messageName} {`;
  const structStart = source.indexOf(structMarker);
  if (structStart < 0) {
    fail(`missing generated Rust message ${messageName}`);
  }
  const structEnd = findMatchingBrace(source, source.indexOf("{", structStart));
  let region = source.slice(structStart, structEnd + 1);

  for (const field of fields.filter((entry) => entry.kind === "bytes")) {
    const declaration = `pub ${field.name}:`;
    const declarationIndex = region.indexOf(declaration);
    if (declarationIndex < 0) {
      fail(`missing generated Rust field ${messageName}.${field.name}`);
    }
    const attributeStart = region.lastIndexOf("#[serde(", declarationIndex);
    const attributeEnd = region.indexOf(")]", attributeStart);
    if (attributeStart < 0 || attributeEnd < 0 || attributeEnd > declarationIndex) {
      fail(`missing generated serde attribute for ${messageName}.${field.name}`);
    }
    const attribute = region.slice(attributeStart, attributeEnd + 2);
    const generated = 'with = "::buffa::json_helpers::bytes"';
    const hardened = 'serialize_with = "__reallyme_serialize_sensitive_bytes"';
    const serializerAndDeserializer = `serialize_with = "__reallyme_serialize_sensitive_bytes",
        deserialize_with = "::buffa::json_helpers::bytes::deserialize"`;
    if (attribute.includes(serializerAndDeserializer)) {
      region = `${region.slice(0, attributeStart)}${attribute.replace(serializerAndDeserializer, hardened)}${region.slice(attributeEnd + 2)}`;
      continue;
    }
    if (attribute.includes(hardened)) {
      continue;
    }
    if (!attribute.includes(generated)) {
      fail(`missing generated bytes serializer for ${messageName}.${field.name}`);
    }
    region = `${region.slice(0, attributeStart)}${attribute.replace(generated, hardened)}${region.slice(attributeEnd + 2)}`;
  }

  return `${source.slice(0, structStart)}${region}${source.slice(structEnd + 1)}`;
}

function generatedSensitiveClear(field) {
  switch (field.kind) {
    case "bool":
      return `self.${field.name} = false;`;
    case "uint64":
      return `self.${field.name} = 0u64;`;
    case "sint64":
      return `self.${field.name} = 0i64;`;
    case "message-field":
      return `self.${field.name} = ::buffa::MessageField::none();`;
    case "bytes":
    case "string":
    case "repeated-message":
      return `self.${field.name}.clear();`;
    default:
      fail(`unsupported sensitive field kind ${field.kind}`);
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
  const nonTextClassificationKeys = new Set();
  for (const classification of codecProtoSensitiveNonTextFieldClassifications) {
    if (
      classification === null ||
      typeof classification !== "object" ||
      !/^[A-Z][A-Za-z0-9]*$/u.test(classification.message) ||
      !/^[a-z][a-z0-9_]*$/u.test(classification.field) ||
      !["bool", "uint64", "sint64", "message-field", "repeated-message"].includes(
        classification.kind,
      ) ||
      classification.sensitivity !== "sensitive"
    ) {
      fail("invalid protobuf non-text sensitivity classification");
    }
    const key = `${classification.message}.${classification.field}:${classification.kind}`;
    if (nonTextClassificationKeys.has(key)) {
      fail(`duplicate protobuf non-text sensitivity classification ${key}`);
    }
    nonTextClassificationKeys.add(key);

    const schemaField = messageFields(classification.message)
      .find((field) => field.name === classification.field);
    if (schemaField === undefined) {
      fail(`stale protobuf non-text sensitivity classification ${key}`);
    }
    let schemaKind = schemaField.typeName;
    if (schemaField.repeated && schemaField.typeName.startsWith("Codec")) {
      schemaKind = "repeated-message";
    } else if (
      !schemaField.repeated &&
      schemaField.typeName.startsWith("Codec") &&
      !codecEnumTypeNames.has(schemaField.typeName)
    ) {
      schemaKind = "message-field";
    }
    if (schemaKind !== classification.kind) {
      fail(`protobuf non-text sensitivity classification type mismatch ${key}`);
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
    const scalarFieldNames = new Set(fields.map((field) => field.name));
    const fieldNamesToRedact = new Set(scalarFieldNames);
    if (sensitiveOwnerMessageNames.has(messageName)) {
      for (const fieldName of rustOwnedMessageFieldNames(source, messageName)) {
        fieldNamesToRedact.add(fieldName);
      }
    }
    for (const name of fieldNamesToRedact) {
      source = replaceAllRequired(
        source,
        `.field("${name}", &self.${name})`,
        `.field("${name}", &"<redacted>")`,
        rustGeneratedPath,
      );
      if (!scalarFieldNames.has(name)) {
        continue;
      }
      const field = fields.find((entry) => entry.name === name);
      if (field === undefined) {
        fail(`missing sensitive field metadata for ${messageName}.${name}`);
      }
      source = replaceAllRequired(
        source,
        `        ${generatedSensitiveClear(field)}`,
        `        ${sensitiveFieldWipe(field, "self")}`,
        rustGeneratedPath,
      );
    }

    const structMarker = `pub struct ${messageName} {`;
    const structStart = source.indexOf(structMarker);
    if (structStart < 0) {
      fail(`missing generated Rust message ${messageName}`);
    }

    const generatedSerdeDeserialize = messageHasSerdeDeserializeDerive(source, messageName);
    const needsSensitiveDeserialize =
      fields.length > 0 ||
      (sensitiveOwnerMessageNames.has(messageName) && generatedSerdeDeserialize);
    if (needsSensitiveDeserialize) {
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

    source = hardenSensitiveSerialize(source, messageName, fields);

    if (fields.length > 0 || sensitiveOwnerMessageNames.has(messageName)) {
      source = hardenSensitiveDrop(source, messageName, fields);
    } else {
      source = removeSensitiveDrop(source, messageName);
    }
    if (needsSensitiveDeserialize) {
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
  const generatedHeader = `// @generated by buffa-codegen. DO NOT EDIT.
// source: reallyme/codec/v1/codec.proto
`;
  const helpers = `
fn __reallyme_zeroize_unknown_fields(fields: &mut ::buffa::UnknownFields) {
    // Buffa intentionally keeps UnknownFields storage opaque. Taking the
    // collection and wiping the owned iterator's backing slice is the only
    // safe public route that clears both every value and the allocation slots
    // from which IntoIter will later drop them. No private-layout or unsafe
    // assumption is made.
    let mut owned_fields = ::core::iter::IntoIterator::into_iter(::core::mem::take(fields));
    for field in owned_fields.as_mut_slice() {
        ::zeroize::Zeroize::zeroize(&mut field.number);
        __reallyme_zeroize_unknown_field_data(&mut field.data);
    }
}

fn __reallyme_zeroize_unknown_field_data(data: &mut ::buffa::UnknownFieldData) {
    match data {
        ::buffa::UnknownFieldData::LengthDelimited(bytes) => {
            ::zeroize::Zeroize::zeroize(bytes);
            ::zeroize::Zeroize::zeroize(bytes.spare_capacity_mut());
        }
        ::buffa::UnknownFieldData::Group(fields) => {
            __reallyme_zeroize_unknown_fields(fields);
        }
        ::buffa::UnknownFieldData::Varint(value) => {
            ::zeroize::Zeroize::zeroize(value);
        }
        ::buffa::UnknownFieldData::Fixed64(value) => {
            ::zeroize::Zeroize::zeroize(value);
        }
        ::buffa::UnknownFieldData::Fixed32(value) => {
            ::zeroize::Zeroize::zeroize(value);
        }
    }
}

fn __reallyme_zeroize_message_vec<M: ::buffa::Message>(values: &mut ::buffa::alloc::vec::Vec<M>) {
    for value in values.iter_mut() {
        ::buffa::Message::clear(value);
    }
    values.clear();
    ::zeroize::Zeroize::zeroize(values.spare_capacity_mut());
}

fn __reallyme_zeroize_message_field<M: ::buffa::Message, P: ::buffa::ProtoBox<M>>(
    value: &mut ::buffa::MessageField<M, P>,
) {
    if let ::core::option::Option::Some(message) = value.as_option_mut() {
        ::buffa::Message::clear(message);
    }
    *value = ::buffa::MessageField::none();
}

// Buffa's default ProtoJSON bytes helper materializes a base64 String. For
// sensitive fields that hidden allocation would be freed without wiping. This
// wrapper streams standard padded base64 through Serializer::collect_str using
// one zeroizing stack buffer, so serde_json writes directly into the caller's
// bounded writer without creating an immutable secret-bearing String.
struct __ReallyMeSensitiveBytes<'a>(&'a [u8]);

impl ::serde::Serialize for __ReallyMeSensitiveBytes<'_> {
    fn serialize<S: ::serde::Serializer>(
        &self,
        serializer: S,
    ) -> ::core::result::Result<S::Ok, S::Error> {
        serializer.collect_str(&__ReallyMeSensitiveBase64(self.0))
    }
}

struct __ReallyMeSensitiveBase64<'a>(&'a [u8]);

impl ::core::fmt::Display for __ReallyMeSensitiveBase64<'_> {
    fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        let mut encoded = ::zeroize::Zeroizing::new([0_u8; 4]);
        for chunk in self.0.chunks(3) {
            let written = ::base64::Engine::encode_slice(
                &::base64::engine::general_purpose::STANDARD,
                chunk,
                encoded.as_mut_slice(),
            )
            .map_err(|_| ::core::fmt::Error)?;
            let text = ::core::str::from_utf8(&encoded[..written])
                .map_err(|_| ::core::fmt::Error)?;
            formatter.write_str(text)?;
            ::zeroize::Zeroize::zeroize(encoded.as_mut_slice());
        }
        Ok(())
    }
}

fn __reallyme_serialize_sensitive_bytes<S: ::serde::Serializer>(
    value: &[u8],
    serializer: S,
) -> ::core::result::Result<S::Ok, S::Error> {
    ::serde::Serialize::serialize(&__ReallyMeSensitiveBytes(value), serializer)
}

enum __ReallyMeSensitiveBase64DecodeError {
    LengthOverflow,
    AllocationFailure,
    InvalidEncoding,
}

impl ::core::fmt::Display for __ReallyMeSensitiveBase64DecodeError {
    fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        formatter.write_str(match self {
            Self::LengthOverflow => "sensitive base64 length overflow",
            Self::AllocationFailure => "sensitive base64 allocation failure",
            Self::InvalidEncoding => "invalid sensitive base64 encoding",
        })
    }
}

const __REALLYME_SENSITIVE_BASE64_CONFIG: ::base64::engine::general_purpose::GeneralPurposeConfig =
    ::base64::engine::general_purpose::GeneralPurposeConfig::new()
        .with_decode_allow_trailing_bits(true)
        .with_decode_padding_mode(::base64::engine::DecodePaddingMode::Indifferent);
const __REALLYME_SENSITIVE_BASE64_STANDARD: ::base64::engine::general_purpose::GeneralPurpose =
    ::base64::engine::general_purpose::GeneralPurpose::new(
        &::base64::alphabet::STANDARD,
        __REALLYME_SENSITIVE_BASE64_CONFIG,
    );
const __REALLYME_SENSITIVE_BASE64_URL_SAFE: ::base64::engine::general_purpose::GeneralPurpose =
    ::base64::engine::general_purpose::GeneralPurpose::new(
        &::base64::alphabet::URL_SAFE,
        __REALLYME_SENSITIVE_BASE64_CONFIG,
    );

fn __reallyme_decode_sensitive_base64(
    value: &str,
) -> ::core::result::Result<
    ::zeroize::Zeroizing<::buffa::alloc::vec::Vec<u8>>,
    __ReallyMeSensitiveBase64DecodeError,
> {
    let groups = value
        .len()
        .checked_add(3)
        .ok_or(__ReallyMeSensitiveBase64DecodeError::LengthOverflow)?
        / 4;
    let capacity = groups
        .checked_mul(3)
        .ok_or(__ReallyMeSensitiveBase64DecodeError::LengthOverflow)?;
    let mut decoded = ::zeroize::Zeroizing::new(::buffa::alloc::vec::Vec::new());
    decoded
        .try_reserve_exact(capacity)
        .map_err(|_| __ReallyMeSensitiveBase64DecodeError::AllocationFailure)?;
    decoded.resize(capacity, 0);

    let written = match ::base64::Engine::decode_slice(
        &__REALLYME_SENSITIVE_BASE64_STANDARD,
        value.as_bytes(),
        decoded.as_mut_slice(),
    ) {
        Ok(written) => written,
        Err(_) => {
            ::zeroize::Zeroize::zeroize(decoded.as_mut_slice());
            ::base64::Engine::decode_slice(
                &__REALLYME_SENSITIVE_BASE64_URL_SAFE,
                value.as_bytes(),
                decoded.as_mut_slice(),
            )
            .map_err(|_| __ReallyMeSensitiveBase64DecodeError::InvalidEncoding)?
        }
    };
    decoded.truncate(written);
    Ok(decoded)
}

fn __reallyme_deserialize_sensitive_bytes_zeroizing<'de, D>(
    deserializer: D,
) -> ::core::result::Result<
    ::zeroize::Zeroizing<::buffa::alloc::vec::Vec<u8>>,
    D::Error,
>
where
    D: ::serde::Deserializer<'de>,
{
    struct Visitor;

    impl<'de> ::serde::de::Visitor<'de> for Visitor {
        type Value = ::zeroize::Zeroizing<::buffa::alloc::vec::Vec<u8>>;

        fn expecting(
            &self,
            formatter: &mut ::core::fmt::Formatter<'_>,
        ) -> ::core::fmt::Result {
            formatter.write_str("a base64-encoded string, or null")
        }

        fn visit_unit<E>(self) -> ::core::result::Result<Self::Value, E> {
            Ok(::zeroize::Zeroizing::new(::buffa::alloc::vec::Vec::new()))
        }

        fn visit_str<E: ::serde::de::Error>(
            self,
            value: &str,
        ) -> ::core::result::Result<Self::Value, E> {
            __reallyme_decode_sensitive_base64(value).map_err(E::custom)
        }
    }

    deserializer.deserialize_any(Visitor)
}
`;
  const helperStart = source.indexOf("fn __reallyme_zeroize_unknown_fields(");
  if (helperStart >= 0) {
    const firstDocumentation = source.indexOf("///", helperStart);
    if (firstDocumentation < 0) {
      fail(`${rustGeneratedPath} is missing its first generated declaration`);
    }
    return `${source.slice(0, helperStart)}${helpers.trimStart()}\n${source.slice(firstDocumentation)}`;
  }
  return replaceOnce(source, generatedHeader, `${generatedHeader}${helpers}`, rustGeneratedPath);
}

function hardenSensitiveDrop(source, messageName, fields) {
  const body = fields
    .map((field) => `        ${sensitiveFieldWipe(field, "self")}`)
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
  const existingDropPattern = sensitiveDropPattern(messageName, securityComment);
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

function removeSensitiveDrop(source, messageName) {
  const securityComment = `// SECURITY: Buffa's generated message contract requires Clone. Every clone
// remains an owning value, and this Drop implementation wipes its sensitive
// fields and unknown-field storage when that owner leaves scope.
`;
  return source.replace(sensitiveDropPattern(messageName, securityComment), "");
}

function sensitiveDropPattern(messageName, securityComment) {
  return new RegExp(
    `(?:${escapeRegExp(securityComment)})*impl ::core::ops::Drop for ${messageName} \\{\\n    fn drop\\(&mut self\\) \\{\\n[\\s\\S]*?    \\}\\n\\}\\n`,
    "u",
  );
}

function messageHasSerdeDeserializeDerive(source, messageName) {
  const structMarker = `pub struct ${messageName} {`;
  const structStart = source.indexOf(structMarker);
  if (structStart < 0) {
    fail(`missing generated Rust message ${messageName}`);
  }
  const structHeaderStart = Math.max(0, structStart - 512);
  const structHeader = source.slice(structHeaderStart, structStart);
  return structHeader.includes("#[derive(::serde::Serialize, ::serde::Deserialize)]");
}

function hardenSensitiveDeserialize(source, messageName, fields) {
  const qualifiedMarker = `impl<'de> ::serde::Deserialize<'de> for ${messageName}`;
  const unqualifiedMarker = `impl<'de> serde::Deserialize<'de> for ${messageName}`;
  const qualifiedExactPattern = new RegExp(
    `${escapeRegExp(qualifiedMarker)}\\s*\\{`,
    "u",
  );
  const unqualifiedExactPattern = new RegExp(
    `${escapeRegExp(unqualifiedMarker)}\\s*\\{`,
    "u",
  );
  if (!qualifiedExactPattern.test(source) && !unqualifiedExactPattern.test(source)) {
    const implMarker = `impl ${messageName} {`;
    const implIndex = source.indexOf(implMarker);
    if (implIndex < 0) {
      fail(`missing generated inherent impl for ${messageName}`);
    }
    return `${source.slice(0, implIndex)}${sensitiveDeserializeImpl(source, messageName, fields)}${source.slice(implIndex)}`;
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
    `${sensitiveDeserializeImpl(source, messageName, fields)}impl ${messageName} {`,
  );
}

function sensitiveDeserializeImpl(source, messageName, sensitiveFields) {
  const fields = messageFields(messageName);
  const sensitiveNames = new Map(sensitiveFields.map((field) => [field.name, field.kind]));
  const wireFields = fields
    .map((field) => wireFieldDeclaration(source, messageName, field, sensitiveNames.get(field.name)))
    .join("\n");
  const outputFields = fields
    .map((field) => outputFieldInitializer(field, sensitiveNames.get(field.name)))
    .join("\n");
  const wireBinding = fields.length === 0
    ? "        let _wire = Wire::deserialize(deserializer)?;"
    : "        let mut wire = Wire::deserialize(deserializer)?;";

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
            __reallyme_deserialize_sensitive_bytes_zeroizing(deserializer)
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

${wireBinding}
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

function wireFieldDeclaration(source, messageName, field, sensitiveKind) {
  const serdeParts = [`rename = "${field.jsonName}"`];
  if (field.jsonName !== field.name) {
    serdeParts.push(`alias = "${field.name}"`);
  }
  const rustType = rustFieldType(source, messageName, field, sensitiveKind);
  if (sensitiveKind === "bytes") {
    serdeParts.push("deserialize_with = \"deserialize_zeroizing_bytes\"");
  } else if (sensitiveKind === "string") {
    serdeParts.push("deserialize_with = \"deserialize_zeroizing_string\"");
  } else if (sensitiveKind === "uint64") {
    serdeParts.push("with = \"::buffa::json_helpers::uint64\"");
  } else if (sensitiveKind === "sint64") {
    serdeParts.push("with = \"::buffa::json_helpers::int64\"");
  } else if (codecEnumTypeNames.has(field.typeName)) {
    serdeParts.push("with = \"::buffa::json_helpers::proto_enum\"");
  }
  return `            #[serde(${serdeParts.join(", ")})]
            ${field.name}: ${rustType},`;
}

function rustFieldType(source, messageName, field, sensitiveKind) {
  if (sensitiveKind === "bytes") {
    return "::zeroize::Zeroizing<::buffa::alloc::vec::Vec<u8>>";
  }
  if (sensitiveKind === "string") {
    return "::zeroize::Zeroizing<::buffa::alloc::string::String>";
  }
  if (sensitiveKind === "message-field") {
    return generatedRustFieldType(source, messageName, field.name);
  }
  if (field.repeated && field.typeName.startsWith("Codec")) {
    return `::buffa::alloc::vec::Vec<${field.typeName}>`;
  }
  if (codecEnumTypeNames.has(field.typeName)) {
    return `::buffa::EnumValue<${field.typeName}>`;
  }
  if (field.typeName.startsWith("Codec") && !codecEnumTypeNames.has(field.typeName)) {
    return generatedRustFieldType(source, messageName, field.name);
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
  if (field.typeName === "uint64") {
    return "u64";
  }
  if (field.typeName === "sint64") {
    return "i64";
  }
  if (field.typeName === "bool") {
    return "bool";
  }
  fail(`unsupported generated field type ${field.typeName} on ${field.name}`);
}

function generatedRustFieldType(source, messageName, fieldName) {
  const region = generatedRustMessageRegion(source, messageName);
  const fieldMarker = `pub ${fieldName}:`;
  const fieldStart = region.indexOf(fieldMarker);
  if (fieldStart < 0) {
    fail(`missing generated Rust field ${messageName}.${fieldName}`);
  }
  const typeStart = fieldStart + fieldMarker.length;
  let angleDepth = 0;
  let parenDepth = 0;
  let bracketDepth = 0;
  for (let index = typeStart; index < region.length; index += 1) {
    const char = region[index];
    if (char === "<") {
      angleDepth += 1;
    } else if (char === ">") {
      angleDepth -= 1;
    } else if (char === "(") {
      parenDepth += 1;
    } else if (char === ")") {
      parenDepth -= 1;
    } else if (char === "[") {
      bracketDepth += 1;
    } else if (char === "]") {
      bracketDepth -= 1;
    } else if (
      char === "," &&
      angleDepth === 0 &&
      parenDepth === 0 &&
      bracketDepth === 0
    ) {
      return region.slice(typeStart, index).trim();
    }
  }
  fail(`unable to parse generated Rust type for ${messageName}.${fieldName}`);
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
  return `            ${field.name}: ::core::mem::take(&mut wire.${field.name}),`;
}

function snakeToLowerCamel(name) {
  return name.replace(/_([a-z])/gu, (_match, char) => char.toUpperCase());
}

function redactViewRustDebug() {
  let source = readFileSync(rustGeneratedViewPath, "utf8");
  for (const [messageName, fieldEntries] of sensitiveFieldNames.entries()) {
    const fields = new Set(fieldEntries.map((field) => field.name));
    if (sensitiveOwnerMessageNames.has(messageName)) {
      for (const field of messageFields(messageName)) {
        fields.add(field.name);
      }
    }
    const viewName = `${messageName}View`;
    source = replaceAllRequired(
      source,
      `#[derive(Clone, Debug, Default)]\npub struct ${viewName}<'a> {`,
      `#[derive(Clone, Default)]\npub struct ${viewName}<'a> {`,
      rustGeneratedViewPath,
    );

    if (source.includes(`impl<'a> ::core::fmt::Debug for ${viewName}<'a>`)) {
      source = hardenSensitiveViewSerialize(source, viewName, fieldEntries);
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
    const redactedFields = sensitiveOwnerMessageNames.has(messageName)
      ? new Set(fieldNames)
      : fields;
    const debugFields = fieldNames
      .filter((fieldName) => fieldName !== "__buffa_unknown_fields")
      .map((fieldName) => {
        const fieldValue = redactedFields.has(fieldName) ? '"<redacted>"' : `self.${fieldName}`;
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
    source = hardenSensitiveViewSerialize(source, viewName, fieldEntries);
  }
  writeFileSync(rustGeneratedViewPath, source);
}

function hardenSensitiveViewSerialize(source, viewName, fields) {
  const structMarker = `pub struct ${viewName}<'a> {`;
  const structStart = source.indexOf(structMarker);
  if (structStart < 0) {
    fail(`missing generated Rust view ${viewName}`);
  }
  const remainder = source.slice(structStart + structMarker.length);
  const nextView = /^pub struct [A-Z][A-Za-z0-9]*View<'a> \{/gmu.exec(remainder);
  const regionEnd = nextView === null
    ? source.length
    : structStart + structMarker.length + nextView.index;
  let region = source.slice(structStart, regionEnd);

  for (const field of fields.filter((entry) => entry.kind === "bytes")) {
    const generated = `::buffa::json_helpers::BytesJson(self.${field.name})`;
    const hardened = `super::super::__ReallyMeSensitiveBytes(self.${field.name})`;
    if (region.includes(hardened)) {
      continue;
    }
    if (!region.includes(generated)) {
      fail(`missing generated view bytes serializer for ${viewName}.${field.name}`);
    }
    region = region.replace(generated, hardened);
  }

  return `${source.slice(0, structStart)}${region}${source.slice(regionEnd)}`;
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
      root.pathname,
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

function generatedOutputPaths() {
  const javaDirectory = join(root.pathname, "gen/java/me/really/codec/v1");
  const kotlinDirectory = join(root.pathname, "gen/kotlin/me/really/codec/v1");
  return [
    rustGeneratedPath,
    rustGeneratedViewPath,
    rustGeneratedModulePath,
    esGeneratedPath,
    tsPackageGeneratedPath,
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
normalizeGeneratedWhitespace(join(root.pathname, "gen/es/reallyme/codec/v1"), ".ts");
normalizeGeneratedWhitespace(
  join(root.pathname, "packages/ts/src/proto/generated/reallyme/codec/v1"),
  ".ts",
);

if (idempotencyBefore !== null) {
  for (const [path, before] of idempotencyBefore) {
    if (!before.equals(readFileSync(path))) {
      fail("generated protobuf hardening is not idempotent");
    }
  }
}
