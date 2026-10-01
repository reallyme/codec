#!/usr/bin/env node
// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const limits = JSON.parse(readFileSync(resolve(root, "scripts/codec_limits.json"), "utf8"));
const write = process.argv.length === 3 && process.argv[2] === "--write";
if (!write && !(process.argv.length === 3 && process.argv[2] === "--check")) {
  throw new Error("use --check or --write");
}

const decimal = (value) => {
  if (!Number.isSafeInteger(value) || value <= 0) {
    throw new Error("codec limits must be positive safe integers");
  }
  return value.toString().replace(/\B(?=(\d{3})+(?!\d))/g, "_");
};

const edits = new Map();
const add = (path, declaration, value, terminator = ";") => {
  const entries = edits.get(path) ?? [];
  entries.push({ declaration, value: decimal(value), terminator });
  edits.set(path, entries);
};

const deterministic = limits.deterministicCbor;
const dag = limits.dagCbor;
const protobuf = limits.protobuf;
const payloadBytes = deterministic.aggregateTextBytes + deterministic.aggregateByteStringBytes;
const jsonTextBytes = deterministic.aggregateTextBytes * protobuf.jsonTextEscapeExpansion;
const jsonByteStringBytes = Math.floor((deterministic.aggregateByteStringBytes + 2) / 3) * 4;
const structuralBytes = deterministic.nodes * protobuf.structuralBytesPerNode;
const protoJsonBytes = jsonTextBytes + jsonByteStringBytes + structuralBytes + protobuf.fixedOperationBytes;
const messageDepth = deterministic.nestingDepth * protobuf.messageLayersPerMapDepth
  + protobuf.messageOuterLayers;
