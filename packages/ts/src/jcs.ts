// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { requireBoundaryUtf8String, stringifyBoundaryJson } from "./boundary.js";
import { readStringOutput } from "./readOutput.js";
import { requireReallyMeCodecWasmProvider } from "./wasmProvider.js";

export const canonicalizeJson = (value: unknown): string => {
  const encoded = stringifyBoundaryJson(value);
  return readStringOutput(requireReallyMeCodecWasmProvider().canonicalizeJson(encoded));
};

export const canonicalizeJsonText = (json: string): string => {
  requireBoundaryUtf8String(json, false);
  return readStringOutput(requireReallyMeCodecWasmProvider().canonicalizeJson(json));
};
