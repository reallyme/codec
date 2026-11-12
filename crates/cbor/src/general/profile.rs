// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::error::{error, CborError, ErrorReason};

/// Received indefinite-length policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum LengthPolicy {
    /// Reject all indefinite-length items.
    DefiniteOnly,
    /// Accept well-formed indefinite containers and chunked strings.
    AllowIndefinite,
}

/// Serialization checks are independent of map ordering.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum Serialization {
    /// Preserve received widths without requiring shortest encodings.
    Received,
    /// Require shortest arguments and shortest exact floating-point encodings.
    Preferred,
}

/// Explicit floating-point acceptance policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FloatPolicy {
    /// Accept all binary16/32/64 values, including infinities and NaNs.
    All,
    /// Accept finite values only, including both signs of zero.
    FiniteOnly,
    /// Reject all floating-point values.
    Reject,
}

/// Optional ordering constraint on complete received key encodings.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum MapOrdering {
    /// Preserve any received order; semantic key uniqueness is still checked.
    Received,
    /// Compare encoded length first, then encoded bytes.
    LengthFirst,
    /// Compare complete key encodings lexicographically.
    Bytewise,
}

/// Basic validity and selected serialization guarantees.
///
/// Validation checks UTF-8 and rejects equivalent map keys using RFC 8949's
/// basic generic data model. Registered tag semantics remain opaque; tag 24
/// has a separate explicit embedded-item validation operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Profile {
    pub(super) serialization: Serialization,
    pub(super) lengths: LengthPolicy,
    pub(super) floats: FloatPolicy,
    pub(super) ordering: MapOrdering,
}

impl Profile {
    /// Construct an explicit profile; preferred serialization requires definite lengths.
    pub fn new(
        serialization: Serialization,
        lengths: LengthPolicy,
        floats: FloatPolicy,
        ordering: MapOrdering,
    ) -> Result<Self, CborError> {
        if serialization == Serialization::Preferred && lengths != LengthPolicy::DefiniteOnly {
            return Err(error(ErrorReason::UnsupportedPolicy));
        }
        Ok(Self {
            serialization,
            lengths,
            floats,
            ordering,
        })
    }

    /// Basic validity with received widths, indefinite support, and unsorted unique maps.
    #[must_use]
    pub const fn received() -> Self {
        Self {
            serialization: Serialization::Received,
            lengths: LengthPolicy::AllowIndefinite,
            floats: FloatPolicy::All,
            ordering: MapOrdering::Received,
        }
    }

    /// Preferred widths and definite lengths, with unsorted unique maps.
    #[must_use]
    pub const fn preferred() -> Self {
        Self {
            serialization: Serialization::Preferred,
            lengths: LengthPolicy::DefiniteOnly,
            floats: FloatPolicy::All,
            ordering: MapOrdering::Received,
        }
    }

    /// Select floating-point acceptance without changing other guarantees.
    #[must_use]
    pub const fn with_floats(mut self, policy: FloatPolicy) -> Self {
        self.floats = policy;
        self
    }

    /// Select map ordering; encoding never sorts caller data automatically.
    #[must_use]
    pub const fn with_map_ordering(mut self, ordering: MapOrdering) -> Self {
        self.ordering = ordering;
        self
    }

    /// Inspect the received length policy.
    #[must_use]
    pub const fn lengths(self) -> LengthPolicy {
        self.lengths
    }

    /// Inspect serialization guarantees.
    #[must_use]
    pub const fn serialization(self) -> Serialization {
        self.serialization
    }

    /// Inspect floating-point acceptance.
    #[must_use]
    pub const fn floats(self) -> FloatPolicy {
        self.floats
    }

    /// Inspect the independently selected map-order constraint.
    #[must_use]
    pub const fn map_ordering(self) -> MapOrdering {
        self.ordering
    }
}
