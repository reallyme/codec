// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

/**
 * Typed codec error codes. Errors intentionally carry no raw input bytes so
 * callers can log them without leaking request payloads.
 */
export type ReallyMeCodecErrorCode =
  | "invalid-input"
  | "non-canonical"
  | "provider-failure"
  | "unsupported-codec";

export class ReallyMeCodecError extends Error {
  readonly code: ReallyMeCodecErrorCode;

  constructor(code: ReallyMeCodecErrorCode) {
    super(code);
    this.name = "ReallyMeCodecError";
    this.code = code;
  }
}
