// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import { ScalarType } from "@bufbuild/protobuf";
import type { DescField, DescMessage, MessageShape } from "@bufbuild/protobuf";
import { isReflectList, isReflectMessage, reflect } from "@bufbuild/protobuf/reflect";
import type { ReflectList, ReflectMessage } from "@bufbuild/protobuf/reflect";
import { WireType } from "@bufbuild/protobuf/wire";

import { MAX_CODEC_PROTO_MESSAGE_BYTES, utf8ByteLength, writeUtf8 } from "./boundary.js";
import { ReallyMeCodecError } from "./errors.js";

const VARINT_CONTINUATION = 0x80n;
const VARINT_CONTINUATION_BIT = 0x80;
const VARINT_PAYLOAD_MASK = 0x7fn;
const VARINT_SHIFT = 7n;
const WIRE_WORD_BITS = 64;
const INT32_MIN = -2_147_483_648;
const INT32_MAX = 2_147_483_647;
const UINT32_MAX = 4_294_967_295;
const INT64_MIN = -0x8000_0000_0000_0000n;
const INT64_MAX = 0x7fff_ffff_ffff_ffffn;
const UINT64_MAX = 0xffff_ffff_ffff_ffffn;
const FIXED32_BYTES = 4;
const FIXED64_BYTES = 8;
const BOOL_BYTES = 1;

const providerFailure = (): never => {
  throw new ReallyMeCodecError("provider-failure");
};

const addLength = (total: number, increment: number): number => {
  const next = total + increment;
  if (!Number.isSafeInteger(next) || next > MAX_CODEC_PROTO_MESSAGE_BYTES) {
    throw new ReallyMeCodecError("invalid-input");
  }
  return next;
};

const varintLength = (value: bigint): number => {
  let length = 1;
  let rest = value;
  while (rest >= VARINT_CONTINUATION) {
    rest >>= VARINT_SHIFT;
    length += 1;
  }
  return length;
};

const tagValue = (fieldNumber: number, wireType: WireType): bigint =>
  BigInt(fieldNumber) * 8n + BigInt(wireType);

const requireInteger = (value: unknown, minimum: number, maximum: number): number => {
  if (typeof value !== "number" || !Number.isInteger(value) || value < minimum || value > maximum) {
    return providerFailure();
  }
  return value;
};

const requireBigint = (value: unknown, minimum: bigint, maximum: bigint): bigint => {
  if (typeof value !== "bigint" || value < minimum || value > maximum) {
    return providerFailure();
  }
  return value;
};

const requireFloat = (value: unknown): number => {
  if (typeof value !== "number") {
    return providerFailure();
  }
  return value;
};

/** Wire type used for one scalar or enum value, matching protobuf-es. */
const scalarWireType = (scalar: ScalarType): WireType => {
  switch (scalar) {
    case ScalarType.STRING:
    case ScalarType.BYTES:
      return WireType.LengthDelimited;
    case ScalarType.DOUBLE:
    case ScalarType.FIXED64:
    case ScalarType.SFIXED64:
      return WireType.Bit64;
    case ScalarType.FLOAT:
    case ScalarType.FIXED32:
    case ScalarType.SFIXED32:
      return WireType.Bit32;
    default:
      return WireType.Varint;
  }
};

/** Unsigned varint payload for a varint-typed scalar, matching protobuf-es. */
const varintPayload = (scalar: ScalarType, value: unknown): bigint => {
  switch (scalar) {
    case ScalarType.BOOL:
      if (typeof value !== "boolean") {
        return providerFailure();
      }
      return value ? 1n : 0n;
    case ScalarType.INT32:
      // Negative int32 values are sign-extended to ten bytes.
      return BigInt.asUintN(WIRE_WORD_BITS, BigInt(requireInteger(value, INT32_MIN, INT32_MAX)));
    case ScalarType.UINT32:
      return BigInt(requireInteger(value, 0, UINT32_MAX));
    case ScalarType.SINT32: {
      const signed = BigInt(requireInteger(value, INT32_MIN, INT32_MAX));
      return signed >= 0n ? signed << 1n : ((-signed) << 1n) - 1n;
    }
    case ScalarType.INT64:
      return BigInt.asUintN(WIRE_WORD_BITS, requireBigint(value, INT64_MIN, INT64_MAX));
    case ScalarType.UINT64:
      return requireBigint(value, 0n, UINT64_MAX);
    case ScalarType.SINT64: {
      const signed = requireBigint(value, INT64_MIN, INT64_MAX);
      return signed >= 0n ? signed << 1n : ((-signed) << 1n) - 1n;
    }
    default:
      return providerFailure();
  }
};

