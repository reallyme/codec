// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { MAX_CODEC_PROTO_MESSAGE_BYTES } from "./boundary.js";
import { ReallyMeCodecError } from "./errors.js";

/**
 * One field of an operation request body.
 *
 * - `varint` and `bytes` fields use proto3 implicit presence: zero and empty
 *   values are omitted, exactly as protobuf-es serializes them.
 * - `message` fields carry an already serialized submessage and are always
 *   written, because a set message field has explicit presence.
 */
export type OperationRequestField =
  | Readonly<{ field: number; varint: number }>
  | Readonly<{ field: number; bytes: Uint8Array }>
  | Readonly<{ field: number; message: Uint8Array }>;

const WIRE_TYPE_VARINT = 0;
const WIRE_TYPE_LENGTH_DELIMITED = 2;
const VARINT_CONTINUATION = 0x80;
const VARINT_RADIX = 128;
const MAX_FIELD_NUMBER = 536_870_911;
const MAX_VARINT_VALUE = 0xffff_ffff;

const requireVarintValue = (value: number): void => {
  if (!Number.isSafeInteger(value) || value < 0 || value > MAX_VARINT_VALUE) {
    throw new ReallyMeCodecError("invalid-input");
  }
};

const fieldTag = (field: number, wireType: number): number => {
  if (!Number.isSafeInteger(field) || field < 1 || field > MAX_FIELD_NUMBER) {
    throw new ReallyMeCodecError("provider-failure");
  }
  return field * 8 + wireType;
};

const varintLength = (value: number): number => {
  let length = 1;
  let rest = value;
  while (rest >= VARINT_CONTINUATION) {
    rest = Math.floor(rest / VARINT_RADIX);
    length += 1;
  }
  return length;
};

const writeVarint = (target: Uint8Array, offset: number, value: number): number => {
  let rest = value;
  let cursor = offset;
  while (rest >= VARINT_CONTINUATION) {
    target[cursor] = (rest % VARINT_RADIX) | VARINT_CONTINUATION;
    rest = Math.floor(rest / VARINT_RADIX);
    cursor += 1;
  }
  target[cursor] = rest;
  return cursor + 1;
};

const isOmitted = (part: OperationRequestField): boolean => {
  if ("varint" in part) {
    return part.varint === 0;
  }
  if ("bytes" in part) {
    return part.bytes.length === 0;
  }
  return false;
};

const partPayload = (part: OperationRequestField): Uint8Array | undefined => {
  if ("bytes" in part) {
    return part.bytes;
  }
  if ("message" in part) {
    return part.message;
  }
  return undefined;
};

const encodedPartLength = (part: OperationRequestField): number => {
  const payload = partPayload(part);
  if (payload === undefined) {
    if (!("varint" in part)) {
      throw new ReallyMeCodecError("provider-failure");
    }
    requireVarintValue(part.varint);
    return varintLength(fieldTag(part.field, WIRE_TYPE_VARINT)) + varintLength(part.varint);
  }
  return (
    varintLength(fieldTag(part.field, WIRE_TYPE_LENGTH_DELIMITED)) +
    varintLength(payload.length) +
    payload.length
  );
};

/**
 * Serialize one `CodecOperationRequest` whose oneof `operationField` holds a
 * message built from `fields`, into a single exactly sized buffer.
 *
 * The protobuf-es writer grows and abandons internal buffers that keep a copy
 * of every byte field. Sensitive operations frame their request here instead,
 * so the only copy of secret bytes is the returned buffer, which the caller
 * owns and must wipe. Fields are written in the given order; callers pass them
 * in field-number order to match protobuf-es output byte for byte.
 */
export const frameOperationRequest = (
  operationField: number,
  fields: ReadonlyArray<OperationRequestField>,
): Uint8Array => {
  const present = fields.filter((part) => !isOmitted(part));
  let bodyLength = 0;
  for (const part of present) {
    bodyLength += encodedPartLength(part);
  }
  const operationTag = fieldTag(operationField, WIRE_TYPE_LENGTH_DELIMITED);
  const totalLength = varintLength(operationTag) + varintLength(bodyLength) + bodyLength;
  if (!Number.isSafeInteger(totalLength) || totalLength > MAX_CODEC_PROTO_MESSAGE_BYTES) {
    throw new ReallyMeCodecError("invalid-input");
  }

  const output = new Uint8Array(totalLength);
  let offset = writeVarint(output, 0, operationTag);
  offset = writeVarint(output, offset, bodyLength);
  for (const part of present) {
    const payload = partPayload(part);
    if (payload === undefined) {
      if (!("varint" in part)) {
        throw new ReallyMeCodecError("provider-failure");
      }
      offset = writeVarint(output, offset, fieldTag(part.field, WIRE_TYPE_VARINT));
      offset = writeVarint(output, offset, part.varint);
    } else {
      offset = writeVarint(output, offset, fieldTag(part.field, WIRE_TYPE_LENGTH_DELIMITED));
      offset = writeVarint(output, offset, payload.length);
      output.set(payload, offset);
      offset += payload.length;
    }
  }
  if (offset !== totalLength) {
    output.fill(0);
    throw new ReallyMeCodecError("provider-failure");
  }
  return output;
};
