// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{error::overflow, ErrorReason as R, JsonError, Limits};

const MIB: u64 = 1_048_576;
const HARD_INPUT: u64 = 16 * MIB;
const HARD_WORK: u64 = 1024 * MIB;
const DEFAULT_DEPTH: u64 = 32;
const HARD_DEPTH: u64 = 64;
const DEFAULT_ENTRIES: u64 = 256;
const HARD_ENTRIES: u64 = 65_536;
const DEFAULT_NODES: u64 = 65_536;
const DEFAULT_STRING: u64 = 65_536;
const DEFAULT_WORK: u64 = 16 * MIB;

pub(super) struct Budget {
    pub(super) depth: usize,
    pub(super) entries: usize,
    pub(super) nodes: usize,
    pub(super) string: usize,
    pub(super) total_string: usize,
    pub(super) work: usize,
    pub(super) input: usize,
    used_nodes: usize,
    used_string: usize,
    used_work: usize,
    pub(super) failure: Option<JsonError>,
}

fn bound(value: Option<u64>, default: u64, hard: u64) -> Result<usize, JsonError> {
    let value = value.unwrap_or(default);
    if value == 0 || value > hard {
        return Err(JsonError::new(
            R::CODEC_ERROR_REASON_CANONICAL_JSON_INVALID_LIMITS,
        ));
    }
    usize::try_from(value).map_err(|_| overflow())
}

impl Budget {
    pub(super) fn new(limits: Option<&Limits>) -> Result<Self, JsonError> {
        let defaults = Limits::default();
        let limits = limits.unwrap_or(&defaults);
        Ok(Self {
            input: bound(limits.input_bytes, MIB, HARD_INPUT)?,
            depth: bound(limits.nesting_depth, DEFAULT_DEPTH, HARD_DEPTH)?,
            entries: bound(limits.container_entries, DEFAULT_ENTRIES, HARD_ENTRIES)?,
            nodes: bound(limits.nodes, DEFAULT_NODES, MIB)?,
            string: bound(limits.string_bytes, DEFAULT_STRING, HARD_INPUT)?,
            total_string: bound(limits.total_string_bytes, MIB, HARD_INPUT)?,
            work: bound(limits.work_bytes, DEFAULT_WORK, HARD_WORK)?,
            used_nodes: 0,
            used_string: 0,
            used_work: 0,
            failure: None,
        })
    }

    pub(super) fn node(&mut self) -> Result<(), JsonError> {
        self.used_nodes = self.used_nodes.checked_add(1).ok_or_else(overflow)?;
        if self.used_nodes > self.nodes {
            return Err(JsonError::new(
                R::CODEC_ERROR_REASON_CANONICAL_JSON_NODE_LIMIT,
            ));
        }
        Ok(())
    }

    pub(super) fn scan(&mut self, bytes: usize) -> Result<(), JsonError> {
        self.used_work = self.used_work.checked_add(bytes).ok_or_else(overflow)?;
        if self.used_work > self.work {
            return Err(JsonError::new(
                R::CODEC_ERROR_REASON_CANONICAL_JSON_WORK_LIMIT,
            ));
        }
        Ok(())
    }

    pub(super) fn string_capacity(&self, encoded: usize) -> Result<usize, JsonError> {
        let remaining = self
            .total_string
            .checked_sub(self.used_string)
            .ok_or_else(overflow)?;
        Ok(encoded.min(self.string).min(remaining))
    }

    pub(super) fn text(&mut self, current: usize, additional: usize) -> Result<(), JsonError> {
        if current.checked_add(additional).ok_or_else(overflow)? > self.string {
            return Err(JsonError::new(
                R::CODEC_ERROR_REASON_CANONICAL_JSON_STRING_LIMIT,
            ));
        }
        let next = self
            .used_string
            .checked_add(additional)
            .ok_or_else(overflow)?;
        if next > self.total_string {
            return Err(JsonError::new(
                R::CODEC_ERROR_REASON_CANONICAL_JSON_TOTAL_STRING_LIMIT,
            ));
        }
        self.used_string = next;
        Ok(())
    }

    pub(super) fn reject<E: serde::de::Error>(&mut self, error: JsonError) -> E {
        self.failure = Some(error);
        E::custom(error)
    }
}
