// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use reallyme_codec::der::{decode_der, decode_unsigned_integer_pair, DerLimits};

#[test]
fn ber_feature_unification_does_not_change_the_codec_profile() {
    for input in [
        &[0x30, 0x80, 0x02, 1, 1, 0x02, 1, 2, 0, 0][..],
        &[0x30, 0x81, 6, 0x02, 1, 1, 0x02, 1, 2],
        &[0x30, 6, 0x22, 1, 1, 0x02, 1, 2],
        &[0x30, 6, 0x02, 1, 0x80, 0x02, 1, 2],
    ] {
        assert!(decode_unsigned_integer_pair(input, DerLimits::default()).is_err());
    }
    assert!(
        decode_der(&[0x24, 3, 0x04, 1, 1], DerLimits::default(), |r| r
            .read_octet_string())
        .is_err()
    );
    assert!(decode_der(&[0x03, 2, 1, 1], DerLimits::default(), |r| r
        .read_bit_string())
    .is_err());
    assert!(
        decode_der(&[0x06, 3, 0x2a, 0x80, 1], DerLimits::default(), |r| r
            .read_object_identifier())
        .is_err()
    );
}
