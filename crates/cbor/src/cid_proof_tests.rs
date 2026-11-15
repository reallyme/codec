// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{
    classify_parsed_cid, dag_cbor_cid, parse_cid_wire, take_varint, CidVerificationStatus,
    ContentHash, DagCborMultihash,
};
use cid::{Cid, Version};
use multihash::Multihash;

// Use the wire registry values so a change in the expected CID parameters
// cannot change the reference decision along with the implementation.
const DAG_CBOR: u64 = 0x71;
const SHA2_256: u64 = 0x12;
const SHA2_256_DIGEST_BYTES: usize = 32;
const MAX_DIGEST_BYTES: usize = 64;
const CID_V1_VERSION: u8 = 1;
const DAG_PB: u64 = 0x70;
const CID_V1_VARINT_BYTES: usize = 10;

#[kani::proof]
#[kani::unwind(12)]
fn cid_varints_match_unsigned_base128_u64() {
    let bytes: [u8; 11] = kani::any();
    let available: usize = kani::any();
    kani::assume(available <= bytes.len());
    let source = &bytes[..available];

    // Locate the terminator independently and evaluate the positional
    // representation in u128 so an out-of-range value cannot wrap.
    let expected = source.iter().position(|byte| byte < &128).and_then(|last| {
        if last >= 10 || (last > 0 && source[last] == 0) {
            return None;
        }
        let value = source[..=last]
            .iter()
            .rev()
            .fold(0_u128, |acc, byte| acc * 128 + u128::from(byte % 128));
        Some((u64::try_from(value).ok()?, &source[last + 1..]))
    });
    let actual = take_varint(source);
    assert_eq!(actual, expected);
    kani::cover!(
        matches!(actual, Some((u64::MAX, _))),
        "ten-octet uint64 maximum"
    );
    kani::cover!(
        available == 10 && bytes[..9] == [0x80; 9] && bytes[9] == 2 && actual.is_none(),
        "tenth-octet overflow"
    );
    kani::cover!(
        available == 2 && bytes[..2] == [0x80, 0] && actual.is_none(),
        "redundant high zero group"
    );
    kani::cover!(
        available > 1 && matches!(actual, Some((0, rest)) if rest.len() == available - 1),
        "exact prefix consumption"
    );
}

#[kani::proof]
#[kani::unwind(66)]
fn cid_v1_wire_requires_an_exact_supported_digest_length() {
    let digest: [u8; MAX_DIGEST_BYTES + 1] = kani::any();
    let declared: u8 = kani::any();
    let supplied: usize = kani::any();
    kani::assume(declared < 128);
    kani::assume(supplied <= digest.len());

    let mut wire = [0; 4 + MAX_DIGEST_BYTES + 1];
    // The version, codec, hash algorithm and small digest lengths each use
    // exactly one octet in this family of inputs.
    wire[..4].copy_from_slice(&[CID_V1_VERSION, 0x71, 0x12, declared]);
    wire[4..].copy_from_slice(&digest);
    let parsed = parse_cid_wire(&wire[..4 + supplied], true);
    let expected = supplied == usize::from(declared) && supplied <= MAX_DIGEST_BYTES;
    assert_eq!(parsed.is_some(), expected);
    if let Some(cid) = parsed {
        assert_eq!(cid.version(), Version::V1);
        assert_eq!(cid.codec(), DAG_CBOR);
        assert_eq!(cid.hash().code(), SHA2_256);
        assert_eq!(cid.hash().size(), declared);
        assert_eq!(cid.hash().digest(), &digest[..supplied]);
    }
    kani::cover!(
        declared == 0 && parsed.is_some(),
        "empty digest admitted structurally"
    );
    kani::cover!(
        declared == 64 && parsed.is_some(),
        "maximum digest admitted"
    );
    kani::cover!(
        declared == 65 && supplied == 65 && parsed.is_none(),
        "digest exceeds capacity"
    );
    kani::cover!(
        declared == 32 && supplied == 31 && parsed.is_none(),
        "truncated digest"
    );
    kani::cover!(
        declared == 32 && supplied == 33 && parsed.is_none(),
        "trailing byte"
    );
}

