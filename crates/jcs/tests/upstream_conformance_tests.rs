// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![allow(missing_docs)]

use codec_jcs::{canonicalize_json_text, JcsError};
use sha2::{Digest, Sha256};

const FIXTURES: [(&str, &[u8], &[u8]); 6] = [
    (
        "arrays.json",
        include_bytes!("fixtures/cyberphone/input/arrays.json"),
        include_bytes!("fixtures/cyberphone/output/arrays.json"),
    ),
    (
        "french.json",
        include_bytes!("fixtures/cyberphone/input/french.json"),
        include_bytes!("fixtures/cyberphone/output/french.json"),
    ),
    (
        "structures.json",
        include_bytes!("fixtures/cyberphone/input/structures.json"),
        include_bytes!("fixtures/cyberphone/output/structures.json"),
    ),
    (
        "unicode.json",
        include_bytes!("fixtures/cyberphone/input/unicode.json"),
        include_bytes!("fixtures/cyberphone/output/unicode.json"),
    ),
    (
        "values.json",
        include_bytes!("fixtures/cyberphone/input/values.json"),
        include_bytes!("fixtures/cyberphone/output/values.json"),
    ),
    (
        "weird.json",
        include_bytes!("fixtures/cyberphone/input/weird.json"),
        include_bytes!("fixtures/cyberphone/output/weird.json"),
    ),
];

#[test]
fn upstream_fixture_bytes_match_the_reviewed_commit() {
    let mut digest = Sha256::new();
    for directory in ["input", "output"] {
        for (name, input, output) in FIXTURES {
            let bytes = if directory == "input" { input } else { output };
            digest.update(directory.as_bytes());
            digest.update(b"/");
            digest.update(name.as_bytes());
            digest.update([0]);
            digest.update(bytes);
        }
    }
    let actual_digest: String = digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect();
    assert_eq!(
        actual_digest,
        "4f464dc75b68c8310a2e18b64b388342bcce2dd90b5ffa55a169c5b628a6b2be"
    );
}

#[test]
#[allow(clippy::expect_used)]
fn upstream_jcs_fixtures_match_except_for_the_documented_strict_subset() {
    for (name, input, expected) in FIXTURES {
        let input = core::str::from_utf8(input).expect("upstream JSON fixture is UTF-8");
        if name == "values.json" {
            // The upstream RFC suite permits whole-valued binary64 numbers
            // outside the I-JSON integer range; this crate deliberately does not.
            assert_eq!(
                canonicalize_json_text(input),
                Err(JcsError::IntegerOutsideInteroperableRange)
            );
            continue;
        }
        let canonical = canonicalize_json_text(input).expect("upstream JSON is supported");
        let expected = core::str::from_utf8(expected).expect("upstream output is UTF-8");
        assert_eq!(canonical, expected, "upstream fixture {name}");
    }
}
