// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import type {
  ReallyMeCborMapEntry,
  ReallyMeCborValue,
  ReallyMeDeterministicCborValue,
} from "./cborValues.js";
import { wipeProtoDeterministicCborValueBytes } from "./clearCbor.js";
import { MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES } from "./deterministicCborBoundary.js";
import {
  CodecDeterministicCborArraySchema,
  CodecDeterministicCborBoolSchema,
  CodecDeterministicCborBytesSchema,
  CodecDeterministicCborIntegerSchema,
  CodecDeterministicCborMapEntrySchema,
  CodecDeterministicCborMapKeySchema,
  CodecDeterministicCborMapSchema,
  CodecDeterministicCborNegativeIntegerSchema,
  CodecDeterministicCborNullSchema,
  CodecDeterministicCborTextSchema,
  CodecDeterministicCborUnsignedIntegerSchema,
  CodecDeterministicCborValueSchema,
} from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import type {
  CodecDeterministicCborInteger,
  CodecDeterministicCborMapEntry,
  CodecDeterministicCborValue,
} from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import { providerFailure } from "./readDeterministicCborProto.js";
import { snapshotProviderBytes } from "./readOutput.js";
import { snapshotDeterministicCborBytes } from "./snapshotDeterministicCbor.js";
import { i64Max, i64Min, invalidCborInput, wipeDagCborValueBytes } from "./validateDagCbor.js";
import { create } from "@bufbuild/protobuf";

const dagCborIntegerToProto = (
  value: number | bigint,
): CodecDeterministicCborInteger => {
  const integer = typeof value === "bigint" ? value : BigInt(value);
  if (integer < i64Min || integer > i64Max) {
    return invalidCborInput();
  }
  if (integer < 0n) {
    return create(CodecDeterministicCborIntegerSchema, {
      value: {
        case: "negativeValue",
        value: create(CodecDeterministicCborNegativeIntegerSchema, {
          value: integer,
        }),
      },
    });
  }
  return create(CodecDeterministicCborIntegerSchema, {
    value: {
      case: "unsignedValue",
      value: create(CodecDeterministicCborUnsignedIntegerSchema, {
        value: integer,
      }),
    },
  });
};

export const dagCborValueToProto = (
  value: ReallyMeCborValue,
): CodecDeterministicCborValue => {
  switch (value.type) {
    case "null":
      return create(CodecDeterministicCborValueSchema, {
        value: {
          case: "nullValue",
          value: create(CodecDeterministicCborNullSchema),
        },
      });
    case "bool":
      return create(CodecDeterministicCborValueSchema, {
        value: {
          case: "boolValue",
          value: create(CodecDeterministicCborBoolSchema, { value: value.value }),
        },
      });
    case "int":
      return create(CodecDeterministicCborValueSchema, {
        value: {
          case: "integerValue",
          value: dagCborIntegerToProto(value.value),
        },
      });
    case "string":
      return create(CodecDeterministicCborValueSchema, {
        value: {
          case: "textValue",
          value: create(CodecDeterministicCborTextSchema, { value: value.value }),
        },
      });
    case "bytes": {
      const bytes = snapshotDeterministicCborBytes(
        value.value,
        MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
      );
      try {
        const bytesValue = create(CodecDeterministicCborBytesSchema, { value: bytes });
        return create(CodecDeterministicCborValueSchema, {
          value: {
            case: "bytesValue",
            value: bytesValue,
          },
        });
      } catch (error: unknown) {
        bytes.fill(0);
        throw error;
      }
    }
    case "array": {
      const values: CodecDeterministicCborValue[] = [];
      try {
        for (const child of value.value) {
          values.push(dagCborValueToProto(child));
        }
        return create(CodecDeterministicCborValueSchema, {
          value: {
            case: "arrayValue",
            value: create(CodecDeterministicCborArraySchema, { values }),
          },
        });
      } catch (error: unknown) {
        for (const child of values) {
          wipeProtoDeterministicCborValueBytes(child);
        }
        throw error;
      }
    }
    case "map": {
      const entries: CodecDeterministicCborMapEntry[] = [];
      try {
        for (const entry of value.value) {
          const entryValue = dagCborValueToProto(entry.value);
          try {
            entries.push(
              create(CodecDeterministicCborMapEntrySchema, {
                key: create(CodecDeterministicCborMapKeySchema, {
                  key: {
                    case: "textKey",
                    value: create(CodecDeterministicCborTextSchema, {
                      value: entry.key,
                    }),
                  },
                }),
                value: entryValue,
              }),
            );
          } catch (error: unknown) {
            wipeProtoDeterministicCborValueBytes(entryValue);
            throw error;
          }
        }
        return create(CodecDeterministicCborValueSchema, {
          value: {
            case: "mapValue",
            value: create(CodecDeterministicCborMapSchema, { entries }),
          },
        });
      } catch (error: unknown) {
        for (const entry of entries) {
          wipeProtoDeterministicCborValueBytes(entry.value);
        }
        throw error;
      }
    }
  }
};

export const dagCborValueFromDeterministic = (
  value: ReallyMeDeterministicCborValue,
): ReallyMeCborValue => {
  switch (value.type) {
    case "null":
      return { type: "null" };
    case "bool":
      return { type: "bool", value: value.value };
    case "integer": {
      const integer = value.value.value;
      if (integer < i64Min || integer > i64Max) {
        return providerFailure();
      }
      return { type: "int", value: integer };
    }
    case "text":
      return { type: "string", value: value.value };
    case "bytes":
      return {
        type: "bytes",
        value: snapshotProviderBytes(
          value.value,
          MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
        ),
      };
    case "array": {
      const values: ReallyMeCborValue[] = [];
      try {
        for (const child of value.value) {
          values.push(dagCborValueFromDeterministic(child));
        }
        return {
          type: "array",
          value: values,
        };
      } catch (error: unknown) {
        for (const child of values) {
          wipeDagCborValueBytes(child);
        }
        throw error;
      }
    }
    case "map": {
      const entries: ReallyMeCborMapEntry[] = [];
      try {
        for (const entry of value.value) {
          if (entry.key.type !== "text") {
            return providerFailure();
          }
          entries.push({
            key: entry.key.value,
            value: dagCborValueFromDeterministic(entry.value),
          });
        }
        return {
          type: "map",
          value: entries,
        };
      } catch (error: unknown) {
        for (const entry of entries) {
          wipeDagCborValueBytes(entry.value);
        }
        throw error;
      }
    }
  }
};
