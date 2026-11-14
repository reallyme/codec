// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import type {
  ReallyMeDeterministicCborInteger,
  ReallyMeDeterministicCborMapKey,
  ReallyMeDeterministicCborValue,
} from "./cborValues.js";
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
  CodecDeterministicCborMapKey,
  CodecDeterministicCborValue,
} from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import { create } from "@bufbuild/protobuf";

const deterministicCborIntegerToProto = (
  integer: ReallyMeDeterministicCborInteger,
): CodecDeterministicCborInteger => {
  switch (integer.type) {
    case "unsigned":
      return create(CodecDeterministicCborIntegerSchema, {
        value: {
          case: "unsignedValue",
          value: create(CodecDeterministicCborUnsignedIntegerSchema, {
            value: integer.value,
          }),
        },
      });
    case "negative":
      return create(CodecDeterministicCborIntegerSchema, {
        value: {
          case: "negativeValue",
          value: create(CodecDeterministicCborNegativeIntegerSchema, {
            value: integer.value,
          }),
        },
      });
  }
};

const deterministicCborMapKeyToProto = (
  key: ReallyMeDeterministicCborMapKey,
): CodecDeterministicCborMapKey => {
  switch (key.type) {
    case "integer":
      return create(CodecDeterministicCborMapKeySchema, {
        key: {
          case: "integerKey",
          value: deterministicCborIntegerToProto(key.value),
        },
      });
    case "text":
      return create(CodecDeterministicCborMapKeySchema, {
        key: {
          case: "textKey",
          value: create(CodecDeterministicCborTextSchema, { value: key.value }),
        },
      });
  }
};

export const deterministicCborValueToProto = (
  value: ReallyMeDeterministicCborValue,
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
    case "integer":
      return create(CodecDeterministicCborValueSchema, {
        value: {
          case: "integerValue",
          value: deterministicCborIntegerToProto(value.value),
        },
      });
    case "text":
      return create(CodecDeterministicCborValueSchema, {
        value: {
          case: "textValue",
          value: create(CodecDeterministicCborTextSchema, { value: value.value }),
        },
      });
    case "bytes":
      return create(CodecDeterministicCborValueSchema, {
        value: {
          case: "bytesValue",
          value: create(CodecDeterministicCborBytesSchema, { value: value.value }),
        },
      });
    case "array":
      return create(CodecDeterministicCborValueSchema, {
        value: {
          case: "arrayValue",
          value: create(CodecDeterministicCborArraySchema, {
            values: value.value.map(deterministicCborValueToProto),
          }),
        },
      });
    case "map":
      return create(CodecDeterministicCborValueSchema, {
        value: {
          case: "mapValue",
          value: create(CodecDeterministicCborMapSchema, {
            entries: value.value.map((entry) =>
              create(CodecDeterministicCborMapEntrySchema, {
                key: deterministicCborMapKeyToProto(entry.key),
                value: deterministicCborValueToProto(entry.value),
              }),
            ),
          }),
        },
      });
  }
};
