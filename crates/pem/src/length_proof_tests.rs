// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{preflight_pem_encoded_length, PemEncodeOptions, PemError, PemLabel};
use crate::PemLineEnding;

#[kani::proof]
fn pem_preflight_computes_the_exact_default_width_armor_length() {
    const LINE_WIDTH: usize = 64;
    const MAX_DER: usize = 1_048_576;
    let der: usize = kani::any();
    let cap: usize = kani::any();
    let crlf: bool = kani::any();
    let label: u8 = kani::any();
    kani::assume(cap <= MAX_DER && label < 3);
    let (label, label_bytes) = match label {
        0 => (PemLabel::PrivateKey, 11_u64),
        1 => (PemLabel::EcPrivateKey, 14),
        _ => (PemLabel::PublicKey, 10),
    };
    let options = PemEncodeOptions {
        max_der_len: cap,
        line_width: LINE_WIDTH,
        line_ending: if crlf {
            PemLineEnding::Crlf
        } else {
            PemLineEnding::Lf
        },
    };
    let actual = preflight_pem_encoded_length(label, der, options);
    if der == 0 {
        assert!(matches!(actual, Err(PemError::EmptyDer)));
    } else if der > cap {
        assert!(matches!(actual, Err(PemError::DerTooLarge)));
    } else {
        // Count complete groups and any padded group independently of
        // the dependency. The last encoded column fixes the line count.
        let der = u64::try_from(der).unwrap();
        let body = 4 * (der / 3 + u64::from(der % 3 != 0));
        let body_lines = (body - 1) / 64 + 1;
        let newline = if crlf { 2 } else { 1 };
        let expected = 30 + 2 * label_bytes + body + (body_lines + 2) * newline;
        assert!(matches!(actual, Ok(length) if u64::try_from(length) == Ok(expected)));
    }
    kani::cover!(der == 0, "empty DER is rejected");
    kani::cover!(der > cap, "DER over the cap is rejected");
    kani::cover!(der == 48 && der <= cap && !crlf, "one complete armor line");
    kani::cover!(
        der == 49 && der <= cap && crlf,
        "a padded group begins a new CRLF line"
    );
}
