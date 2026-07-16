// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use crate::PemLabel;

/// Default maximum PEM input size accepted by the parser.
pub const DEFAULT_MAX_PEM_INPUT_LEN: usize = 1024 * 1024;

/// Default maximum decoded DER size accepted by the parser.
pub const DEFAULT_MAX_DER_LEN: usize = 1024 * 1024;

/// Default PEM body line width used by the encoder.
pub const DEFAULT_PEM_LINE_WIDTH: usize = 64;

/// Labels accepted by the default decode policy.
pub const DEFAULT_ALLOWED_LABELS: &[PemLabel] = &[
    PemLabel::PrivateKey,
    PemLabel::EcPrivateKey,
    PemLabel::PublicKey,
];

/// Policy controlling PEM decoding.
#[derive(Debug, Clone, Copy)]
pub struct PemDecodePolicy<'a> {
    /// Exact labels accepted by the caller.
    pub allowed_labels: &'a [PemLabel],
    /// Maximum input text length in bytes.
    pub max_input_len: usize,
    /// Maximum decoded DER body length in bytes.
    pub max_der_len: usize,
}

impl Default for PemDecodePolicy<'_> {
    fn default() -> Self {
        Self {
            allowed_labels: DEFAULT_ALLOWED_LABELS,
            max_input_len: DEFAULT_MAX_PEM_INPUT_LEN,
            max_der_len: DEFAULT_MAX_DER_LEN,
        }
    }
}

/// Line ending emitted by PEM encoding.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PemLineEnding {
    /// Unix line endings.
    Lf,
    /// Network/Windows line endings.
    Crlf,
}

impl PemLineEnding {
    pub(crate) const fn as_str(self) -> &'static str {
        match self {
            Self::Lf => "\n",
            Self::Crlf => "\r\n",
        }
    }
}

/// Options controlling PEM encoding.
#[derive(Debug, Clone, Copy)]
pub struct PemEncodeOptions {
    /// Maximum DER body length accepted for encoding.
    pub max_der_len: usize,
    /// Base64 body line width.
    pub line_width: usize,
    /// Line ending to emit.
    pub line_ending: PemLineEnding,
}

impl Default for PemEncodeOptions {
    fn default() -> Self {
        Self {
            max_der_len: DEFAULT_MAX_DER_LEN,
            line_width: DEFAULT_PEM_LINE_WIDTH,
            line_ending: PemLineEnding::Lf,
        }
    }
}
