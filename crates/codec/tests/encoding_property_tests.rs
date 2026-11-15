// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Round-trip and strict admission properties for the public encoders.

#![cfg(all(
    feature = "base64",
    feature = "base64url",
    feature = "hex",
    feature = "multibase",
    feature = "pem"
))]
#![allow(clippy::unwrap_used, reason = "property failures stop the test")]

use proptest::collection::vec;
use proptest::prelude::{any, prop_assert, prop_assert_eq, proptest};
use reallyme_codec::base64::{base64_to_bytes, bytes_to_base64};
use reallyme_codec::base64url::{base64url_to_bytes, bytes_to_base64url};
use reallyme_codec::hex::{bytes_to_lower_hex, lower_hex_to_bytes};
use reallyme_codec::multibase::{
    base58btc_decode, base58btc_encode, bytes_to_multibase58btc, bytes_to_multibase_base64url,
    multibase_to_bytes,
};
use reallyme_codec::pem::{
    decode_pem, encode_pem, PemDecodePolicy, PemEncodeOptions, PemError, PemLabel, PemLineEnding,
};

proptest! {
    #[test]
    fn binary_encodings_round_trip_and_reject_invalid_suffixes(bytes in vec(any::<u8>(), 0..128)) {
        let standard = bytes_to_base64(&bytes);
        let url = bytes_to_base64url(&bytes);
        let hex = bytes_to_lower_hex(&bytes);
        let btc = base58btc_encode(&bytes).unwrap();
        prop_assert_eq!(&base64_to_bytes(&standard).unwrap(), &bytes);
        prop_assert_eq!(&base64url_to_bytes(&url).unwrap(), &bytes);
        prop_assert_eq!(&lower_hex_to_bytes(&hex).unwrap(), &bytes);
        prop_assert_eq!(&base58btc_decode(&btc).unwrap(), &bytes);
        prop_assert!(base64_to_bytes(&(standard + "*")).is_err());
        prop_assert!(base64url_to_bytes(&(url + "=")).is_err());
        prop_assert!(lower_hex_to_bytes(&(hex + "ffXX")).is_err());
        prop_assert!(base58btc_decode(&(btc + "0")).is_err());
    }

    #[test]
    fn multibase_round_trip_retains_each_canonical_alphabet(bytes in vec(any::<u8>(), 0..128)) {
        let btc = bytes_to_multibase58btc(&bytes).unwrap();
        let url = bytes_to_multibase_base64url(&bytes).unwrap();
        let btc_bytes = multibase_to_bytes(&btc).unwrap();
        let url_bytes = multibase_to_bytes(&url).unwrap();
        prop_assert_eq!(&btc_bytes, &bytes);
        prop_assert_eq!(&url_bytes, &bytes);
        prop_assert_eq!(bytes_to_multibase58btc(&btc_bytes).unwrap(), btc);
        prop_assert_eq!(bytes_to_multibase_base64url(&url_bytes).unwrap(), url);
    }

    #[test]
    fn pem_round_trip_preserves_the_selected_armor_policy(
        bytes in vec(any::<u8>(), 1..256), width in 1_usize..=76, crlf in any::<bool>(),
        label_index in 0_usize..3
    ) {
        let label = [PemLabel::PublicKey, PemLabel::PrivateKey, PemLabel::EcPrivateKey][label_index];
        let options = PemEncodeOptions {
            max_der_len: bytes.len(),
            line_width: width,
            line_ending: if crlf { PemLineEnding::Crlf } else { PemLineEnding::Lf },
        };
        let wrapped = encode_pem(label, &bytes, options).unwrap();
        let policy = PemDecodePolicy {
            allowed_labels: &[label],
            max_der_len: bytes.len(),
            ..PemDecodePolicy::default()
        };
        let decoded = decode_pem(&wrapped, policy).unwrap();
        prop_assert_eq!(decoded.label, label);
        prop_assert_eq!(decoded.der.as_slice(), bytes.as_slice());
        let reencoded = encode_pem(label, &decoded.der, options).unwrap();
        prop_assert_eq!(reencoded.as_str(), wrapped.as_str());

        // A nonempty line after the END boundary is outside the document.
        let mut with_suffix = wrapped;
        with_suffix.push('X');
        prop_assert!(matches!(decode_pem(&with_suffix, policy), Err(PemError::InvalidBoundary)));
    }
}
