// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(missing_docs)]

use codec_cbor::{
    compute_cid_dag_cbor, decode_dag_cbor, encode_dag_cbor, verify_dag_cbor_cid, CborError,
    CidVerificationStatus,
};
use sha2::{Digest, Sha256};

const FIXTURES: [(&str, &[u8], &str); 5] = [
    (
        "array-2.dag-cbor",
        include_bytes!("fixtures/ipld/array-2.dag-cbor"),
        "bafyreihdb57fdysx5h35urvxz64ros7zvywshber7id6t6c6fek37jgyfe",
    ),
    (
        "false.dag-cbor",
        include_bytes!("fixtures/ipld/false.dag-cbor"),
        "bafyreibac77tiyjzkzzkucve6zejj7jpswslcihcnehisulfnv423qxo2i",
    ),
    (
        "int-255.dag-cbor",
        include_bytes!("fixtures/ipld/int-255.dag-cbor"),
        "bafyreih4vluto2froiw457akazzjhcfm7y22juemxx6jsyyjufp227tcv4",
    ),
    (
        "map-1-pair.dag-cbor",
        include_bytes!("fixtures/ipld/map-1-pair.dag-cbor"),
        "bafyreihltcnuuyqp2jm24aqydpnlj7b6w3ogwrplomrjtg5rifv44mmjey",
    ),
    (
        "string-a.dag-cbor",
        include_bytes!("fixtures/ipld/string-a.dag-cbor"),
        "bafyreiewdnw5h3pdzohmxkwl22g6aqgnpdvs5vmiseymz22mjeti5jgvay",
    ),
];

#[test]
fn upstream_fixture_bytes_match_the_reviewed_commit() {
    let mut digest = Sha256::new();
    for (name, bytes, _) in FIXTURES {
        digest.update(name.as_bytes());
        digest.update([0]);
        digest.update(bytes);
    }
    let actual_digest: String = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(
        actual_digest,
        "2e92a7371662a7c2060079f2485c68101887bb04f2cd94830f7e3001353f69e9"
    );
}

#[test]
#[allow(clippy::expect_used)]
fn upstream_dag_cbor_fixtures_round_trip_and_match_their_cids() {
    for (name, bytes, expected_cid) in FIXTURES {
        let value = decode_dag_cbor(bytes).expect("upstream DAG-CBOR fixture must decode");
        assert_eq!(
            encode_dag_cbor(&value).expect("upstream DAG-CBOR fixture must encode"),
            bytes,
            "upstream fixture {name}"
        );
        assert_eq!(compute_cid_dag_cbor(bytes), expected_cid, "{name}");
        assert_eq!(
            verify_dag_cbor_cid(expected_cid, bytes)
                .expect("upstream CID must parse")
                .status(),
            CidVerificationStatus::Match,
            "upstream fixture {name}"
        );
    }
}

#[test]
fn upstream_duplicate_key_fixture_is_rejected() {
    // negative-fixtures/dag-cbor/decode/duplicate-keys.json in the same
    // upstream commit records this exact hexadecimal payload.
    let duplicate_keys = [
        0xa3, 0x63, 0x62, 0x61, 0x72, 0x03, 0x63, 0x66, 0x6f, 0x6f, 0x01, 0x63, 0x66, 0x6f, 0x6f,
        0x02,
    ];
    assert_eq!(
        decode_dag_cbor(&duplicate_keys),
        Err(CborError::DuplicateMapKey)
    );
}
