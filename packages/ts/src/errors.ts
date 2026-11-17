// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { isReallyMeCodecErrorCode } from "./proto/generated/sdkContract.js";
import type { ReallyMeCodecErrorCode } from "./proto/generated/sdkContract.js";
import { CodecErrorOrigin, CodecErrorReason } from "./proto/generated/reallyme/codec/v1/codec_pb.js";
export type { ReallyMeCodecErrorCode } from "./proto/generated/sdkContract.js";
export { CodecErrorOrigin, CodecErrorReason } from "./proto/generated/reallyme/codec/v1/codec_pb.js";

const knownReasons: ReadonlySet<unknown> = new Set(Object.values(CodecErrorReason)
  .filter((reason) => typeof reason === "number" && reason !== CodecErrorReason.UNSPECIFIED));
const knownOrigins: ReadonlySet<unknown> = new Set(Object.values(CodecErrorOrigin)
  .filter((origin) => typeof origin === "number" && origin !== CodecErrorOrigin.UNSPECIFIED));

/** Typed codec errors never publish untrusted diagnostic text. */
export class ReallyMeCodecError extends Error {
  readonly code: ReallyMeCodecErrorCode;
  readonly reason: CodecErrorReason | undefined;
  readonly origin: CodecErrorOrigin | undefined;

  constructor(code: ReallyMeCodecErrorCode, reason?: CodecErrorReason, origin?: CodecErrorOrigin) {
    const checkedCode = isReallyMeCodecErrorCode(code) ? code : "provider-failure";
    super(checkedCode);
    this.name = "ReallyMeCodecError";
    this.code = checkedCode;
    // Local platform failures have no received protobuf context. Keep metadata
    // absent unless both generated enum values are recognized, including when
    // a JavaScript caller bypasses the constructor's TypeScript signature.
    const hasContext = knownReasons.has(reason) && knownOrigins.has(origin);
    this.reason = hasContext ? reason : undefined;
    this.origin = hasContext ? origin : undefined;
  }
}
