// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Bounded, schema-directed Distinguished Encoding Rules (DER) decoding.
//!
//! The caller selects the ASN.1 schema through typed reader methods. Each field
//! must have a canonical tag, definite minimal length, and canonical contents.
//! Document and container boundaries require exact consumption. Supported types
//! are BOOLEAN, INTEGER, BIT STRING, OCTET STRING, NULL, OBJECT IDENTIFIER,
//! UTF8String, SEQUENCE, and SET OF. Other types and implicit tagging are not
//! exposed. This is not a schema-free validator for arbitrary ASN.1 documents.
//! The schema callback must enforce field presence, ASN.1 constraints, omission
//! of DEFAULT-valued components, and trailing-zero removal for named BIT STRING
//! values. Those rules depend on the schema and cannot be inferred from a TLV.
//!
//! Parsing borrows the input and does not allocate or copy its payload. Returned
//! views have no Debug, serialization, or cloning implementation. The caller
//! retains ownership and must use a zeroizing owner for secret-bearing input;
//! borrowed views cannot erase someone else's immutable storage. Do not log
//! returned slices or strings. Curve widths, scalar ranges, and signature
//! normalization belong to the consuming cryptographic layer.
//!
//! ```
//! use reallyme_codec::der::{decode_unsigned_integer_pair, DerError, DerLimits};
//!
//! fn parse_pair() -> Result<(), DerError> {
//!     let encoded = [0x30, 0x06, 0x02, 0x01, 0x01, 0x02, 0x01, 0x02];
//!     let limits = DerLimits::new(256, 1, 3, 66)?;
//!     let pair = decode_unsigned_integer_pair(&encoded, limits)?;
//!     assert_eq!(pair.first(), &[1]);
//!     assert_eq!(pair.second(), &[2]);
//!     Ok(())
//! }
//! # parse_pair()?;
//! # Ok::<(), DerError>(())
//! ```
//!
//! Borrowed views cannot outlive their input owner:
//!
//! ```compile_fail
//! use reallyme_codec::der::{decode_unsigned_integer_pair, DerLimits};
//! let pair = {
//!     let bytes = vec![0x30, 6, 0x02, 1, 1, 0x02, 1, 2];
//!     decode_unsigned_integer_pair(&bytes, DerLimits::default())?
//! };
//! let _ = pair.first();
//! # Ok::<(), reallyme_codec::der::DerError>(())
//! ```

mod decode;
mod error;
mod limits;
mod read;
mod types;
mod validate;

pub use decode::{decode_der, decode_unsigned_integer_pair};
pub use error::{DerError, DerErrorReason};
pub use limits::{
    DerLimits, MAX_DER_ELEMENTS, MAX_DER_INPUT_LEN, MAX_DER_INTEGER_LEN, MAX_DER_NESTING_DEPTH,
};
pub use read::DerReader;
pub use types::{DerBitStringRef, DerIntegerRef, DerOidRef, DerUnsignedIntegerPair};
