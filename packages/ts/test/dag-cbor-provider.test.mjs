// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import { test } from "node:test";
import { create, toBinary } from "@bufbuild/protobuf";
import * as wasm from "../dist/wasm/reallyme_codec_wasm.js";
import {
  ReallyMeCodecError,
  dagCborDecode,
  installReallyMeCodecWasmProvider,
} from "../dist/index.js";
import {
  CodecDagCborDecodeResultSchema,
  CodecDeterministicCborArraySchema,
  CodecDeterministicCborBytesSchema,
  CodecDeterministicCborIntegerSchema,
  CodecDeterministicCborMapEntrySchema,
  CodecDeterministicCborMapKeySchema,
  CodecDeterministicCborMapSchema,
  CodecDeterministicCborNullSchema,
  CodecDeterministicCborUnsignedIntegerSchema,
  CodecDeterministicCborValueSchema,
  CodecOperationResponseSchema,
  CodecOperationResultSchema,
} from "../dist/proto.js";

let currentResult = create(CodecDagCborDecodeResultSchema);

const provider = {
  ...wasm,
  processOperation() {
    return toBinary(
      CodecOperationResponseSchema,
      create(CodecOperationResponseSchema, {
        outcome: {
          case: "result",
          value: create(CodecOperationResultSchema, {
            result: {
              case: "dagCborDecode",
              value: currentResult,
            },
          }),
        },
      }),
    );
  },
};

installReallyMeCodecWasmProvider(provider);

const nullValue = () =>
  create(CodecDeterministicCborValueSchema, {
    value: {
      case: "nullValue",
      value: create(CodecDeterministicCborNullSchema),
    },
  });

const assertProviderFailure = (operation) => {
  assert.throws(
    operation,
    (error) =>
      error instanceof ReallyMeCodecError && error.code === "provider-failure",
  );
};

test("DAG-CBOR decode wipes partially built byte nodes when a later node fails", () => {
  const sensitiveBytes = Uint8Array.of(1, 2, 3);
  currentResult = create(CodecDagCborDecodeResultSchema, {
    value: create(CodecDeterministicCborValueSchema, {
      value: {
        case: "arrayValue",
        value: create(CodecDeterministicCborArraySchema, {
          values: [
            create(CodecDeterministicCborValueSchema, {
              value: {
                case: "bytesValue",
                value: create(CodecDeterministicCborBytesSchema, {
                  value: sensitiveBytes,
                }),
              },
            }),
            create(CodecDeterministicCborValueSchema, {
              value: {
                case: "mapValue",
                value: create(CodecDeterministicCborMapSchema, {
                  entries: [
                    create(CodecDeterministicCborMapEntrySchema, {
                      key: create(CodecDeterministicCborMapKeySchema, {
                        key: {
                          case: "integerKey",
                          value: create(CodecDeterministicCborIntegerSchema, {
                            value: {
                              case: "unsignedValue",
                              value: create(
                                CodecDeterministicCborUnsignedIntegerSchema,
                                { value: 1n },
                              ),
                            },
                          }),
                        },
                      }),
                      value: nullValue(),
                    }),
                  ],
                }),
              },
            }),
          ],
        }),
      },
    }),
  });

  const originalFill = Uint8Array.prototype.fill;
  const wipedSensitiveOwners = [];
  Uint8Array.prototype.fill = function patchedFill(value, start, end) {
    if (
      this.length === 3 &&
      this[0] === 1 &&
      this[1] === 2 &&
      this[2] === 3
    ) {
      wipedSensitiveOwners.push(this);
    }
    return Reflect.apply(originalFill, this, [value, start, end]);
  };
  try {
    assertProviderFailure(() => dagCborDecode(Uint8Array.of(0x80)));
  } finally {
    Uint8Array.prototype.fill = originalFill;
    currentResult = create(CodecDagCborDecodeResultSchema);
  }

  assert.equal(wipedSensitiveOwners.length >= 3, true);
});
