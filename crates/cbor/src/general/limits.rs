// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::error::{error, CborError, ErrorReason};

/// Hard cap for one received input or protected output: one MiB.
pub const MAX_BYTES: usize = 1_048_576;
/// Hard cap for simultaneously open containers and tags.
pub const MAX_DEPTH: usize = 64;
/// Hard cap for structural nodes, including tags and string chunks.
pub const MAX_NODES: usize = 65_536;
/// Hard cap for entries in one array/map or chunks in one string.
pub const MAX_ENTRIES: usize = 4_096;
/// Hard cap for validation work credits.
pub const MAX_WORK: usize = 67_108_864;
const DEFAULT_WORK: usize = 16_777_216;

/// Caller-lowerable limits; private fields prevent bypassing hard ceilings.
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
        "Set or inspect the maximum entries or chunks in one container."
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
        MAX_BYTES,
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
            input: MAX_BYTES,
            output: MAX_BYTES,
            depth: MAX_DEPTH,
            nodes: MAX_NODES,
            entries: MAX_ENTRIES,
            bytes: MAX_BYTES,
            text: MAX_BYTES,
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
}

impl Budget {
    pub(super) const fn new(limits: Limits) -> Self {
        Self {
            limits,
            work: 0,
            nodes: 0,
            bytes: 0,
            text: 0,
        }
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
        if next > self.limits.depth {
            return Err(error(ErrorReason::DepthLimit));
        }
        Ok(next)
    }
}
