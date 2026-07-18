// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import test from "node:test";

import {
  sourceBlockFromNeedle,
  stripSourceComments,
  stripSourceStringsAndComments,
} from "./source-policy.mjs";

test("source policy removes comments but preserves strings and line offsets", () => {
  const source = [
    'const url = "https://example.test/path"; // forbiddenCall()',
    "/* outer /* nested */ requiredCall() */",
    "requiredCall();",
  ].join("\n");
  const stripped = stripSourceComments(source, { nestedBlockComments: true });

  assert.equal(stripped.length, source.length);
  assert.equal(stripped.split("\n").length, source.split("\n").length);
  assert.match(stripped, /https:\/\/example\.test\/path/u);
  assert.equal(stripped.includes("forbiddenCall()"), false);
  assert.equal(stripped.match(/requiredCall\(\)/gu)?.length, 1);
});

test("TypeScript block comments do not nest or hide following code", () => {
  const source = [
    "/* note /* nested */",
    "dangerousCall();",
    "*/",
  ].join("\n");
  const stripped = stripSourceComments(source);

  assert.equal(stripped.includes("dangerousCall();"), true);
});

test("regex literals with quotes do not blank following executable code", () => {
  const source = [
    "const quoted = /[\"']/u;",
    "dangerousCall();",
  ].join("\n");
  const stripped = stripSourceStringsAndComments(source);

  assert.equal(stripped.includes("dangerousCall();"), true);
  assert.equal(stripped.includes("[\"']"), false);
});

test("commented architecture calls cannot satisfy a scoped block", () => {
  const block = sourceBlockFromNeedle({
    source: [
      "export const operation = () => {",
      "  // processGeneratedOperationRequest(request);",
      "};",
      "export const nextOperation = () => processGeneratedOperationRequest(request);",
    ].join("\n"),
    startNeedle: "export const operation =",
    nextNeedle: "\nexport const ",
  });

  assert.notEqual(block, undefined);
  assert.equal(block.includes("processGeneratedOperationRequest(request)"), false);
});

test("call-shaped string contents cannot satisfy executable-source policy", () => {
  const source = [
    'const decoy = "processGeneratedOperationRequest(request)";',
    "const real = generatedBoundary(request);",
  ].join("\n");
  const executable = stripSourceStringsAndComments(source);

  assert.equal(executable.includes("processGeneratedOperationRequest(request)"), false);
  assert.equal(executable.includes("generatedBoundary(request)"), true);
});

test("Rust lifetimes remain executable tokens rather than opening strings", () => {
  const source = "fn borrow<'a>(value: &'a str) { semantic_operation(value); }";
  const executable = stripSourceStringsAndComments(source);

  assert.equal(executable.includes("<'a>"), true);
  assert.equal(executable.includes("&'a str"), true);
  assert.equal(executable.includes("semantic_operation(value)"), true);
});

test("required calls elsewhere cannot satisfy the selected operation block", () => {
  const block = sourceBlockFromNeedle({
    source: [
      "fn selected_operation() { direct_primitive_call(); }",
      "fn next_operation() { semantic_operation(); }",
    ].join("\n"),
    startNeedle: "fn selected_operation(",
    nextNeedle: "\nfn ",
  });

  assert.notEqual(block, undefined);
  assert.equal(block.includes("semantic_operation()"), false);
  assert.equal(block.includes("direct_primitive_call()"), true);
});

test("source policy reports a missing executable block", () => {
  const block = sourceBlockFromNeedle({
    source: "// export const operation = () => generatedBoundary();",
    startNeedle: "export const operation =",
    nextNeedle: "\nexport const ",
  });

  assert.equal(block, undefined);
});
