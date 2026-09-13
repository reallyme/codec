// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use cid::multibase::{decode as multibase_decode, Base};
use cid::Cid;
use multihash::Multihash;
use multihash_codetable::{Code, MultihashDigest};
use sha2::{Digest, Sha256};

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

/// Multihash envelope size used by the CID stack for sha2-256 digests.
pub type DagCborMultihash = Multihash<64>;

/// Returns the raw sha2-256 digest of `bytes`.
pub fn sha2_256_content_hash(bytes: &[u8]) -> ContentHash {
    Sha256::digest(bytes).into()
}

/// Returns a sha2-256 multihash of `bytes` for use in a CID.
pub fn dag_cbor_multihash(bytes: &[u8]) -> DagCborMultihash {
    Code::Sha2_256.digest(bytes)
}

/// Computes the CIDv1 (dag-cbor, sha2-256) of `bytes` in canonical
/// base32-lower string form.
///
/// Hashes the supplied bytes as-is, without parsing CBOR or applying the
/// encoder/decoder size limit. Encode a value first to obtain a canonical block.
pub fn compute_cid_dag_cbor(bytes: &[u8]) -> String {
    let hash = dag_cbor_multihash(bytes);
    let cid = Cid::new_v1(DAG_CBOR_CODEC, hash);
    cid.to_string()
}

/// Recomputes the CID of `bytes` and compares it to `cid_str`.
///
/// Returns whether the parsed CID values match, plus the expected CID and the
/// parsed actual CID in canonical string form. Invalid CID input never matches
/// and returns an empty actual string so unvalidated caller input does not cross
/// diagnostic or FFI boundaries.
/// Like [`compute_cid_dag_cbor`], this hashes bytes without validating CBOR.
/// Verification rejects uppercase base16, base32, and base36 variants, including
/// uppercase payloads with lowercase prefixes; generic [`try_parse_cid`] also
/// accepts those case variants.
pub fn verify_dag_cbor_cid(cid_str: &str, bytes: &[u8]) -> (bool, String, String) {
    let expected_hash = dag_cbor_multihash(bytes);
    let expected_cid = Cid::new_v1(DAG_CBOR_CODEC, expected_hash);
    let expected = expected_cid.to_string();
    let Some(actual_cid) = parse_verification_cid(cid_str) else {
        return (false, expected, String::new());
    };
    let actual = actual_cid.to_string();
    (expected_cid == actual_cid, expected, actual)
}

fn parse_verification_cid(cid_str: &str) -> Option<Cid> {
    let (actual, base) = parse_cid_string(cid_str)?;
    let Some(base) = base else {
        return Some(actual);
    };
    if rejects_case_variant_base(base) || has_uppercase_payload_for_lowercase_base(base, cid_str) {
        return None;
    }
    Some(actual)
}

fn rejects_case_variant_base(base: Base) -> bool {
    matches!(
        base,
        Base::Base16Upper
            | Base::Base32Upper
            | Base::Base32PadUpper
            | Base::Base32HexUpper
            | Base::Base32HexPadUpper
            | Base::Base36Upper
    )
}

fn has_uppercase_payload_for_lowercase_base(base: Base, cid_str: &str) -> bool {
    if !matches!(
        base,
        Base::Base16Lower
            | Base::Base32Lower
            | Base::Base32PadLower
            | Base::Base32HexLower
            | Base::Base32HexPadLower
            | Base::Base36Lower
    ) {
        return false;
    }
    cid_str
        .get(1..)
        .is_some_and(|payload| payload.bytes().any(|byte| byte.is_ascii_uppercase()))
}

/// Returns whether `s` parses as a valid CID string.
pub fn is_valid_cid_string(s: &str) -> bool {
    try_parse_cid(s).is_some()
}

/// Parses `s` as a CID, returning `None` if it is not a valid CID string.
///
/// Accepts CIDv0 and multibase CID strings up to [`MAX_CID_STRING_LEN`]. Paths,
/// non-minimal binary encodings, and trailing decoded bytes are rejected.
pub fn try_parse_cid(s: &str) -> Option<Cid> {
    parse_cid_string(s).map(|(cid, _base)| cid)
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
    // read_bytes is a stream parser. Exhaustion is essential for validating
    // an identifier, and byte equality also enforces minimal varint forms.
    if !remaining.is_empty() || cid.to_bytes() != decoded {
        return None;
    }
    Some((cid, base))
}
