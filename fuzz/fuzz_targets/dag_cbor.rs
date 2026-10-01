// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Fuzz the DAG-CBOR decoder and CID verifier on arbitrary bytes.
//! Property: decoding untrusted, possibly-malformed CBOR (deep nesting,
//! truncated lengths, bogus tags) must never panic, overflow, or run
//! unbounded.

#![no_main]

use codec_cbor::CidVerificationStatus;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let canonical_cid = codec_cbor::compute_cid_dag_cbor(data);
    let verification = codec_cbor::verify_dag_cbor_cid(&canonical_cid, data);
    match codec_cbor::decode_dag_cbor(data) {
        Ok(value) => {
            // A matching digest is meaningful only after the canonical DAG
            // profile accepts the block. Exercise the encoder as an oracle.
            assert_eq!(codec_cbor::encode_dag_cbor(&value).as_deref(), Ok(data));
            let verified = verification.expect("canonical block must verify");
            assert_eq!(verified.status(), CidVerificationStatus::Match);
            assert_eq!(verified.expected_cid(), canonical_cid);
            assert_eq!(verified.actual_cid(), canonical_cid);
        }
        Err(error) => assert_eq!(verification.err(), Some(error)),
    }

    // Split the input so the tail also drives the CID string parser and the
    // CID-over-bytes verifier without either allocating unbounded memory.
    if let Some((head, tail)) = data.split_first() {
        let cid_len = (*head as usize).min(tail.len());
        if let Ok(cid_text) = core::str::from_utf8(&tail[..cid_len]) {
            let _ = codec_cbor::try_parse_cid(cid_text);
            if let Ok(verified) = codec_cbor::verify_dag_cbor_cid(cid_text, &tail[cid_len..]) {
                if verified.status() == CidVerificationStatus::Match {
                    assert_eq!(verified.expected_cid(), verified.actual_cid());
                    assert_eq!(verified.expected_cid(), cid_text);
                }
            }
        }
    }
});
