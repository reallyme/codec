// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::error::{error, CborError, ErrorReason};
use super::profile::Profile;

/// Default received input, protected output, and aggregate byte-string budget: one MiB.
pub const DEFAULT_BYTES: usize = 1_048_576;
/// Explicit ceiling for received input, protected output, and aggregate byte strings: two MiB.
pub const MAX_BYTES: usize = 2_097_152;
/// Default and ceiling for aggregate UTF-8 text; raising binary budgets does not raise this.
pub const MAX_TEXT_BYTES: usize = 1_048_576;
/// Hard cap for simultaneously open containers and tags.
pub const MAX_DEPTH: usize = 64;
/// Hard cap for structural nodes, including tags and string chunks.
pub const MAX_NODES: usize = 65_536;
/// Hard cap for entries in one array/map or chunks in one string.
pub const MAX_ENTRIES: usize = 65_536;
/// Default entries in one array/map or chunks in one string.
pub const DEFAULT_ENTRIES: usize = 4_096;
/// Hard cap for validation work credits.
pub const MAX_WORK: usize = 67_108_864;
const DEFAULT_WORK: usize = 16_777_216;

const ENCODED_KEY_MAX_SCAN_NODES: usize = 1_048_576;
const ENCODED_KEY_MAX_COMPARISON_BYTES: usize = 1_048_576;

/// Caller-configurable limits; private fields prevent bypassing hard ceilings.
///
/// Container entries default to 4,096 and may be explicitly raised to 65,536.
/// Input, output, and aggregate byte strings default independently to one MiB
/// and may each be explicitly raised to two MiB. Encoded byte budgets include
/// headers; aggregate content budgets do not. Text remains capped at one MiB.
/// The independent node, byte, depth, and work budgets still apply to the whole
/// operation, so a container and its children must fit within the node budget.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Limits {
    pub(super) input: usize,
    pub(super) output: usize,
    pub(super) depth: usize,
    pub(super) nodes: usize,
    pub(super) entries: usize,
    pub(super) bytes: usize,
    pub(super) text: usize,
    pub(super) work: usize,
}

macro_rules! limit {
    ($setter:ident, $getter:ident, $field:ident, $ceiling:ident, $doc:literal) => {
        #[doc = $doc]
        pub fn $setter(mut self, value: usize) -> Result<Self, CborError> {
            if value > $ceiling {
                return Err(error(ErrorReason::InvalidLimit));
            }
            self.$field = value;
            Ok(self)
        }

        #[doc = $doc]
        #[must_use]
        pub const fn $getter(self) -> usize {
            self.$field
        }
    };
}

impl Limits {
    limit!(
        with_input_bytes,
        input_bytes,
        input,
        MAX_BYTES,
        "Set or inspect the maximum received input bytes."
    );
    limit!(
        with_output_bytes,
        output_bytes,
        output,
        MAX_BYTES,
        "Set or inspect the maximum encoded output bytes."
    );
    limit!(
        with_depth,
        depth,
        depth,
        MAX_DEPTH,
        "Set or inspect the maximum simultaneously open containers and tags."
    );
    limit!(
        with_nodes,
        nodes,
        nodes,
        MAX_NODES,
        "Set or inspect the maximum structural node count."
    );
    limit!(
        with_entries,
        entries,
        entries,
        MAX_ENTRIES,
        "Set or inspect entries or chunks per container (default 4,096; ceiling 65,536)."
    );
    limit!(
        with_byte_string_bytes,
        byte_string_bytes,
        bytes,
        MAX_BYTES,
        "Set or inspect aggregate byte-string content bytes."
    );
    limit!(
        with_text_bytes,
        text_bytes,
        text,
        MAX_TEXT_BYTES,
        "Set or inspect aggregate UTF-8 text bytes."
    );
    limit!(
        with_work,
        work,
        work,
        MAX_WORK,
        "Set or inspect validation work credits, including duplicate-key comparisons."
    );
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            input: DEFAULT_BYTES,
            output: DEFAULT_BYTES,
            depth: MAX_DEPTH,
            nodes: MAX_NODES,
            entries: DEFAULT_ENTRIES,
            bytes: DEFAULT_BYTES,
            text: MAX_TEXT_BYTES,
            work: DEFAULT_WORK,
        }
    }
}

pub(super) struct Budget {
    pub(super) limits: Limits,
    pub(super) work: usize,
    nodes: usize,
    bytes: usize,
    text: usize,
    encoded_keys: Option<EncodedKeyWork>,
}

struct EncodedKeyWork {
    scan_nodes_left: usize,
    comparison_bytes_left: usize,
}

