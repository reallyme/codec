// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import {
  CodecBoundaryErrorSchema,
  CodecErrorOrigin,
  CodecErrorReason,
  CodecErrorSchema,
  CodecOperationResponseSchema,
} from "./proto/generated/reallyme/codec/v1/codec_pb.js";
import { create, toBinary } from "@bufbuild/protobuf";

export const PROTO_REQUEST_OVERHEAD_BUDGET = 64;

export const DETERMINISTIC_CBOR_PROTO_NODE_OVERHEAD_BUDGET = 16;

export const DAG_CBOR_I64_MAX = (1n << 63n) - 1n;

export const resourceLimitOperationResponse = (): Uint8Array =>
  toBinary(
    CodecOperationResponseSchema,
    create(CodecOperationResponseSchema, {
      outcome: {
        case: "error",
        value: create(CodecErrorSchema, {
          error: {
            case: "boundary",
            value: create(CodecBoundaryErrorSchema, {
              reason: CodecErrorReason.BOUNDARY_RESOURCE_LIMIT_EXCEEDED,
            }),
          },
          origin: CodecErrorOrigin.CALLER,
        }),
      },
    }),
  );
