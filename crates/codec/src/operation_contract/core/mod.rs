// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Semantic adapters used only by the generated operation contract.

mod dag_cbor;
mod multikey;
mod pem;

pub(super) use dag_cbor::{
    decode_dag_cbor_value, decode_deterministic_cbor_value, encode_dag_cbor_value,
    encode_deterministic_cbor_value, verify_dag_cbor_cid, DagCborCidVerification,
    DagCborOperationError,
};
pub(super) use multikey::{
    parse_multikey, MultikeyOperationError, ParsedMultikey as SemanticParsedMultikey,
};
pub(super) use pem::{decode_pem, encode_pem, DecodedPem, EncodedPem, PemOperationError};
