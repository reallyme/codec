// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { codecProtoScalarFieldClassifications, codecProtoSensitiveNonTextFieldClassifications, codecProtoSensitiveOwnerMessages } from "./codec_proto_sensitivity.mjs";
const root = fileURLToPath(new URL("..", import.meta.url));
const protoSource = readFileSync(join(root, "crates/proto/proto/reallyme/codec/v1/codec.proto"), "utf8");
const sensitiveFieldNames = new Map();
for (const field of [...codecProtoScalarFieldClassifications, ...codecProtoSensitiveNonTextFieldClassifications]) {
  if (field.sensitivity !== "sensitive") continue;
  const fields = sensitiveFieldNames.get(field.message) ?? [];
  fields.push({ name: field.field, kind: field.kind });
  sensitiveFieldNames.set(field.message, fields);
}
for (const name of codecProtoSensitiveOwnerMessages) if (!sensitiveFieldNames.has(name)) sensitiveFieldNames.set(name, []);
const sensitiveMessageNames = [...sensitiveFieldNames.keys()];
const sensitiveOwnerMessageNames = new Set(codecProtoSensitiveOwnerMessages);
const codecEnumTypeNames = new Set([...scrubProtoCommentsAndStrings(protoSource).matchAll(/^enum\s+(Codec\w+)\s*\{/gmu)].map((match) => match[1]));
function snakeToLowerCamel(name) { return name.replace(/_([a-z])/gu, (_match, letter) => letter.toUpperCase()); }
function fail(message) { console.error(`codec protocol generation failed: ${message}`); process.exit(1); }
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
    /(?:^|[;{}])\s*(?:(optional|required|repeated)\s+)?(\w+)\s+(\w+)\s*=\s*(\d+)(?:\s*\[[^\]]*\])?\s*(?=;)/gmu;
  return [...body.matchAll(fieldPattern)].map(
    ([, label, typeName, name, number]) => ({
      repeated: label === "repeated",
      optional: label === "optional",
      typeName,
      name,
      number: Number(number),
      jsonName: snakeToLowerCamel(name),
    }),
  );
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
      !["bool", "uint32", "uint64", "sint64", "message-field", "repeated-message"].includes(
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


export { root, protoSource, sensitiveFieldNames, sensitiveMessageNames, sensitiveOwnerMessageNames, codecEnumTypeNames, snakeToLowerCamel, fail, scrubProtoCommentsAndStrings, findMatchingBrace, messageFields, validateScalarFieldClassifications };
