#!/usr/bin/env node
// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { readFileSync, readdirSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { join } from "node:path";
const check = process.argv.length === 3 && process.argv[2] === "--check";
if (process.argv.length !== 2 && !check) {
  console.error("codec protocol generation failed: unsupported arguments");
  process.exit(1);
}
function writeArtifact(path, source) {
  if (check) {
    if (readFileSync(path, "utf8") !== source) fail("schema-derived support artifact is stale");
  } else {
    writeFileSync(path, source);
  }
}
import { root, protoSource, fail, scrubProtoCommentsAndStrings,
  messageFields, validateScalarFieldClassifications,
} from "./codec_proto_schema.mjs";
const rustContractErrorTraitsPath = join(root, "crates/proto/src/generated/contract_error_traits.rs");
const rustWirePolicyPath = join(root, "crates/proto/src/generated/wire_policy.rs");
function generateContractErrorTraits() {
  const names = [
    "CodecGeneralCborErrorReason", "CodecDerErrorReason", "CodecTlvErrorReason",
    "CodecGeneralCborLengthPolicy", "CodecGeneralCborSerialization",
    "CodecGeneralCborFloatPolicy", "CodecGeneralCborMapOrdering",
    "CodecGeneralCborFloatWidth", "CodecGeneralCborMajorType", "CodecGeneralCborArgumentWidth",
  ];
  let generated = `// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

// Generated from the fixed error text in codec.proto. Do not edit.
${names.map((name) => `use super::proto::reallyme::codec::v1::${name};`).join("\n")}
`;
  for (const name of names) {
    const match = new RegExp(`enum ${name} \\{([\\s\\S]*?)\\n\\}`, "u").exec(protoSource);
    if (match === null) {
      fail(`missing schema error enum ${name}`);
    }
    const prefix = name.replace(/([a-z0-9])([A-Z])/gu, "$1_$2").toUpperCase();
    const entries = [{ constant: `${prefix}_UNSPECIFIED`, debug: "Unspecified", display: "unspecified error reason" }];
    const isError = name.endsWith("ErrorReason");
    for (const item of match[1].matchAll(/\/\/ Error text: ([^\n]+)\n\s*([A-Z][A-Z0-9_]*)\s*=\s*[0-9]+;/gu)) {
      if (!/^[\x20-\x7e]+$/u.test(item[1]) || !item[2].startsWith(`${prefix}_`)) {
        fail(`invalid fixed error metadata in ${name}`);
      }
      const suffix = item[2].slice(prefix.length + 1);
      entries.push({
        constant: item[2],
        debug: suffix.toLowerCase().split("_").map((part) => part[0].toUpperCase() + part.slice(1)).join(""),
        display: item[1],
      });
    }
    const declarations = [...match[1].matchAll(/^\s*([A-Z][A-Z0-9_]*)\s*=\s*-?[0-9]+;/gmu)];
    if (!isError) {
      entries.splice(0, entries.length, ...declarations.map((item) => ({
        constant: item[1],
        debug: item[1].slice(prefix.length + 1).toLowerCase().split("_")
          .map((part) => part[0].toUpperCase() + part.slice(1)).join(""),
      })));
    }
    if (entries.length !== declarations.length) {
      fail(`every error value in ${name} must declare fixed error text`);
    }
    for (const [trait, property] of (isError ? [["Display", "display"], ["Debug", "debug"]] : [["Debug", "debug"]])) {
      const arms = entries.map((entry) => `Self::${entry.constant} => ${JSON.stringify(entry[property])},`).join("\n");
      generated += `\nimpl ::core::fmt::${trait} for ${name} {
fn fmt(&self, formatter: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
let label = match self { ${arms} };
formatter.write_str(label)
}
}\n`;
    }
    if (isError) {
      generated += `\nimpl ::std::error::Error for ${name} {}\n`;
    }
  }
  const formatted = spawnSync("rustfmt", ["--edition", "2021", "--emit", "stdout"], {
    input: generated, encoding: "utf8", stdio: ["pipe", "pipe", "pipe"],
  });
  if (formatted.error !== undefined || formatted.status !== 0 || typeof formatted.stdout !== "string") {
    fail("formatting generated contract error traits failed");
  }
  writeArtifact(rustContractErrorTraitsPath, formatted.stdout);
}

