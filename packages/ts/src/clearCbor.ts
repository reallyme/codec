// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import type { ReallyMeDeterministicCborValue } from "./cborValues.js";
import type { CodecDeterministicCborValue } from "./proto/generated/reallyme/codec/v1/codec_pb.js";

export const wipeDeterministicCborValueBytes = (
  value: ReallyMeDeterministicCborValue,
): void => {
  switch (value.type) {
    case "bytes":
      value.value.fill(0);
      break;
    case "array":
      for (const child of value.value) {
        wipeDeterministicCborValueBytes(child);
      }
      break;
    case "map":
      for (const entry of value.value) {
        wipeDeterministicCborValueBytes(entry.value);
      }
      break;
    case "null":
    case "bool":
    case "integer":
    case "text":
      break;
  }
};

export const wipeProtoDeterministicCborValueBytes = (
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
        wipeProtoDeterministicCborValueBytes(child);
      }
      break;
    case "mapValue":
      for (const entry of value.value.value.entries) {
        wipeProtoDeterministicCborValueBytes(entry.value);
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
