// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import {
  ensureStringValue,
  readBytesOutput,
  readStringOutput,
  withSnapshotBytesInput,
} from "./readOutput.js";
import { requireReallyMeCodecWasmProvider } from "./wasmProvider.js";

export const base64Encode = (bytes: Uint8Array): string => {
  return withSnapshotBytesInput(bytes, (snapshot) =>
    readStringOutput(requireReallyMeCodecWasmProvider().base64Encode(snapshot)),
  );
};

export const base64Decode = (encoded: string): Uint8Array => {
  ensureStringValue(encoded);
  return readBytesOutput(requireReallyMeCodecWasmProvider().base64Decode(encoded));
};

export const base64urlEncode = (bytes: Uint8Array): string => {
  return withSnapshotBytesInput(bytes, (snapshot) =>
    readStringOutput(requireReallyMeCodecWasmProvider().base64urlEncode(snapshot)),
  );
};

export const base64urlDecode = (encoded: string): Uint8Array => {
  ensureStringValue(encoded);
  return readBytesOutput(requireReallyMeCodecWasmProvider().base64urlDecode(encoded));
};

// The encoded bytes stay in wipeable views end to end; they are never turned
// into an immutable JavaScript string.
export const base64urlDecodeBytes = (encoded: Uint8Array): Uint8Array => {
  return withSnapshotBytesInput(encoded, (snapshot) =>
    readBytesOutput(requireReallyMeCodecWasmProvider().base64urlDecodeBytes(snapshot)),
  );
};

export const bytesToLowerHex = (bytes: Uint8Array): string => {
  return withSnapshotBytesInput(bytes, (snapshot) =>
    readStringOutput(requireReallyMeCodecWasmProvider().bytesToLowerHex(snapshot)),
  );
};

export const lowerHexToBytes = (encoded: string): Uint8Array => {
  ensureStringValue(encoded);
  return readBytesOutput(requireReallyMeCodecWasmProvider().lowerHexToBytes(encoded));
};
