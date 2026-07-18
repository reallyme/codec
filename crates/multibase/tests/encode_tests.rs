// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use codec_multibase::{base58btc_encode, bytes_to_multibase58btc, bytes_to_multibase_base64url};

#[test]
fn base64url_multibase_encodes_directly_to_exact_sized_output() {
    let bytes = b"sensitive document bytes";
    let encoded = bytes_to_multibase_base64url(bytes).unwrap();

    assert_eq!(encoded, "uc2Vuc2l0aXZlIGRvY3VtZW50IGJ5dGVz");
    assert_eq!(encoded.capacity(), encoded.len());
}

#[test]
fn base64url_multibase_empty_payload_is_only_prefix() {
    let encoded = bytes_to_multibase_base64url(b"").unwrap();

    assert_eq!(encoded, "u");
    assert_eq!(encoded.capacity(), encoded.len());
}

#[test]
fn base58btc_multibase_matches_unprefixed_encoding_with_single_prefix() {
    let bytes = b"identifier payload";
    let prefixed = bytes_to_multibase58btc(bytes).unwrap();
    let unprefixed = base58btc_encode(bytes).unwrap();

    assert_eq!(prefixed, ["z", unprefixed.as_str()].concat());
}
