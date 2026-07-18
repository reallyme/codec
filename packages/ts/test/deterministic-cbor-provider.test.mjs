// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import assert from "node:assert/strict";
import { test } from "node:test";
import { create, toBinary } from "@bufbuild/protobuf";
import * as wasm from "../dist/wasm/reallyme_codec_wasm.js";
import {
  ReallyMeCodecError,
  deterministicCborDecode,
  deterministicCborEncode,
  installReallyMeCodecWasmProvider,
} from "../dist/index.js";
import {
  CodecDeterministicCborArraySchema,
  CodecDeterministicCborBytesSchema,
  CodecDeterministicCborDecodeResultSchema,
  CodecDeterministicCborIntegerSchema,
  CodecDeterministicCborMapEntrySchema,
  CodecDeterministicCborMapKeySchema,
  CodecDeterministicCborMapSchema,
  CodecDeterministicCborNegativeIntegerSchema,
  CodecDeterministicCborNullSchema,
  CodecDeterministicCborTextSchema,
  CodecDeterministicCborValueSchema,
  CodecOperationResponseSchema,
  CodecOperationResultSchema,
} from "../dist/proto.js";

const MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES = 16_384;
const MAX_DETERMINISTIC_CBOR_NESTING_DEPTH = 64;

let currentResult = create(CodecDeterministicCborDecodeResultSchema);
let currentHasOutcome = true;
let currentRawOutput;