impl Budget {
    pub(super) const fn new(limits: Limits) -> Self {
        Self {
            limits,
            work: 0,
            nodes: 0,
            bytes: 0,
            text: 0,
            encoded_keys: None,
        }
    }

    pub(super) fn for_profile(limits: Limits, profile: Profile) -> Self {
        let mut budget = Self::new(limits);
        if profile.has_encoded_scalar_keys() {
            budget.encoded_keys = Some(EncodedKeyWork {
                scan_nodes_left: ENCODED_KEY_MAX_SCAN_NODES,
                comparison_bytes_left: ENCODED_KEY_MAX_COMPARISON_BYTES,
            });
        }
        budget
    }

    pub(super) const fn has_encoded_scalar_keys(&self) -> bool {
        self.encoded_keys.is_some()
    }

    pub(super) fn encoded_key_scan_node(
        &mut self,
        depth: usize,
        account: bool,
    ) -> Result<(), CborError> {
        if let Some(encoded_keys) = &mut self.encoded_keys {
            if !account {
                encoded_keys.scan_nodes_left = encoded_keys
                    .scan_nodes_left
                    .checked_sub(1)
                    .ok_or(error(ErrorReason::WorkLimit))?;
            }
        }
        self.check_item_depth(depth)
    }

    pub(super) fn check_item_depth(&self, depth: usize) -> Result<(), CborError> {
        // The encoded-key preflight limits visited nodes. An empty container
        // at the last allowed depth has no deeper node to visit.
        if self.has_encoded_scalar_keys() && depth > self.limits.depth {
            return Err(error(ErrorReason::DepthLimit));
        }
        Ok(())
    }

    pub(super) fn encoded_key_compare_bytes(
        &mut self,
        left: usize,
        right: usize,
    ) -> Result<(), CborError> {
        let cost = left
            .checked_add(right)
            .ok_or(error(ErrorReason::ArithmeticOverflow))?;
        self.charge(cost)?;
        let encoded_keys = self
            .encoded_keys
            .as_mut()
            .ok_or(error(ErrorReason::UnsupportedPolicy))?;
        encoded_keys.comparison_bytes_left = encoded_keys
            .comparison_bytes_left
            .checked_sub(cost)
            .ok_or(error(ErrorReason::WorkLimit))?;
        Ok(())
    }

    pub(super) fn charge(&mut self, amount: usize) -> Result<(), CborError> {
        self.work = self
            .work
            .checked_add(amount)
            .ok_or(error(ErrorReason::ArithmeticOverflow))?;
        if self.work > self.limits.work {
            return Err(error(ErrorReason::WorkLimit));
        }
        Ok(())
    }

    pub(super) fn node(&mut self) -> Result<(), CborError> {
        self.nodes = self
            .nodes
            .checked_add(1)
            .ok_or(error(ErrorReason::ArithmeticOverflow))?;
        if self.nodes > self.limits.nodes {
            return Err(error(ErrorReason::NodeLimit));
        }
        Ok(())
    }

    pub(super) fn declared_nodes(&self, count: usize) -> Result<(), CborError> {
        let total = self
            .nodes
            .checked_add(count)
            .ok_or(error(ErrorReason::ArithmeticOverflow))?;
        if total > self.limits.nodes {
            return Err(error(ErrorReason::NodeLimit));
        }
        let work = self
            .work
            .checked_add(count)
            .ok_or(error(ErrorReason::ArithmeticOverflow))?;
        if work > self.limits.work {
            return Err(error(ErrorReason::WorkLimit));
        }
        Ok(())
    }

    pub(super) fn contents(&mut self, amount: usize, text: bool) -> Result<(), CborError> {
        let (count, limit, reason) = if text {
            (&mut self.text, self.limits.text, ErrorReason::TextLimit)
        } else {
            (
                &mut self.bytes,
                self.limits.bytes,
                ErrorReason::ByteStringLimit,
            )
        };
        *count = count
            .checked_add(amount)
            .ok_or(error(ErrorReason::ArithmeticOverflow))?;
        if *count > limit {
            return Err(error(reason));
        }
        Ok(())
    }

    pub(super) fn enter(&self, depth: usize) -> Result<usize, CborError> {
        let next = depth
            .checked_add(1)
            .ok_or(error(ErrorReason::ArithmeticOverflow))?;
        if !self.has_encoded_scalar_keys() && next > self.limits.depth {
            return Err(error(ErrorReason::DepthLimit));
        }
        Ok(next)
    }
}

#[cfg(kani)]
#[path = "budget_proof_tests.rs"]
mod proofs;