const tsMessageDepth = messageDepth + protobuf.typescriptExtraRecursionLayers;
const swiftMessageDepth = messageDepth + protobuf.swiftExtraRecursionLayers;
const shared = [
  ["INPUT_LEN", deterministic.inputBytes, "inputBytes"],
  ["OUTPUT_LEN", deterministic.outputBytes, "outputBytes"],
  ["NESTING_DEPTH", deterministic.nestingDepth, "nestingDepth"],
  ["NODES", deterministic.nodes, "nodes"],
  ["CONTAINER_ENTRIES", deterministic.containerEntries, "containerEntries"],
  ["AGGREGATE_TEXT_BYTES", deterministic.aggregateTextBytes, "aggregateTextBytes"],
  ["AGGREGATE_BYTE_STRING_BYTES", deterministic.aggregateByteStringBytes, "aggregateByteStringBytes"],
];
const kotlinNames = new Map([
  ["nestingDepth", "MAX_DETERMINISTIC_CBOR_NESTING_DEPTH"],
  ["nodes", "MAX_DETERMINISTIC_CBOR_NODES"],
  ["containerEntries", "MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES"],
  ["aggregateTextBytes", "MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES"],
  ["aggregateByteStringBytes", "MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES"],
]);
const swiftNames = new Map([
  ["nestingDepth", "maxDeterministicCborNestingDepth"],
  ["nodes", "maxDeterministicCborNodes"],
  ["containerEntries", "maxDeterministicCborContainerEntries"],
  ["aggregateTextBytes", "maxDeterministicCborAggregateTextBytes"],
  ["aggregateByteStringBytes", "maxDeterministicCborAggregateByteStringBytes"],
]);
for (const [suffix, value, property] of shared) {
  const name = `MAX_DETERMINISTIC_CBOR_${suffix}`;
  add("crates/cbor/src/deterministic/limits.rs", `pub const ${name}: usize = `, value);
  add("packages/ts/src/deterministicCborBoundary.ts", `export const ${name} = `, value);
  if (kotlinNames.has(property)) {
    add("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", `private const val ${kotlinNames.get(property)}: Int = `, value, "");
    add("packages/swift/Sources/ReallyMeCodec/DeterministicCbor.swift", `private let ${swiftNames.get(property)} = `, value, "");
  }
}
for (const [suffix, value] of [
  ["MAX_NESTING_DEPTH", dag.nestingDepth],
  ["MAX_DAG_CBOR_NODES", dag.nodes],
  ["MAX_DAG_CBOR_CONTAINER_ENTRIES", dag.containerEntries],
  ["MAX_DAG_CBOR_INPUT_LEN", dag.inputBytes],
]) {
  add("crates/cbor/src/lib.rs", `pub const ${suffix}: usize = `, value);
}
for (const [name, value] of [
  ["CODEC_PROTO_DETERMINISTIC_CBOR_TEXT_BYTES", deterministic.aggregateTextBytes],
  ["CODEC_PROTO_DETERMINISTIC_CBOR_BYTE_STRING_BYTES", deterministic.aggregateByteStringBytes],
  ["CODEC_PROTO_DETERMINISTIC_CBOR_NODES", deterministic.nodes],
  ["CODEC_PROTO_MAX_STRUCTURAL_BYTES_PER_CBOR_NODE", protobuf.structuralBytesPerNode],
  ["CODEC_PROTO_MAX_FIXED_OPERATION_BYTES", protobuf.fixedOperationBytes],
]) {
  add("crates/proto/src/limits.rs", `const ${name}: usize = `, value);
}
add("crates/proto/src/limits.rs", "const CODEC_PROTO_DETERMINISTIC_CBOR_NESTING_DEPTH: u32 = ", deterministic.nestingDepth);
add("crates/proto/src/limits.rs", "const CODEC_PROTO_MESSAGE_LAYERS_PER_CBOR_MAP_DEPTH: u32 = ", protobuf.messageLayersPerMapDepth);
add("crates/proto/src/limits.rs", "const CODEC_PROTO_OUTER_AND_KEY_WRAPPER_LAYERS: u32 = ", protobuf.messageOuterLayers);
add("crates/proto/src/limits.rs", "const CODEC_PROTO_JSON_MAX_TEXT_ESCAPE_EXPANSION: usize = ", protobuf.jsonTextEscapeExpansion);
add("crates/proto/src/limits.rs", "const CODEC_PROTO_JSON_TOKENS_PER_CBOR_NODE: usize = ", protobuf.jsonTokensPerNode);
add("crates/proto/src/limits.rs", "const CODEC_PROTO_JSON_FIXED_OPERATION_TOKENS: usize = ", protobuf.jsonFixedOperationTokens);
add("crates/proto/src/limits.rs", "const CODEC_PROTO_JSON_CONTAINERS_PER_CBOR_MAP_DEPTH: usize = ", protobuf.jsonContainersPerMapDepth);
add("crates/proto/src/limits.rs", "const CODEC_PROTO_JSON_OUTER_AND_KEY_CONTAINERS: usize = ", protobuf.jsonOuterContainers);
add("crates/proto/src/limits.rs", "const CODEC_PROTO_ELEMENT_MEMORY_BYTES_PER_NODE: usize = ", protobuf.elementMemoryBytesPerNode);
for (const [name, value] of [
  ["MAX_CODEC_PROTO_SEMANTIC_NODES", deterministic.nodes],
  ["MAX_CODEC_PROTO_STRUCTURAL_BYTES_PER_NODE", protobuf.structuralBytesPerNode],
  ["MAX_CODEC_PROTO_FIXED_OPERATION_BYTES", protobuf.fixedOperationBytes],
]) {
  add("packages/ts/src/boundary.ts", `const ${name} = `, value);
}
for (const [name, value] of [
  ["MAX_CODEC_PROTO_SENSITIVE_PAYLOAD_BYTES", payloadBytes],
  ["MAX_CODEC_PROTO_JSON_TEXT_BYTES", jsonTextBytes],
  ["MAX_CODEC_PROTO_JSON_BYTE_STRING_BYTES", jsonByteStringBytes],
]) {
  add("packages/ts/src/boundary.ts", `const ${name} = `, value);
}
add("packages/ts/src/deterministicCborBoundary.ts", "export const MAX_DETERMINISTIC_CBOR_PROTO_RECURSION_DEPTH = ", tsMessageDepth);
add("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "private const val MAX_CODEC_PROTO_STRUCTURAL_BYTES_PER_NODE: Int = ", protobuf.structuralBytesPerNode, "");
add("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "private const val MAX_CODEC_PROTO_FIXED_OPERATION_BYTES: Int = ", protobuf.fixedOperationBytes, "");
add("packages/kotlin/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "private const val MAX_DETERMINISTIC_CBOR_PROTO_MESSAGE_DEPTH: Int = ", messageDepth, "");
add("packages/swift/Sources/ReallyMeCodec/CallCodecWithRustCAbi.swift", "private let maxProtoJsonRequestLength = ", protoJsonBytes, "");
add("packages/swift/Sources/ReallyMeCodec/DeterministicCbor.swift", "private let maxCodecProtoStructuralBytesPerDeterministicCborNode = ", protobuf.structuralBytesPerNode, "");
add("packages/swift/Sources/ReallyMeCodec/DeterministicCbor.swift", "private let maxCodecProtoFixedDeterministicCborOperationBytes = ", protobuf.fixedOperationBytes, "");
add("packages/swift/Sources/ReallyMeCodec/DeterministicCbor.swift", "let maxDeterministicCborProtoMessageDepth = ", swiftMessageDepth, "");

for (const [path, entries] of edits) {
  const absolute = resolve(root, path);
  const original = readFileSync(absolute, "utf8");
  let updated = original;
  for (const { declaration, value, terminator } of entries) {
    const lines = updated.split("\n");
    const matching = lines.flatMap((line, index) => line.startsWith(declaration) ? [index] : []);
    if (matching.length !== 1) {
      throw new Error(`${path}: expected exactly one declaration for ${declaration}`);
    }
    const index = matching[0];
    lines[index] = `${declaration}${value}${terminator}`;
    updated = lines.join("\n");
  }
  if (write) {
    if (updated !== original) {
      writeFileSync(absolute, updated);
    }
  } else if (updated !== original) {
    throw new Error(`${path}: codec limits differ from scripts/codec_limits.json`);
  }
}