const provider = {
  ...wasm,
  processOperation() {
    if (currentRawOutput !== undefined) {
      return currentRawOutput;
    }
    if (!currentHasOutcome) {
      return toBinary(CodecOperationResponseSchema, create(CodecOperationResponseSchema));
    }
    return toBinary(
      CodecOperationResponseSchema,
      create(CodecOperationResponseSchema, {
        outcome: {
          case: "result",
          value: create(CodecOperationResultSchema, {
            result: {
              case: "deterministicCborDecode",
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

const nestedArrayValue = (depth) => {
  let value = nullValue();
  for (let index = 0; index < depth; index += 1) {
    value = create(CodecDeterministicCborValueSchema, {
      value: {
        case: "arrayValue",
        value: create(CodecDeterministicCborArraySchema, { values: [value] }),
      },
    });
  }
  return value;
};

const balancedArrayValue = (depth) => {
  if (depth === 0) {
    return nullValue();
  }
  return create(CodecDeterministicCborValueSchema, {
    value: {
      case: "arrayValue",
      value: create(CodecDeterministicCborArraySchema, {
        values: Array.from({ length: 16 }, () => balancedArrayValue(depth - 1)),
      }),
    },
  });
};

const decodeCurrentResult = () => deterministicCborDecode(Uint8Array.of(0xf6));

const assertProviderFailure = (operation) => {
  assert.throws(
    operation,
    (error) =>
      error instanceof ReallyMeCodecError && error.code === "provider-failure",
  );
};

test("deterministic CBOR accepts the documented protobuf recursion depth", () => {
  currentResult = create(CodecDeterministicCborDecodeResultSchema, {
    value: nestedArrayValue(MAX_DETERMINISTIC_CBOR_NESTING_DEPTH),
  });
  let decoded = decodeCurrentResult();
  let depth = 0;
  while (decoded.type === "array") {
    assert.equal(decoded.value.length, 1);
    decoded = decoded.value[0];
    depth += 1;
  }
  assert.equal(depth, MAX_DETERMINISTIC_CBOR_NESTING_DEPTH);
  assert.equal(decoded.type, "null");
});

test("deterministic CBOR rejects over-depth provider output as provider failure", () => {
  currentResult = create(CodecDeterministicCborDecodeResultSchema, {
    value: nestedArrayValue(MAX_DETERMINISTIC_CBOR_NESTING_DEPTH + 1),
  });
  assertProviderFailure(decodeCurrentResult);
});

test("deterministic CBOR rejects oversized provider containers as provider failure", () => {
  currentResult = create(CodecDeterministicCborDecodeResultSchema, {
    value: create(CodecDeterministicCborValueSchema, {
      value: {
        case: "arrayValue",
        value: create(CodecDeterministicCborArraySchema, {
          values: Array.from(
            { length: MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES + 1 },
            nullValue,
          ),
        }),
      },
    }),
  });
  assertProviderFailure(decodeCurrentResult);
});

test("deterministic CBOR rejects provider output over the node budget", () => {
  // 1 + 16 + 256 + 4,096 + 65,536 nodes exceeds the 65,536-node profile
  // without relying on the independent per-container limit.
  currentResult = create(CodecDeterministicCborDecodeResultSchema, {
    value: balancedArrayValue(4),
  });
  try {
    assertProviderFailure(decodeCurrentResult);
  } finally {
    currentResult = create(CodecDeterministicCborDecodeResultSchema);
  }
});

test("deterministic CBOR rejects nested unknown provider fields", () => {
  const value = nullValue();
  value.value.value.$unknown = [
    { data: Uint8Array.of(1), no: 99, wireType: 0 },
  ];
  currentResult = create(CodecDeterministicCborDecodeResultSchema, { value });
  assertProviderFailure(decodeCurrentResult);
});

test("deterministic CBOR rejects duplicate provider map keys", () => {
  const duplicateKey = () =>
    create(CodecDeterministicCborMapKeySchema, {
      key: {
        case: "textKey",
        value: create(CodecDeterministicCborTextSchema, { value: "duplicate" }),
      },
    });
  currentResult = create(CodecDeterministicCborDecodeResultSchema, {
    value: create(CodecDeterministicCborValueSchema, {
      value: {
        case: "mapValue",
        value: create(CodecDeterministicCborMapSchema, {
          entries: [
            create(CodecDeterministicCborMapEntrySchema, {
              key: duplicateKey(),
              value: nullValue(),
            }),
            create(CodecDeterministicCborMapEntrySchema, {
              key: duplicateKey(),
              value: nullValue(),
            }),
          ],
        }),
      },
    }),
  });
  assertProviderFailure(decodeCurrentResult);
});

test("deterministic CBOR rejects invalid or missing provider variants", () => {
  currentResult = create(CodecDeterministicCborDecodeResultSchema);
  assertProviderFailure(decodeCurrentResult);

  currentResult = create(CodecDeterministicCborDecodeResultSchema, {
    value: create(CodecDeterministicCborValueSchema, {
      value: {
        case: "integerValue",
        value: create(CodecDeterministicCborIntegerSchema, {
          value: {
            case: "negativeValue",
            value: create(CodecDeterministicCborNegativeIntegerSchema, {
              value: 0n,
            }),
          },
        }),
      },
    }),
  });
  assertProviderFailure(decodeCurrentResult);
});

test("deterministic CBOR rejects provider aggregate text and byte excess", () => {
  currentResult = create(CodecDeterministicCborDecodeResultSchema, {
    value: create(CodecDeterministicCborValueSchema, {
      value: {
        case: "textValue",
        value: create(CodecDeterministicCborTextSchema, {
          value: "x".repeat(1_048_577),
        }),
      },
    }),
  });
  assertProviderFailure(decodeCurrentResult);

  const oversizedBytes = new Uint8Array(1_048_577);
  try {
    currentResult = create(CodecDeterministicCborDecodeResultSchema, {
      value: create(CodecDeterministicCborValueSchema, {
        value: {
          case: "bytesValue",
          value: create(CodecDeterministicCborBytesSchema, {
            value: oversizedBytes,
          }),
        },
      }),
    });
    assertProviderFailure(decodeCurrentResult);
  } finally {
    oversizedBytes.fill(0);
  }
});

test("deterministic CBOR rejects a provider response without an outcome", () => {
  currentResult = create(CodecDeterministicCborDecodeResultSchema, {
    value: nullValue(),
  });
  currentHasOutcome = false;
  try {
    assertProviderFailure(decodeCurrentResult);
  } finally {
    currentHasOutcome = true;
  }
});

test("deterministic CBOR maps hostile provider byte owners to provider failure", () => {
  const detached = Uint8Array.of(1);
  structuredClone(detached.buffer, { transfer: [detached.buffer] });
  currentRawOutput = detached;
  try {
    assertProviderFailure(decodeCurrentResult);
  } finally {
    currentRawOutput = undefined;
  }

  currentRawOutput = new Proxy(Uint8Array.of(1), {});
  try {
    assertProviderFailure(decodeCurrentResult);
  } finally {
    currentRawOutput = undefined;
  }
});

test("deterministic CBOR rejects an impossible empty encode result", () => {
  currentResult = create(CodecDeterministicCborDecodeResultSchema);
  assertProviderFailure(() => deterministicCborEncode({ type: "null" }));
});
