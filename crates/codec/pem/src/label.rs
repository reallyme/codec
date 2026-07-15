// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use crate::PemError;

/// Supported PEM labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PemLabel {
    /// PKCS#8 private key material.
    PrivateKey,
    /// SEC1 elliptic-curve private key material.
    EcPrivateKey,
    /// SubjectPublicKeyInfo public key material.
    PublicKey,
}

impl PemLabel {
    /// Returns the exact RFC 7468 label text used in BEGIN/END boundaries.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PrivateKey => "PRIVATE KEY",
            Self::EcPrivateKey => "EC PRIVATE KEY",
            Self::PublicKey => "PUBLIC KEY",
        }
    }

    pub(crate) fn parse(input: &str) -> Result<Self, PemError> {
        match input {
            "PRIVATE KEY" => Ok(Self::PrivateKey),
            "EC PRIVATE KEY" => Ok(Self::EcPrivateKey),
            "PUBLIC KEY" => Ok(Self::PublicKey),
            _ => Err(PemError::UnsupportedLabel),
        }
    }
}