// Structural preflight constants are derived from the same messages as Buffa.
// A handwritten selector list could omit a new operation's allocation guards.
function formatGeneratedRust(source) {
  const result = spawnSync("rustfmt", ["--edition", "2021", "--emit", "stdout"], {
    input: source, encoding: "utf8", stdio: ["pipe", "pipe", "pipe"],
  });
  if (result.error !== undefined || result.status !== 0 || typeof result.stdout !== "string") {
    fail("formatting generated wire policy failed");
  }
  return result.stdout;
}

function generateWirePolicy() {
  const operations = messageFields("CodecApiOperationRequest");
  if (operations.length === 0 || operations.some((field) => !Number.isSafeInteger(field.number) || field.number <= 0)) {
    fail("invalid generated operation preflight numbers");
  }
  const fieldNumber = (message, name) => {
    const field = messageFields(message).find((candidate) => candidate.name === name);
    if (field === undefined || !Number.isSafeInteger(field.number)) fail(`missing wire-policy field ${message}.${name}`);
    return field.number;
  };
  const labels = /enum CodecPemLabel \{([\s\S]*?)\n\}/u.exec(protoSource)?.[1];
  if (labels === undefined) fail("missing PEM label enum");
  const labelCount = [...labels.matchAll(/^\s*CODEC_PEM_LABEL_\w+\s*=\s*([1-9][0-9]*)\s*;/gmu)].length;
  if (labelCount === 0) fail("missing supported PEM label values");
  writeArtifact(rustWirePolicyPath, formatGeneratedRust(`// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

// Generated from codec.proto. Do not edit.

/// All operation tags guarded before generated request allocation.
pub const OPERATION_FIELD_NUMBERS: &[u64] = &[${operations.map((field) => field.number).sort((left, right) => left - right).join(", ")}];
/// PEM decode operation tag.
pub const PEM_DECODE_OPERATION_FIELD: u64 = ${fieldNumber("CodecApiOperationRequest", "pem_decode")};
/// PEM decode options tag.
pub const PEM_DECODE_OPTIONS_FIELD: u64 = ${fieldNumber("CodecPemDecodeRequest", "options")};
/// PEM accepted-label list tag.
pub const PEM_ALLOWED_LABELS_FIELD: u64 = ${fieldNumber("CodecPemDecodeOptions", "allowed_labels")};
/// Number of nonzero PEM label values in the schema.
pub const MAX_PEM_ALLOWED_LABELS: usize = ${labelCount};
`));
}


function generateExports() {
  const schema = scrubProtoCommentsAndStrings(protoSource);
  const names = [...schema.matchAll(/^(?:message|enum)\s+(Codec\w+)\s*\{/gmu)].map((match) => match[1]);
  const packageSource = readFileSync(join(root, "crates/proto/src/generated/buffa/reallyme.codec.v1.mod.rs"), "utf8");
  for (const match of packageSource.matchAll(/pub use self::__buffa::(?:oneof|view|view_oneof)::(\w+);/gu)) {
    names.push(match[1]);
  }
  const unique = [...new Set(names)].sort();
  const imports = unique.join(", ");
  const oneofSource = readFileSync(join(root, "crates/proto/src/generated/buffa/reallyme.codec.v1.codec.__oneof.rs"), "utf8");
  const oneofModules = [...oneofSource.matchAll(/^pub mod (\w+) \{/gmu)].map((match) => match[1]).join(", ");
  const source = `// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

// Generated schema-derived public exports. Raw generated modules remain private.
pub mod reallyme { pub mod codec { pub mod v1 {
pub use super::super::super::super::raw_proto::reallyme::codec::v1::{${imports}};
pub use super::super::super::super::raw_proto::reallyme::codec::v1::__buffa::oneof::{${oneofModules}};
/// Generated oneof types preserve the established contract import paths.
pub mod __buffa { pub use super::super::super::super::super::raw_proto::reallyme::codec::v1::__buffa::oneof; }
}}}
`;
  writeArtifact(join(root, "crates/proto/src/generated/contract_exports.rs"), formatGeneratedRust(source));
}
validateScalarFieldClassifications();
generateContractErrorTraits();
generateWirePolicy();
generateExports();
