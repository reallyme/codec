// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::error::{error, CborError, ErrorReason};

/// Schema-generated policies used by the native compatibility facade.
pub use codec_proto::generated::proto::reallyme::codec::v1::{
    CodecGeneralCborFloatPolicy as FloatPolicy, CodecGeneralCborLengthPolicy as LengthPolicy,
    CodecGeneralCborMapKeyEquality as MapKeyEquality, CodecGeneralCborMapOrdering as MapOrdering,
    CodecGeneralCborNanPolicy as NanPolicy, CodecGeneralCborSerialization as Serialization,
};

/// Basic validity and selected serialization guarantees.
///
/// Validation checks UTF-8 and rejects equivalent map keys using the selected
/// data model. Registered tag semantics remain opaque; tag 24
/// has a separate explicit embedded-item validation operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Profile {
    pub(super) serialization: Serialization,
    pub(super) lengths: LengthPolicy,
    pub(super) floats: FloatPolicy,
    pub(super) ordering: MapOrdering,
    map_key_equality: Option<MapKeyEquality>,
    nan_policy: Option<NanPolicy>,
}

impl Profile {
    pub(super) fn ensure_supported(self) -> Result<(), CborError> {
        Self::new(self.serialization, self.lengths, self.floats, self.ordering).map(|_| ())
    }

    /// Construct an explicit profile; preferred serialization requires definite lengths.
    pub fn new(
        serialization: Serialization,
        lengths: LengthPolicy,
        floats: FloatPolicy,
        ordering: MapOrdering,
    ) -> Result<Self, CborError> {
        ensure_lengths_supported(lengths)?;
        if !matches!(
            serialization,
            Serialization::Received | Serialization::Preferred
        ) || !matches!(
            floats,
            FloatPolicy::All | FloatPolicy::FiniteOnly | FloatPolicy::Reject
        ) || !matches!(
            ordering,
            MapOrdering::Received | MapOrdering::LengthFirst | MapOrdering::Bytewise
        ) {
            return Err(error(ErrorReason::UnsupportedPolicy));
        }
        if serialization == Serialization::Preferred && lengths != LengthPolicy::DefiniteOnly {
            return Err(error(ErrorReason::UnsupportedPolicy));
        }
        Ok(Self {
            serialization,
            lengths,
            floats,
            ordering,
            map_key_equality: None,
            nan_policy: None,
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
            map_key_equality: None,
            nan_policy: None,
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
            map_key_equality: None,
            nan_policy: None,
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

    /// Select scalar wire equality and structural compound key equality.
    pub fn with_map_key_equality(mut self, equality: MapKeyEquality) -> Result<Self, CborError> {
        match equality {
            MapKeyEquality::EncodedScalars => {
                self.map_key_equality = Some(equality);
                Ok(self)
            }
            _ => Err(error(ErrorReason::UnsupportedPolicy)),
        }
    }

    /// Select a NaN width restriction independently of general float acceptance.
    pub fn with_nan_policy(mut self, policy: NanPolicy) -> Result<Self, CborError> {
        match policy {
            NanPolicy::Binary16Only => {
                self.nan_policy = Some(policy);
                Ok(self)
            }
            _ => Err(error(ErrorReason::UnsupportedPolicy)),
        }
    }

    /// Inspect the optional map key equality rule.
    #[must_use]
    pub const fn map_key_equality(self) -> Option<MapKeyEquality> {
        self.map_key_equality
    }

    /// Inspect the optional NaN width restriction.
    #[must_use]
    pub const fn nan_policy(self) -> Option<NanPolicy> {
        self.nan_policy
    }

    pub(super) const fn has_encoded_scalar_keys(self) -> bool {
        self.map_key_equality.is_some()
    }

    pub(super) const fn requires_binary16_nan(self) -> bool {
        self.nan_policy.is_some()
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

pub(super) fn ensure_lengths_supported(value: LengthPolicy) -> Result<(), CborError> {
    match value {
        LengthPolicy::DefiniteOnly | LengthPolicy::AllowIndefinite => Ok(()),
        _ => Err(error(ErrorReason::UnsupportedPolicy)),
    }
}
