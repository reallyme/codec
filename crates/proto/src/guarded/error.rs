// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#[derive(Debug, thiserror::Error)]
pub(super) enum StorageErrorReason {
    #[error("guarded storage length overflow")]
    LengthOverflow,
    #[error("guarded storage allocation failed")]
    AllocationFailure,
    #[error("invalid guarded base64 encoding")]
    InvalidEncoding,
}
