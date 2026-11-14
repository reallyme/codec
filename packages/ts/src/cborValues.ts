// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0



export type ReallyMeCborValue =
  | Readonly<{ type: "null" }>
  | Readonly<{ type: "bool"; value: boolean }>
  | Readonly<{ type: "int"; value: number | bigint }>
  | Readonly<{ type: "string"; value: string }>
  | Readonly<{ type: "bytes"; value: Uint8Array }>
  | Readonly<{ type: "array"; value: ReadonlyArray<ReallyMeCborValue> }>
  | Readonly<{ type: "map"; value: ReadonlyArray<ReallyMeCborMapEntry> }>;

export type ReallyMeCborMapEntry = Readonly<{
  key: string;
  value: ReallyMeCborValue;
}>;

export type ReallyMeDagCborCidVerification = Readonly<{
  valid: boolean;
  expectedCid: string;
  actualCid: string;
}>;

export type ReallyMeDeterministicCborInteger =
  | Readonly<{ type: "unsigned"; value: bigint }>
  | Readonly<{ type: "negative"; value: bigint }>;

export type ReallyMeDeterministicCborMapKey =
  | Readonly<{ type: "integer"; value: ReallyMeDeterministicCborInteger }>
  | Readonly<{ type: "text"; value: string }>;

export type ReallyMeDeterministicCborMapEntry = Readonly<{
  key: ReallyMeDeterministicCborMapKey;
  value: ReallyMeDeterministicCborValue;
}>;

export type ReallyMeDeterministicCborValue =
  | Readonly<{ type: "null" }>
  | Readonly<{ type: "bool"; value: boolean }>
  | Readonly<{ type: "integer"; value: ReallyMeDeterministicCborInteger }>
  | Readonly<{ type: "text"; value: string }>
  | Readonly<{ type: "bytes"; value: Uint8Array }>
  | Readonly<{
      type: "array";
      value: ReadonlyArray<ReallyMeDeterministicCborValue>;
    }>
  | Readonly<{
      type: "map";
      value: ReadonlyArray<ReallyMeDeterministicCborMapEntry>;
    }>;
