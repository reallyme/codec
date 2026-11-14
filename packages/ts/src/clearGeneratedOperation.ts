// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import type { CodecDeterministicCborValue, CodecOperationResult } from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import { snapshotProviderBytes } from "./readOutput.js";

// Buf's binary decoder may expose byte fields as views into the serialized
// response owner. Detach every byte field before that owner is wiped so the
// returned generated result remains valid and has independent cleanup.
export const detachGeneratedOperationResultBytes = (
  result: CodecOperationResult,
): void => {
  switch (result.result.case) {
    case "multicodecPrefixForName":
      result.result.value.prefix = snapshotProviderBytes(result.result.value.prefix);
      break;
    case "multicodecLookupPrefix":
      if (result.result.value.metadata !== undefined) {
        result.result.value.metadata.prefix =
          snapshotProviderBytes(result.result.value.metadata.prefix);
      }
      break;
    case "multicodecTable":
      for (const entry of result.result.value.entries) {
        entry.prefix = snapshotProviderBytes(entry.prefix);
      }
      break;
    case "multikeyParse":
      result.result.value.publicKey = snapshotProviderBytes(result.result.value.publicKey);
      break;
    case "dagCborVerifyCid":
      break;
    case "dagCborEncode":
      result.result.value.encoded = snapshotProviderBytes(result.result.value.encoded);
      break;
    case "dagCborDecode":
      detachGeneratedCborValueBytes(result.result.value.value);
      break;
    case "pemDecode":
      result.result.value.der = snapshotProviderBytes(result.result.value.der);
      break;
    case "pemEncode":
      result.result.value.pem = snapshotProviderBytes(result.result.value.pem);
      break;
    case "deterministicCborEncode":
      result.result.value.encoded = snapshotProviderBytes(result.result.value.encoded);
      break;
    case "deterministicCborDecode":
      detachGeneratedCborValueBytes(result.result.value.value);
      break;
    case undefined:
      break;
  }
};

const detachGeneratedCborValueBytes = (
  value: CodecDeterministicCborValue | undefined,
): void => {
  if (value === undefined) {
    return;
  }
  switch (value.value.case) {
    case "bytesValue":
      value.value.value.value = snapshotProviderBytes(value.value.value.value);
      break;
    case "arrayValue":
      for (const child of value.value.value.values) {
        detachGeneratedCborValueBytes(child);
      }
      break;
    case "mapValue":
      for (const entry of value.value.value.entries) {
        detachGeneratedCborValueBytes(entry.value);
      }
      break;
    case "nullValue":
    case "boolValue":
    case "integerValue":
    case "textValue":
    case undefined:
      break;
  }
};

/**
 * Clears mutable byte owners held by a generated operation result.
 *
 * Callers invoke this before rejecting an unexpected valid oneof variant and
 * after copying a sensitive result into its public SDK owner. JavaScript
 * strings remain subject to the managed-runtime memory model, but every
 * mutable `Uint8Array` created by this adapter is released promptly.
 */
export const clearGeneratedOperationResult = (
  result: CodecOperationResult,
): void => {
  switch (result.result.case) {
    case "multicodecPrefixForName":
      result.result.value.prefix.fill(0);
      break;
    case "multicodecLookupPrefix":
      result.result.value.metadata?.prefix.fill(0);
      break;
    case "multicodecTable":
      for (const entry of result.result.value.entries) {
        entry.prefix.fill(0);
      }
      break;
    case "multikeyParse":
      result.result.value.publicKey.fill(0);
      break;
    case "dagCborVerifyCid":
      break;
    case "dagCborEncode":
      result.result.value.encoded.fill(0);
      break;
    case "dagCborDecode":
      wipeGeneratedCborValue(result.result.value.value);
      break;
    case "pemDecode":
      result.result.value.der.fill(0);
      break;
    case "pemEncode":
      result.result.value.pem.fill(0);
      break;
    case "deterministicCborEncode":
      result.result.value.encoded.fill(0);
      break;
    case "deterministicCborDecode":
      wipeGeneratedCborValue(result.result.value.value);
      break;
    case undefined:
      break;
  }
  result.result = { case: undefined };
};

const wipeGeneratedCborValue = (
  value: CodecDeterministicCborValue | undefined,
): void => {
  if (value === undefined) {
    return;
  }
  switch (value.value.case) {
    case "bytesValue":
      value.value.value.value.fill(0);
      break;
    case "arrayValue":
      for (const child of value.value.value.values) {
        wipeGeneratedCborValue(child);
      }
      break;
    case "mapValue":
      for (const entry of value.value.value.entries) {
        wipeGeneratedCborValue(entry.value);
      }
      break;
    case "nullValue":
    case "boolValue":
    case "integerValue":
    case "textValue":
    case undefined:
      break;
  }
  value.value = { case: undefined };
};
