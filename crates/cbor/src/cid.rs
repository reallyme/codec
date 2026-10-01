// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use cid::multibase::{decode as multibase_decode, Base};
use cid::{Cid, Version};
use multihash::Multihash;
use multihash_codetable::{Code, MultihashDigest};
use sha2::{Digest, Sha256};
use zeroize::Zeroizing;

use crate::{decode_dag_cbor, CborError};

/// dag-cbor multicodec code (IPLD)
pub const DAG_CBOR_CODEC: u64 = 0x71;

/// Maximum CID string size accepted before multibase decoding.
///
/// A CID with the supported 64-byte digest and four u64 varints occupies at
/// most 104 binary bytes. This budget accommodates even base2 and the UTF-8
/// base256emoji representation, while bounding quadratic base conversions.
pub const MAX_CID_STRING_LEN: usize = 1024;

const CID_V0_STRING_LEN: usize = 46;

/// Hash output for sha2-256
pub type ContentHash = [u8; 32];

/// SHA-256 multihash for a DAG-CBOR block.
///
/// The wrapper keeps the upstream multihash representation out of the public
/// API while preserving the canonical wire bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DagCborMultihash(Multihash<64>);

impl DagCborMultihash {
    /// Return the multihash algorithm code.
    pub fn code(&self) -> u64 {
        self.0.code()
    }

    /// Return the digest length.
    pub fn size(&self) -> u8 {
        self.0.size()
    }

    /// Borrow the digest bytes.
    pub fn digest(&self) -> &[u8] {
        self.0.digest()
    }

    /// Return canonical multihash wire bytes.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }
}

/// Validated CID, independent of the upstream CID type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedCid(Cid);

impl ParsedCid {
    /// Return the validated CID's canonical binary representation.
    pub fn to_bytes(&self) -> Vec<u8> {
        self.0.to_bytes()
    }
}

impl core::fmt::Display for ParsedCid {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.0.fmt(formatter)
    }
}

/// Returns the raw sha2-256 digest of `bytes`.
pub fn sha2_256_content_hash(bytes: &[u8]) -> ContentHash {
    Sha256::digest(bytes).into()
}

/// Returns a sha2-256 multihash of `bytes` for use in a CID.
pub fn dag_cbor_multihash(bytes: &[u8]) -> DagCborMultihash {
    DagCborMultihash(Code::Sha2_256.digest(bytes))
}

/// Computes the CIDv1 (dag-cbor, sha2-256) of `bytes` in canonical
/// base32-lower string form.
///
/// Hashes the supplied bytes as-is, without parsing CBOR or applying the
/// encoder/decoder size limit. Encode a value first to obtain a canonical block.
pub fn compute_cid_dag_cbor(bytes: &[u8]) -> String {
    let hash = dag_cbor_multihash(bytes);
    let cid = Cid::new_v1(DAG_CBOR_CODEC, hash.0);
    cid.to_string()
}

/// Outcome of comparing a canonical DAG-CBOR block with a CID string.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CidVerificationStatus {
    /// The canonical CID matches the validated block.
    Match,
    /// A canonical CID identifies different bytes.
    Mismatch,
    /// The CID parses but its textual representation is not canonical.
    NonCanonical,
    /// The supplied text is not a valid CID.
    InvalidCid,
}

/// Validated CID comparison with canonical, input-independent diagnostics.
#[must_use]
pub struct DagCborCidVerification {
    status: CidVerificationStatus,
    expected_cid: String,
    actual_cid: String,
}

impl DagCborCidVerification {
    /// Return the exact verification outcome.
    pub const fn status(&self) -> CidVerificationStatus {
        self.status
    }

    /// Return the canonical CID computed from the block.
    pub fn expected_cid(&self) -> &str {
        self.expected_cid.as_str()
    }

    /// Return the parsed CID in canonical form, or empty for invalid text.
    pub fn actual_cid(&self) -> &str {
        self.actual_cid.as_str()
    }

    /// Transfer the canonical strings to a transport adapter without copying.
    pub fn into_parts(self) -> (CidVerificationStatus, String, String) {
        (self.status, self.expected_cid, self.actual_cid)
    }
}

/// Validate one canonical DAG-CBOR block and compare its CID text.
///
/// # Errors
///
/// Returns a typed DAG-CBOR error when the payload is malformed, noncanonical,
/// or exceeds the parser's resource limits. Use [`compute_cid_dag_cbor`] when
/// intentionally hashing opaque bytes without validating a DAG-CBOR block.
pub fn verify_dag_cbor_cid(
    cid_str: &str,
    bytes: &[u8],
) -> Result<DagCborCidVerification, CborError> {
    let _validated = Zeroizing::new(decode_dag_cbor(bytes)?);
    let expected_hash = dag_cbor_multihash(bytes);
    let expected_cid = Cid::new_v1(DAG_CBOR_CODEC, expected_hash.0);
    let expected = expected_cid.to_string();
    let Some((actual_cid, _base)) = parse_cid_string(cid_str) else {
        return Ok(DagCborCidVerification {
            status: CidVerificationStatus::InvalidCid,
            expected_cid: expected,
            actual_cid: String::new(),
        });
    };
    let actual = actual_cid.to_string();
    let status = if cid_str != actual {
        CidVerificationStatus::NonCanonical
    } else if expected_cid == actual_cid {
        CidVerificationStatus::Match
    } else {
        CidVerificationStatus::Mismatch
    };
    Ok(DagCborCidVerification {
        status,
        expected_cid: expected,
        actual_cid: actual,
    })
}

/// Returns whether `s` parses as a valid CID string.
pub fn is_valid_cid_string(s: &str) -> bool {
    try_parse_cid(s).is_some()
}

/// Parses `s` as a CID, returning `None` if it is not a valid CID string.
///
/// Accepts CIDv0 and multibase CID strings up to [`MAX_CID_STRING_LEN`]. Paths,
/// non-minimal binary encodings, and trailing decoded bytes are rejected.
pub fn try_parse_cid(s: &str) -> Option<ParsedCid> {
    parse_cid_string(s).map(|(cid, _base)| ParsedCid(cid))
}

fn parse_cid_string(s: &str) -> Option<(Cid, Option<Base>)> {
    if s.len() > MAX_CID_STRING_LEN {
        return None;
    }
    // The upstream string convenience parser also extracts CIDs from paths.
    // Decode the entire identifier ourselves so no prefix can be discarded.
    let (base, decoded) = if s.len() == CID_V0_STRING_LEN && s.starts_with("Qm") {
        (None, Base::Base58Btc.decode(s).ok()?)
    } else {
        let (base, bytes) = multibase_decode(s).ok()?;
        (Some(base), bytes)
    };
    let mut remaining = decoded.as_slice();
    let cid = Cid::read_bytes(&mut remaining).ok()?;
    // CIDv0 has exactly one canonical textual representation: bare base58btc.
    if base.is_some() && cid.version() == Version::V0 {
        return None;
    }
    // read_bytes is a stream parser. Exhaustion is essential for validating
    // an identifier, and byte equality also enforces minimal varint forms.
    if !remaining.is_empty() || cid.to_bytes() != decoded {
        return None;
    }
    Some((cid, base))
}
