// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

// Generated from codec.proto. Do not edit.

/// All operation tags guarded before generated request allocation.
pub const OPERATION_FIELD_NUMBERS: &[u64] = &[
    1000, 1001, 1002, 1003, 2000, 2001, 2002, 2003, 2004, 3000, 3001, 3002, 3003, 3004, 3005, 3006,
    3007, 3008, 4000, 4001, 5000, 5001, 6000, 6001, 6002, 6003, 6004, 6005, 6006, 6007, 6008, 6009,
    6010, 7000, 7001, 7002, 8000, 8001, 8010, 8011, 8012, 8020, 8021, 8030, 8031, 8040, 8041, 8042,
    9000, 10000, 11000, 11001, 11002, 11003, 11004,
];
/// PEM decode operation tag.
pub const PEM_DECODE_OPERATION_FIELD: u64 = 4000;
/// PEM decode options tag.
pub const PEM_DECODE_OPTIONS_FIELD: u64 = 2;
/// PEM accepted-label list tag.
pub const PEM_ALLOWED_LABELS_FIELD: u64 = 1;
/// Number of nonzero PEM label values in the schema.
pub const MAX_PEM_ALLOWED_LABELS: usize = 3;
