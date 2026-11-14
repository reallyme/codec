// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Generated Buffa code boundary.

/// Canonical protobuf package for ReallyMe codec identifiers.
pub const CODEC_PROTO_PACKAGE: &str = "reallyme.codec.v1";

/// Generated Buffa protobuf message and view types.
#[allow(missing_docs)]
#[allow(
    clippy::enum_variant_names,
    reason = "native generated oneof variants preserve protobuf field names"
)]
#[cfg(feature = "generated")]
#[path = "generated/buffa/mod.rs"]
mod raw_proto;

/// Schema-derived exports with restricted sensitive owned-view handles.
///
/// Buffa omits standard owned-view wrappers for sensitive messages and their
/// containers. Borrowed views can still convert to native wipe owners.
///
/// ```compile_fail
/// use reallyme_codec_proto::generated::proto::reallyme::codec::v1::CodecPemEncodeRequestOwnedView;
/// ```
///
/// ```compile_fail
/// use buffa::HasMessageView;
/// use reallyme_codec_proto::generated::proto::reallyme::codec::v1::CodecPemEncodeRequest;
/// fn requires_standard_owned_view<T: HasMessageView>() {}
/// requires_standard_owned_view::<CodecPemEncodeRequest>();
/// ```
///
/// ```compile_fail
/// use buffa::HasMessageView;
/// use reallyme_codec_proto::generated::proto::reallyme::codec::v1::CodecApiOperationRequest;
/// fn requires_standard_owned_view<T: HasMessageView>() {}
/// requires_standard_owned_view::<CodecApiOperationRequest>();
/// ```
///
/// ```compile_fail
/// use reallyme_codec_proto::generated::raw_proto;
/// ```
#[allow(missing_docs)]
#[cfg(feature = "generated")]
#[path = "generated/contract_exports.rs"]
pub mod proto;

#[cfg(feature = "generated")]
#[path = "generated/contract_error_traits.rs"]
mod contract_error_traits;

/// Schema-derived constants for allocation-safe wire preflight.
#[cfg(feature = "generated")]
#[path = "generated/wire_policy.rs"]
pub mod wire_policy;
