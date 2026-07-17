// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import {
  asciiBytesToString,
  ensureBytesInput,
  ensureStringValue,
  readBytesOutput,
  readStringOutput,
} from "./readOutput.js";
import { requireReallyMeCodecWasmProvider } from "./wasmProvider.js";

export const base64Encode = (bytes: Uint8Array): string => {
  ensureBytesInput(bytes);
  return readStringOutput(requireReallyMeCodecWasmProvider().base64Encode(bytes));
};

export const base64Decode = (encoded: string): Uint8Array => {
  ensureStringValue(encoded);
  return readBytesOutput(requireReallyMeCodecWasmProvider().base64Decode(encoded));
};

export const base64urlEncode = (bytes: Uint8Array): string => {
  ensureBytesInput(bytes);
  return readStringOutput(requireReallyMeCodecWasmProvider().base64urlEncode(bytes));
};

export const base64urlDecode = (encoded: string): Uint8Array => {
  ensureStringValue(encoded);
  return readBytesOutput(requireReallyMeCodecWasmProvider().base64urlDecode(encoded));
};

export const base64urlDecodeBytes = (encoded: Uint8Array): Uint8Array => {
  ensureBytesInput(encoded);
  return base64urlDecode(asciiBytesToString(encoded));
};

export const bytesToLowerHex = (bytes: Uint8Array): string => {
  ensureBytesInput(bytes);
  return readStringOutput(requireReallyMeCodecWasmProvider().bytesToLowerHex(bytes));
};

export const lowerHexToBytes = (encoded: string): Uint8Array => {
  ensureStringValue(encoded);
  return readBytesOutput(requireReallyMeCodecWasmProvider().lowerHexToBytes(encoded));
};