/** Encoded size of one scalar value without its tag. */
const scalarLength = (scalar: ScalarType, value: unknown): number => {
  switch (scalar) {
    case ScalarType.STRING: {
      if (typeof value !== "string") {
        return providerFailure();
      }
      const length = utf8ByteLength(value);
      return addLength(varintLength(BigInt(length)), length);
    }
    case ScalarType.BYTES:
      if (!(value instanceof Uint8Array)) {
        return providerFailure();
      }
      return addLength(varintLength(BigInt(value.length)), value.length);
    case ScalarType.DOUBLE:
    case ScalarType.FIXED64:
    case ScalarType.SFIXED64:
      return FIXED64_BYTES;
    case ScalarType.FLOAT:
    case ScalarType.FIXED32:
    case ScalarType.SFIXED32:
      return FIXED32_BYTES;
    case ScalarType.BOOL:
      return BOOL_BYTES;
    default:
      return varintLength(varintPayload(scalar, value));
  }
};

const fieldScalar = (field: DescField): ScalarType => {
  if (field.fieldKind === "scalar") {
    return field.scalar;
  }
  if (field.fieldKind === "enum") {
    return ScalarType.INT32;
  }
  if (field.fieldKind === "list" && field.listKind === "scalar") {
    return field.scalar;
  }
  if (field.fieldKind === "list" && field.listKind === "enum") {
    return ScalarType.INT32;
  }
  return providerFailure();
};

const childList = (value: unknown): ReflectList => {
  if (!isReflectList(value)) {
    return providerFailure();
  }
  return value;
};

const childMessage = (value: unknown): ReflectMessage => {
  if (!isReflectMessage(value)) {
    return providerFailure();
  }
  return value;
};

/**
 * Descriptor-driven sizing pass. Records the encoded body size of every
 * nested message so the write pass emits length prefixes without serializing
 * any subtree twice.
 */
class SizePlan {
  readonly sizes = new Map<object, number>();

  message(message: ReflectMessage): number {
    let size = 0;
    for (const field of message.sortedFields) {
      if (message.isSet(field)) {
        size = addLength(size, this.field(field, message.get(field)));
      }
    }
    for (const unknown of message.getUnknown() ?? []) {
      size = addLength(
        size,
        addLength(varintLength(tagValue(unknown.no, unknown.wireType)), unknown.data.length),
      );
    }
    this.sizes.set(message.message, size);
    return size;
  }

  private delimited(fieldNumber: number, payload: number): number {
    return addLength(
      addLength(varintLength(tagValue(fieldNumber, WireType.LengthDelimited)), varintLength(BigInt(payload))),
      payload,
    );
  }

  private field(field: DescField, value: unknown): number {
    switch (field.fieldKind) {
      case "scalar":
      case "enum": {
        const scalar = fieldScalar(field);
        return addLength(
          varintLength(tagValue(field.number, scalarWireType(scalar))),
          scalarLength(scalar, value),
        );
      }
      case "message":
        if (field.delimitedEncoding) {
          return providerFailure();
        }
        return this.delimited(field.number, this.message(childMessage(value)));
      case "list": {
        if (field.listKind === "message" && field.delimitedEncoding) {
          return providerFailure();
        }
        const list = childList(value);
        let size = 0;
        if (field.listKind === "message") {
          for (const item of list) {
            size = addLength(size, this.delimited(field.number, this.message(childMessage(item))));
          }
          return size;
        }
        const scalar = fieldScalar(field);
        if (field.packed) {
          let payload = 0;
          for (const item of list) {
            payload = addLength(payload, scalarLength(scalar, item));
          }
          return this.delimited(field.number, payload);
        }
        const tagLength = varintLength(tagValue(field.number, scalarWireType(scalar)));
        for (const item of list) {
          size = addLength(size, addLength(tagLength, scalarLength(scalar, item)));
        }
        return size;
      }
      case "map":
        // The codec schema deliberately has no map fields.
        return providerFailure();
    }
  }
}

/** Descriptor-driven write pass into one exactly sized buffer. */
class ExactWriter {
  offset = 0;
  private readonly view: DataView;

  constructor(
    readonly output: Uint8Array,
    private readonly sizes: ReadonlyMap<object, number>,
  ) {
    this.view = new DataView(output.buffer, output.byteOffset, output.byteLength);
  }

  private planned(message: ReflectMessage): number {
    return this.sizes.get(message.message) ?? providerFailure();
  }

  private varint(value: bigint): void {
    let rest = value;
    while (rest >= VARINT_CONTINUATION) {
      this.output[this.offset] = Number(rest & VARINT_PAYLOAD_MASK) | VARINT_CONTINUATION_BIT;
      rest >>= VARINT_SHIFT;
      this.offset += 1;
    }
    this.output[this.offset] = Number(rest);
    this.offset += 1;
  }

