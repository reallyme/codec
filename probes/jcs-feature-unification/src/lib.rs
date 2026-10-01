// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#[cfg(test)]
mod tests {
    use codec_jcs::{canonicalize_json_text, JcsError};

    #[test]
    fn feature_unification_never_emits_feature_dependent_number_bytes() {
        for input in [
            r#"{"a":1.5}"#,
            r#"{"a":12345678901234567890123}"#,
            r#"{"a":{"$serde_json::private::Number":"1.5"}}"#,
        ] {
            assert_eq!(
                canonicalize_json_text(input),
                Err(JcsError::UnsupportedNumberRepresentation)
            );
        }
    }
}