#[kani::proof]
#[kani::unwind(34)]
fn cid_v0_wire_requires_the_bare_fixed_shape() {
    let digest: [u8; SHA2_256_DIGEST_BYTES + 1] = kani::any();
    let first: u8 = kani::any();
    let second: u8 = kani::any();
    let supplied: usize = kani::any();
    let has_multibase_prefix: bool = kani::any();
    kani::assume(first < 128 && first != CID_V1_VERSION && second < 128);
    kani::assume(supplied <= digest.len());

    let mut wire = [0; 2 + SHA2_256_DIGEST_BYTES + 1];
    wire[..2].copy_from_slice(&[first, second]);
    wire[2..].copy_from_slice(&digest);
    let parsed = parse_cid_wire(&wire[..2 + supplied], has_multibase_prefix);
    let expected =
        first == 0x12 && second == 32 && supplied == SHA2_256_DIGEST_BYTES && !has_multibase_prefix;
    assert_eq!(parsed.is_some(), expected);
    if let Some(cid) = parsed {
        assert_eq!(cid.version(), Version::V0);
        assert_eq!(cid.codec(), DAG_PB);
        assert_eq!(cid.hash().code(), SHA2_256);
        assert_eq!(cid.hash().digest(), &digest[..supplied]);
    }
    kani::cover!(parsed.is_some(), "bare CIDv0");
    kani::cover!(
        first == 0x12 && second == 32 && supplied == 32 && has_multibase_prefix && parsed.is_none(),
        "wrapped CIDv0"
    );
    kani::cover!(
        first == 0x12
            && second == 32
            && supplied == 33
            && !has_multibase_prefix
            && parsed.is_none(),
        "CIDv0 with trailing data"
    );
    kani::cover!(
        first == 0 && second == 32 && supplied == 32 && parsed.is_none(),
        "explicit version zero"
    );
}

#[kani::proof]
#[kani::unwind(14)]
fn cid_v1_wire_requires_minimal_varints_in_each_field() {
    // Each original field fits in one octet. Adding zero high groups never
    // changes its value; it only makes the selected field non-minimal.
    let field: usize = kani::any();
    let extra_groups: usize = kani::any();
    kani::assume(field < 4);
    kani::assume(extra_groups <= 10);
    let canonical: [u8; 5] = [CID_V1_VERSION, 0x71, 0x12, 1, kani::any()];
    let mut wire = [0x80; 15];
    wire[..field].copy_from_slice(&canonical[..field]);
    if extra_groups != 0 {
        wire[field] = canonical[field] | 0x80;
        wire[field + extra_groups] = 0;
    }
    let end = canonical.len() + extra_groups;
    wire[field + extra_groups + 1..end].copy_from_slice(&canonical[field + 1..]);
    if extra_groups == 0 {
        wire[field] = canonical[field];
    }
    let parsed = parse_cid_wire(&wire[..end], true);
    assert_eq!(parsed.is_some(), extra_groups == 0);
    kani::cover!(parsed.is_some(), "canonical CIDv1");
    kani::cover!(
        field == 0 && extra_groups == 1 && parsed.is_none(),
        "overlong version"
    );
    kani::cover!(
        field == 1 && extra_groups == 9 && parsed.is_none(),
        "overlong codec at u64 varint width"
    );
    kani::cover!(
        field == 2 && extra_groups == 1 && parsed.is_none(),
        "overlong hash algorithm"
    );
    kani::cover!(
        field == 3 && extra_groups == 10 && parsed.is_none(),
        "overlong digest length past u64 varint width"
    );
}

#[kani::proof]
#[kani::unwind(13)]
fn cid_v1_wire_preserves_both_variable_width_registry_codes() {
    const DIGEST_LEN: u8 = 3;
    const WIRE_CAPACITY: usize = 1 + 2 * CID_V1_VARINT_BYTES + 1 + 3;
    let codec: u64 = kani::any();
    let algorithm: u64 = kani::any();
    let digest: [u8; 3] = kani::any();
    let complete: bool = kani::any();
    let (mut codec_bytes, mut algorithm_bytes) =
        ([0; CID_V1_VARINT_BYTES], [0; CID_V1_VARINT_BYTES]);
    let mut wire = [0; WIRE_CAPACITY];
    let Some(bytes) = compose_cid_v1_wire(
        reference_varint(codec, &mut codec_bytes),
        reference_varint(algorithm, &mut algorithm_bytes),
        if complete { DIGEST_LEN } else { DIGEST_LEN - 1 },
        &digest,
        &mut wire,
    ) else {
        kani::assert(false, "both codes, the declared length and the digest fit");
        return;
    };
    let parsed = parse_cid_wire(bytes, true);
    assert_eq!(parsed.is_some(), complete);
    if let Some(cid) = parsed {
        assert_eq!(cid.version(), Version::V1);
        assert_eq!(cid.codec(), codec);
        assert_eq!(cid.hash().code(), algorithm);
        assert_eq!(cid.hash().size(), DIGEST_LEN);
        assert_eq!(cid.hash().digest(), &digest);
    }
    kani::cover!(
        complete && codec < 128 && algorithm < 128,
        "both codes use one octet"
    );
    kani::cover!(
        complete && codec >= 128 && algorithm < 128,
        "multi-octet codec only"
    );
    kani::cover!(
        complete && codec < 128 && algorithm >= 128,
        "multi-octet algorithm only"
    );
    kani::cover!(
        complete && codec == u64::MAX && algorithm == u64::MAX,
        "both codes use ten octets"
    );
    kani::cover!(
        !complete && codec >= 128 && algorithm >= 128,
        "both codes followed by excess digest data"
    );
}

