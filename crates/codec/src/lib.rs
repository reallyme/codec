// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! # reallyme-codec
//!
//! Codec-only utilities used by ReallyMe and DID tooling: base encodings,
//! canonical JSON/CBOR serialization, multicodec lookup, multikey handling, and
//! PEM text armor.
//! SHA-256 is used for content identifiers. This crate does not expose signing,
//! encryption, or key generation.
//!
//! # Usage examples
//!
#![cfg_attr(feature = "base64url", doc = include_str!("../README.md"))]
#![forbid(unsafe_code)]

/// Standard (RFC 4648) base64 encode/decode.
#[cfg(feature = "base64")]
pub mod base64;

/// URL-safe (RFC 4648 §5) base64 encode/decode without padding.
#[cfg(feature = "base64url")]
pub mod base64url;

/// Borrowed general CBOR, deterministic CBOR, DAG-CBOR, and CID helpers.
#[cfg(feature = "cbor")]
pub mod cbor;

/// Strict, bounded, schema-directed DER parsing for Rust consumers.
#[cfg(feature = "der")]
pub mod der;

/// Bounded definite BER/ISO 7816 and DER header framing.
#[cfg(feature = "tlv")]
pub mod tlv;

/// Canonical lowercase hexadecimal encode/decode helpers.
#[cfg(feature = "hex")]
pub mod hex;

/// JSON Canonicalization Scheme helpers.
#[cfg(feature = "jcs")]
pub mod jcs;

/// Multibase self-describing base encodings.
#[cfg(feature = "multibase")]
pub mod multibase;

/// Semantic multicodec registry lookup, prefix handling, and metadata.
#[cfg(feature = "multicodec")]
pub mod multicodec;

/// Multikey encoding/parsing that binds an algorithm to opaque key bytes.
#[cfg(feature = "multikey")]
pub mod multikey;

/// PEM text armor parsing and encoding.
#[cfg(feature = "pem")]
pub mod pem;

/// Generated protobuf operation contract lane.
#[cfg(feature = "operation-contract")]
pub mod operation_contract;
