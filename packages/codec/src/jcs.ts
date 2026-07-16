// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { ReallyMeCodecError } from "./errors.js";
import { readStringOutput } from "./readOutput.js";
import { requireReallyMeCodecWasmProvider } from "./wasmProvider.js";

const maxJsonContainerDepth = 128;

const validateJsonCompatible = (
  value: unknown,
  depth = 0,
  seen: WeakSet<object> = new WeakSet<object>(),
): void => {
  if (value === null) {
    return;
  }
  switch (typeof value) {
    case "boolean":
    case "string":
      return;
    case "number":
      if (!Number.isFinite(value)) {
        throw new ReallyMeCodecError("invalid-input");
      }
      return;
    case "bigint":
    case "function":
    case "symbol":
    case "undefined":
      throw new ReallyMeCodecError("invalid-input");
    case "object":
      break;
  }

  if (seen.has(value)) {
    throw new ReallyMeCodecError("invalid-input");
  }
  if (depth >= maxJsonContainerDepth) {
    throw new ReallyMeCodecError("invalid-input");
  }
  if (typeof Reflect.get(value, "toJSON") === "function") {
    throw new ReallyMeCodecError("invalid-input");
  }

  const prototype = Object.getPrototypeOf(value);
  if (prototype !== Object.prototype && prototype !== null && !Array.isArray(value)) {
    throw new ReallyMeCodecError("invalid-input");
  }

  seen.add(value);
  try {
    if (Array.isArray(value)) {
      for (const entry of value) {
        validateJsonCompatible(entry, depth + 1, seen);
      }
      return;
    }
    for (const key of Object.keys(value)) {
      validateJsonCompatible(Reflect.get(value, key), depth + 1, seen);
    }
  } finally {
    seen.delete(value);
  }
};

export const canonicalizeJson = (value: unknown): string => {
  validateJsonCompatible(value);
  const encoded = JSON.stringify(value);
  if (encoded === undefined) {
    throw new ReallyMeCodecError("invalid-input");
  }
  return readStringOutput(requireReallyMeCodecWasmProvider().canonicalizeJson(encoded));
};

export const canonicalizeJsonText = (json: string): string => {
  if (typeof json !== "string" || json.length === 0) {
    throw new ReallyMeCodecError("invalid-input");
  }
  return readStringOutput(requireReallyMeCodecWasmProvider().canonicalizeJson(json));
};