fn reference_varint(value: u64, output: &mut [u8; CID_V1_VARINT_BYTES]) -> Option<&[u8]> {
    // Repeated division emits base-128 digits independently of the reader's
    // shifts. The final octet is the first after which no digits remain.
    let mut remaining = value;
    for (index, octet) in output.iter_mut().enumerate() {
        let digit = u8::try_from(remaining % 128).ok()?;
        remaining /= 128;
        if remaining == 0 {
            *octet = digit;
            return output.get(..=index);
        }
        *octet = digit | 0x80;
    }
    None
}

fn compose_cid_v1_wire<'a>(
    codec: Option<&[u8]>,
    algorithm: Option<&[u8]>,
    length: u8,
    digest: &[u8],
    wire: &'a mut [u8],
) -> Option<&'a [u8]> {
    let codec = codec?;
    let algorithm = algorithm?;
    let length_offset = 1_usize
        .checked_add(codec.len())?
        .checked_add(algorithm.len())?;
    let digest_offset = length_offset.checked_add(1)?;
    let end = digest_offset.checked_add(digest.len())?;
    let wire = wire.get_mut(..end)?;
    let (prefix, body) = wire.split_at_mut_checked(length_offset)?;
    let (version, codes) = prefix.split_first_mut()?;
    *version = CID_V1_VERSION;
    let (codec_field, algorithm_field) = codes.split_at_mut_checked(codec.len())?;
    codec_field.copy_from_slice(codec);
    algorithm_field.copy_from_slice(algorithm);
    let (length_field, digest_field) = body.split_first_mut()?;
    *length_field = length;
    digest_field.copy_from_slice(digest);
    Some(wire)
}

#[kani::proof]
#[kani::unwind(65)]
fn cid_match_requires_canonical_text_and_the_complete_digest_identity() {
    let expected_digest: ContentHash = kani::any();
    let actual_digest: [u8; MAX_DIGEST_BYTES] = kani::any();
    let actual_len: usize = kani::any();
    let actual_codec: u64 = kani::any();
    let actual_algorithm: u64 = kani::any();
    let v0: bool = kani::any();
    let text_matches: bool = kani::any();
    kani::assume(actual_len <= actual_digest.len());

    let expected = dag_cbor_cid(DagCborMultihash(
        Multihash::wrap(u64::from(super::Code::Sha2_256), &expected_digest).unwrap(),
    ));
    let hash = Multihash::wrap(actual_algorithm, &actual_digest[..actual_len]).unwrap();
    let actual = if v0 {
        // A parsed v0 is constructible only for a full SHA-256 digest.
        kani::assume(actual_algorithm == SHA2_256 && actual_len == SHA2_256_DIGEST_BYTES);
        Cid::new_v0(hash).unwrap()
    } else {
        Cid::new_v1(actual_codec, hash)
    };

    // Text rendering and SHA-256 are covered separately. Model equality with
    // the canonical rendering using two distinct representative strings.
    let canonical = "canonical";
    let supplied = if text_matches { canonical } else { "alternate" };
    let status = classify_parsed_cid(supplied, canonical, &expected, &actual);
    let same_digest = actual_digest[..SHA2_256_DIGEST_BYTES] == expected_digest;
    let same_identity = !v0
        && actual_codec == DAG_CBOR
        && actual_algorithm == SHA2_256
        && actual_len == SHA2_256_DIGEST_BYTES
        && same_digest;
    let reference = if !text_matches {
        CidVerificationStatus::NonCanonical
    } else if same_identity {
        CidVerificationStatus::Match
    } else {
        CidVerificationStatus::Mismatch
    };

    kani::cover!(
        status == CidVerificationStatus::Match,
        "canonical matching CID"
    );
    kani::cover!(
        !text_matches && same_identity && status == CidVerificationStatus::NonCanonical,
        "alternate text for matching CID"
    );
    kani::cover!(
        v0 && same_digest && status == CidVerificationStatus::Mismatch,
        "v0 with matching digest"
    );
    kani::cover!(
        !v0 && actual_codec != DAG_CBOR
            && same_digest
            && actual_algorithm == SHA2_256
            && actual_len == SHA2_256_DIGEST_BYTES
            && status == CidVerificationStatus::Mismatch,
        "wrong codec with matching digest"
    );
    kani::cover!(
        !v0 && actual_codec == DAG_CBOR
            && same_digest
            && actual_algorithm != SHA2_256
            && actual_len == SHA2_256_DIGEST_BYTES
            && status == CidVerificationStatus::Mismatch,
        "wrong algorithm with matching digest"
    );
    kani::cover!(
        !v0 && actual_codec == DAG_CBOR
            && same_digest
            && actual_algorithm == SHA2_256
            && actual_len < SHA2_256_DIGEST_BYTES
            && status == CidVerificationStatus::Mismatch,
        "truncated digest with matching prefix"
    );
    kani::cover!(
        !v0 && actual_codec == DAG_CBOR
            && !same_digest
            && actual_algorithm == SHA2_256
            && actual_len == SHA2_256_DIGEST_BYTES
            && status == CidVerificationStatus::Mismatch,
        "wrong digest with matching parameters"
    );
    assert_eq!(status, reference);
    assert_eq!(actual.version() == Version::V0, v0);
}
