// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { isReallyMeCodecErrorCode } from "./proto/generated/sdkContract.js";
import type { ReallyMeCodecErrorCode } from "./proto/generated/sdkContract.js";
export type { ReallyMeCodecErrorCode } from "./proto/generated/sdkContract.js";

/** Typed codec errors never publish untrusted diagnostic text. */
export class ReallyMeCodecError extends Error {
  readonly code: ReallyMeCodecErrorCode;

  constructor(code: ReallyMeCodecErrorCode) {
    const checkedCode = isReallyMeCodecErrorCode(code) ? code : "provider-failure";
    super(checkedCode);
    this.name = "ReallyMeCodecError";
    this.code = checkedCode;
  }
}
