// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Fuzz the DAG-CBOR decoder and CID verifier on arbitrary bytes.
//! Property: decoding untrusted, possibly-malformed CBOR (deep nesting,
//! truncated lengths, bogus tags) must never panic, overflow, or run
//! unbounded.

#![no_main]

use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let _ = codec_cbor::decode_dag_cbor(data);

    // Split the input so the tail also drives the CID string parser and the
    // CID-over-bytes verifier without either allocating unbounded memory.
    if let Some((head, tail)) = data.split_first() {
        let cid_len = (*head as usize).min(tail.len());
        if let Ok(cid_text) = core::str::from_utf8(&tail[..cid_len]) {
            let _ = codec_cbor::try_parse_cid(cid_text);
            let _ = codec_cbor::verify_dag_cbor_cid(cid_text, &tail[cid_len..]);
        }
    }
});