  private tag(fieldNumber: number, wireType: WireType): void {
    this.varint(tagValue(fieldNumber, wireType));
  }

  private scalar(scalar: ScalarType, value: unknown): void {
    switch (scalar) {
      case ScalarType.STRING: {
        if (typeof value !== "string") {
          providerFailure();
          return;
        }
        const length = utf8ByteLength(value);
        this.varint(BigInt(length));
        const end = writeUtf8(this.output, this.offset, value);
        if (end !== this.offset + length) {
          providerFailure();
        }
        this.offset = end;
        return;
      }
      case ScalarType.BYTES:
        if (!(value instanceof Uint8Array)) {
          providerFailure();
          return;
        }
        this.varint(BigInt(value.length));
        this.output.set(value, this.offset);
        this.offset += value.length;
        return;
      case ScalarType.DOUBLE:
        this.view.setFloat64(this.offset, requireFloat(value), true);
        this.offset += FIXED64_BYTES;
        return;
      case ScalarType.FLOAT:
        this.view.setFloat32(this.offset, requireFloat(value), true);
        this.offset += FIXED32_BYTES;
        return;
      case ScalarType.FIXED64:
        this.view.setBigUint64(this.offset, requireBigint(value, 0n, UINT64_MAX), true);
        this.offset += FIXED64_BYTES;
        return;
      case ScalarType.SFIXED64:
        this.view.setBigInt64(this.offset, requireBigint(value, INT64_MIN, INT64_MAX), true);
        this.offset += FIXED64_BYTES;
        return;
      case ScalarType.FIXED32:
        this.view.setUint32(this.offset, requireInteger(value, 0, UINT32_MAX), true);
        this.offset += FIXED32_BYTES;
        return;
      case ScalarType.SFIXED32:
        this.view.setInt32(this.offset, requireInteger(value, INT32_MIN, INT32_MAX), true);
        this.offset += FIXED32_BYTES;
        return;
      default:
        this.varint(varintPayload(scalar, value));
    }
  }

  message(message: ReflectMessage): void {
    for (const field of message.sortedFields) {
      if (message.isSet(field)) {
        this.field(field, message.get(field));
      }
    }
    for (const unknown of message.getUnknown() ?? []) {
      this.tag(unknown.no, unknown.wireType);
      this.output.set(unknown.data, this.offset);
      this.offset += unknown.data.length;
    }
  }

  private field(field: DescField, value: unknown): void {
    switch (field.fieldKind) {
      case "scalar":
      case "enum": {
        const scalar = fieldScalar(field);
        this.tag(field.number, scalarWireType(scalar));
        this.scalar(scalar, value);
        return;
      }
      case "message": {
        const child = childMessage(value);
        this.tag(field.number, WireType.LengthDelimited);
        this.varint(BigInt(this.planned(child)));
        this.message(child);
        return;
      }
      case "list": {
        const list = childList(value);
        if (field.listKind === "message") {
          for (const item of list) {
            const child = childMessage(item);
            this.tag(field.number, WireType.LengthDelimited);
            this.varint(BigInt(this.planned(child)));
            this.message(child);
          }
          return;
        }
        const scalar = fieldScalar(field);
        if (field.packed) {
          let payload = 0;
          for (const item of list) {
            payload += scalarLength(scalar, item);
          }
          this.tag(field.number, WireType.LengthDelimited);
          this.varint(BigInt(payload));
          for (const item of list) {
            this.scalar(scalar, item);
          }
          return;
        }
        for (const item of list) {
          this.tag(field.number, scalarWireType(scalar));
          this.scalar(scalar, item);
        }
        return;
      }
      case "map":
        providerFailure();
    }
  }
}

/**
 * Serialize a generated message into one exactly sized buffer, byte for byte
 * as protobuf-es `toBinary` would.
 *
 * The encoding is driven entirely by the generated descriptor, so the schema
 * stays the single source of truth for every field. Unlike the protobuf-es
 * writer, which grows and abandons internal buffers that keep copies of every
 * text and byte field, this produces exactly one serialized copy: the returned
 * buffer, which the caller owns and must wipe. Map and group-encoded fields,
 * which the codec schema does not use, fail closed.
 */
export const serializeExactBinary = <Desc extends DescMessage>(
  schema: Desc,
  message: MessageShape<Desc>,
): Uint8Array => {
  const root = reflect(schema, message, false);
  const plan = new SizePlan();
  const output = new Uint8Array(plan.message(root));
  try {
    const writer = new ExactWriter(output, plan.sizes);
    writer.message(root);
    if (writer.offset !== output.length) {
      providerFailure();
    }
  } catch (error: unknown) {
    output.fill(0);
    throw error;
  } finally {
    plan.sizes.clear();
  }
  return output;
};
