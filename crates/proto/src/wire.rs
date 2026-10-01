// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use buffa::{DecodeError, DecodeOptions, Message};
use serde::de::DeserializeOwned;
use zeroize::Zeroizing;

use crate::error::{
    malformed_json_error, malformed_protobuf_error, resource_limit_error, CodecWireResult,
};
use crate::limits::{
    CODEC_PROTO_RECURSION_LIMIT, CODEC_PROTO_UNKNOWN_FIELD_LIMIT,
    MAX_CODEC_PROTO_ELEMENT_MEMORY_BYTES, MAX_CODEC_PROTO_JSON_BYTES,
    MAX_CODEC_PROTO_JSON_NESTING_DEPTH, MAX_CODEC_PROTO_JSON_TOKENS, MAX_CODEC_PROTO_MESSAGE_BYTES,
    MAX_CODEC_PROTO_SEMANTIC_NODES,
};

#[derive(Clone, Copy)]
struct JsonContainer {
    is_array: bool,
    is_map_entries: bool,
    is_map_entry: bool,
    has_member: bool,
    expects_element: bool,
}

const EMPTY_JSON_CONTAINER: JsonContainer = JsonContainer {
    is_array: false,
    is_map_entries: false,
    is_map_entry: false,
    has_member: false,
    expects_element: false,
};

fn account_array_element(
    containers: &mut [JsonContainer; MAX_CODEC_PROTO_JSON_NESTING_DEPTH],
    depth: usize,
    element_units: &mut usize,
) -> CodecWireResult<()> {
    if depth == 0 {
        return Ok(());
    }
    let parent = &mut containers[depth - 1];
    if !parent.is_array || !parent.expects_element {
        return Ok(());
    }
    parent.expects_element = false;
    // A protobuf map entry represents a key and a value, even when malformed
    // JSON omits both fields. Charge that allocation before deserialization.
    let units = if parent.is_map_entries { 2 } else { 1 };
    *element_units = element_units
        .checked_add(units)
        .ok_or_else(resource_limit_error)?;
    if *element_units > MAX_CODEC_PROTO_SEMANTIC_NODES {
        return Err(resource_limit_error());
    }
    Ok(())
}

fn match_entries_character(position: &mut usize, matches: &mut bool, byte: u8) {
    const ENTRIES: &[u8] = b"entries";
    if *matches {
        *matches = ENTRIES.get(*position).copied() == Some(byte);
    }
    *position = position.saturating_add(1);
}

fn hex_digit(byte: u8) -> Option<u16> {
    match byte {
        b'0'..=b'9' => Some(u16::from(byte - b'0')),
        b'a'..=b'f' => Some(u16::from(byte - b'a') + 10),
        b'A'..=b'F' => Some(u16::from(byte - b'A') + 10),
        _ => None,
    }
}

/// Encodes a protobuf message with Buffa without invoking its panicking size path.
///
/// # Errors
///
/// Returns a resource-limit error when the message exceeds the protobuf size
/// limit. Encoding errors are mapped without retaining message contents.
pub fn encode_protobuf<M: Message>(message: &M) -> CodecWireResult<Zeroizing<Vec<u8>>> {
    message
        .try_encode_to_vec()
        .map(Zeroizing::new)
        .map_err(|_| resource_limit_error())
}

/// Decodes a bounded protobuf message from untrusted bytes.
///
/// # Errors
///
/// Returns a boundary wire error when input exceeds the size limit or fails
/// protobuf decoding.
pub fn decode_protobuf<M: Message>(bytes: &[u8]) -> CodecWireResult<M> {
    decode_protobuf_with_limit(bytes, MAX_CODEC_PROTO_MESSAGE_BYTES)
}

/// Decodes a generated protobuf message from proto3-compatible JSON bytes.
///
/// The decoded message is immediately re-encoded to protobuf so compact JSON
/// that expands past the binary protobuf cap is rejected at the same boundary
/// as native protobuf input.
///
/// # Errors
///
/// Returns a boundary wire error for oversized or malformed JSON and for JSON
/// messages whose binary encoding exceeds the protobuf cap.
pub fn decode_json<M: DeserializeOwned + Message>(bytes: &[u8]) -> CodecWireResult<M> {
    if bytes.len() > MAX_CODEC_PROTO_JSON_BYTES {
        return Err(resource_limit_error());
    }

    validate_json_nesting(bytes)?;
    let mut deserializer = serde_json::Deserializer::from_slice(bytes);
    deserializer.disable_recursion_limit();
    let message = <M as serde::Deserialize>::deserialize(&mut deserializer)
        .map_err(|_| malformed_json_error())?;
    deserializer.end().map_err(|_| malformed_json_error())?;
    let encoded = encode_protobuf(&message)?;
    if encoded.len() > MAX_CODEC_PROTO_MESSAGE_BYTES {
        return Err(resource_limit_error());
    }
    Ok(message)
}

fn validate_json_nesting(bytes: &[u8]) -> CodecWireResult<()> {
    let mut depth = 0_usize;
    let mut tokens = 0_usize;
    let mut element_units = 0_usize;
    let mut containers = [EMPTY_JSON_CONTAINER; MAX_CODEC_PROTO_JSON_NESTING_DEPTH];
    let mut in_string = false;
    let mut escaped = false;
    let mut in_number = false;
    let mut entries_position = 0_usize;
    let mut string_matches_entries = false;
    let mut last_string_is_entries = false;
    let mut pending_entries_array = false;
    let mut unicode_digits = 0_u8;
    let mut unicode_value = 0_u16;
    for byte in bytes {
        if in_string {
            if unicode_digits != 0 {
                if let Some(digit) = hex_digit(*byte) {
                    unicode_value = (unicode_value << 4) | digit;
                } else {
                    string_matches_entries = false;
                }
                unicode_digits -= 1;
                if unicode_digits == 0 {
                    if let Ok(decoded) = u8::try_from(unicode_value) {
                        match_entries_character(
                            &mut entries_position,
                            &mut string_matches_entries,
                            decoded,
                        );
                    } else {
                        string_matches_entries = false;
                    }
                    unicode_value = 0;
                }
            } else if escaped {
                escaped = false;
                match *byte {
                    b'u' => unicode_digits = 4,
                    b'"' | b'\\' | b'/' => match_entries_character(
                        &mut entries_position,
                        &mut string_matches_entries,
                        *byte,
                    ),
                    _ => string_matches_entries = false,
                }
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                in_string = false;
                last_string_is_entries = string_matches_entries && entries_position == 7;
            } else {
                match_entries_character(&mut entries_position, &mut string_matches_entries, *byte);
            }
            continue;
        }

        match *byte {
            b'"' => {
                account_array_element(&mut containers, depth, &mut element_units)?;
                if depth != 0 && containers[depth - 1].is_map_entry {
                    containers[depth - 1].has_member = true;
                }
                in_string = true;
                in_number = false;
                entries_position = 0;
                string_matches_entries = true;
                pending_entries_array = false;
                tokens = tokens.checked_add(1).ok_or_else(resource_limit_error)?;
            }
            b'{' | b'[' => {
                account_array_element(&mut containers, depth, &mut element_units)?;
                let is_map_entry =
                    depth != 0 && *byte == b'{' && containers[depth - 1].is_map_entries;
                in_number = false;
                tokens = tokens.checked_add(1).ok_or_else(resource_limit_error)?;
                depth = depth.checked_add(1).ok_or_else(resource_limit_error)?;
                if depth > MAX_CODEC_PROTO_JSON_NESTING_DEPTH {
                    return Err(resource_limit_error());
                }
                containers[depth - 1] = JsonContainer {
                    is_array: *byte == b'[',
                    is_map_entries: *byte == b'[' && pending_entries_array,
                    is_map_entry,
                    has_member: false,
                    expects_element: *byte == b'[',
                };
                pending_entries_array = false;
                last_string_is_entries = false;
            }
            b'}' | b']' => {
                in_number = false;
                depth = depth.checked_sub(1).ok_or_else(malformed_json_error)?;
                if *byte == b'}' && containers[depth].is_map_entry && !containers[depth].has_member
                {
                    // Empty map-entry objects allocate generated message
                    // wrappers but can never be a valid key/value pair.
                    return Err(malformed_json_error());
                }
                pending_entries_array = false;
                last_string_is_entries = false;
            }
            b':' => {
                in_number = false;
                pending_entries_array = last_string_is_entries;
                last_string_is_entries = false;
            }
            b',' => {
                in_number = false;
                pending_entries_array = false;
                last_string_is_entries = false;
                if depth != 0 && containers[depth - 1].is_array {
                    containers[depth - 1].expects_element = true;
                }
            }
            b't' | b'f' | b'n' => {
                account_array_element(&mut containers, depth, &mut element_units)?;
                in_number = false;
                pending_entries_array = false;
                last_string_is_entries = false;
                tokens = tokens.checked_add(1).ok_or_else(resource_limit_error)?;
            }
            b'-' | b'0'..=b'9' => {
                if !in_number {
                    account_array_element(&mut containers, depth, &mut element_units)?;
                    tokens = tokens.checked_add(1).ok_or_else(resource_limit_error)?;
                    in_number = true;
                }
                pending_entries_array = false;
                last_string_is_entries = false;
            }
            b'.' | b'e' | b'E' if in_number => return Err(malformed_json_error()),
            b' ' | b'\n' | b'\r' | b'\t' => in_number = false,
            _ => {
                in_number = false;
                pending_entries_array = false;
                last_string_is_entries = false;
            }
        }
        if tokens > MAX_CODEC_PROTO_JSON_TOKENS {
            return Err(resource_limit_error());
        }
    }

    if in_string || escaped || depth != 0 {
        return Err(malformed_json_error());
    }
    Ok(())
}

fn decode_protobuf_with_limit<M: Message>(bytes: &[u8], max_bytes: usize) -> CodecWireResult<M> {
    if bytes.len() > max_bytes {
        return Err(resource_limit_error());
    }

    DecodeOptions::new()
        .with_recursion_limit(CODEC_PROTO_RECURSION_LIMIT)
        .with_max_message_size(max_bytes)
        .with_element_memory_limit(MAX_CODEC_PROTO_ELEMENT_MEMORY_BYTES)
        .with_unknown_field_limit(CODEC_PROTO_UNKNOWN_FIELD_LIMIT)
        .decode_from_slice(bytes)
        .map_err(|error| match error {
            DecodeError::ElementMemoryLimitExceeded => resource_limit_error(),
            _ => malformed_protobuf_error(),
        })
}

#[cfg(test)]
mod tests {
    use super::{
        decode_json, decode_protobuf, decode_protobuf_with_limit, validate_json_nesting,
        MAX_CODEC_PROTO_ELEMENT_MEMORY_BYTES, MAX_CODEC_PROTO_JSON_BYTES,
        MAX_CODEC_PROTO_JSON_NESTING_DEPTH, MAX_CODEC_PROTO_JSON_TOKENS,
        MAX_CODEC_PROTO_SEMANTIC_NODES,
    };
    use crate::generated::proto::reallyme::codec::v1::{
        CodecDeterministicCborArray, CodecDeterministicCborValue, CodecErrorReason,
        CodecPemDecodeRequest,
    };

    #[test]
    fn binary_byte_cap_accepts_exact_size_and_rejects_next_byte() {
        let encoded = [0x0a, 0x01, b'a'];
        assert!(
            decode_protobuf_with_limit::<CodecPemDecodeRequest>(&encoded, encoded.len()).is_ok()
        );
        assert_eq!(
            decode_protobuf_with_limit::<CodecPemDecodeRequest>(&encoded, encoded.len() - 1)
                .err()
                .map(|error| error.reason()),
            Some(CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED)
        );
    }

    #[test]
    fn proto_json_byte_cap_accepts_exact_size_and_rejects_next_byte() {
        let mut exact = vec![b' '; MAX_CODEC_PROTO_JSON_BYTES - 2];
        exact.extend_from_slice(b"{}");
        assert!(decode_json::<CodecPemDecodeRequest>(&exact).is_ok());
        exact.push(b' ');
        assert_eq!(
            decode_json::<CodecPemDecodeRequest>(&exact)
                .err()
                .map(|error| error.reason()),
            Some(CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED)
        );
    }

    #[test]
    fn binary_element_memory_limit_rejects_repeated_empty_messages() {
        let element_size = core::mem::size_of::<CodecDeterministicCborValue>();
        assert!(element_size > 0);
        let count = (MAX_CODEC_PROTO_ELEMENT_MEMORY_BYTES / element_size) + 1;
        let mut encoded = Vec::with_capacity(count * 2);
        for _ in 0..count {
            encoded.extend_from_slice(&[0x0a, 0]);
        }
        assert_eq!(
            decode_protobuf::<CodecDeterministicCborArray>(&encoded)
                .err()
                .map(|error| error.reason()),
            Some(CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED)
        );
    }

    #[test]
    fn proto_json_nesting_accepts_exact_limit_and_rejects_one_more() {
        // Test the structural scanner directly so an unrelated generated
        // message shape or deserializer recursion limit cannot mask this cap.
        let mut exact = vec![b'['; MAX_CODEC_PROTO_JSON_NESTING_DEPTH];
        exact.push(b'0');
        exact.extend(core::iter::repeat_n(
            b']',
            MAX_CODEC_PROTO_JSON_NESTING_DEPTH,
        ));
        assert!(validate_json_nesting(&exact).is_ok());

        let mut over_limit = Vec::with_capacity(exact.len() + 2);
        over_limit.push(b'[');
        over_limit.extend_from_slice(&exact);
        over_limit.push(b']');
        assert_eq!(
            validate_json_nesting(&over_limit)
                .err()
                .map(|error| error.reason()),
            Some(CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED)
        );
    }

    #[test]
    #[allow(clippy::unwrap_used)]
    fn proto_json_token_budget_accepts_exact_limit_and_counts_literals() {
        // Repeated object fields exercise the token budget independently of
        // the array-element budget; the full parser rejects duplicates later.
        let scalar_count = MAX_CODEC_PROTO_JSON_TOKENS.checked_sub(4).unwrap() / 2;
        let capacity = scalar_count
            .checked_mul(6)
            .unwrap()
            .checked_add(16)
            .unwrap();
        let mut exact = Vec::with_capacity(capacity);
        exact.extend_from_slice(b"{\"x\":[0]");
        for _ in 0..scalar_count {
            exact.extend_from_slice(b",\"x\":0");
        }
        exact.push(b'}');
        assert!(validate_json_nesting(&exact).is_ok());

        // The next scalar is a literal so its accounting cannot be hidden by
        // the number-token branch used to fill the accepted boundary.
        exact.pop();
        exact.extend_from_slice(b",\"x\":true}");
        assert_eq!(
            validate_json_nesting(&exact)
                .err()
                .map(|error| error.reason()),
            Some(CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED)
        );
    }

    #[test]
    fn proto_json_pre_scan_charges_map_entries_before_deserialization() {
        let entries = MAX_CODEC_PROTO_SEMANTIC_NODES / 2;
        for property in ["entries", "en\\u0074ries"] {
            let mut exact = format!("{{\"{property}\":[").into_bytes();
            for index in 0..entries {
                if index != 0 {
                    exact.push(b',');
                }
                exact.extend_from_slice(b"{\"key\":0,\"value\":0}");
            }
            exact.extend_from_slice(b"]}");
            assert!(validate_json_nesting(&exact).is_ok());

            exact.truncate(exact.len() - 2);
            exact.extend_from_slice(b",{\"key\":0,\"value\":0}]}");
            assert_eq!(
                validate_json_nesting(&exact)
                    .err()
                    .map(|error| error.reason()),
                Some(CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED)
            );
        }
    }

    #[test]
    fn proto_json_pre_scan_rejects_empty_map_entries_before_allocation() {
        for property in ["entries", "en\\u0074ries"] {
            let input = format!("{{\"mapValue\":{{\"{property}\":[{{  }}]}}}}");
            assert_eq!(
                validate_json_nesting(input.as_bytes())
                    .err()
                    .map(|error| error.reason()),
                Some(CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MALFORMED_JSON)
            );
        }
    }

    #[test]
    fn proto_json_scanner_rejects_unclosed_structure_independently() {
        for incomplete in [b"[".as_slice(), b"\"".as_slice(), b"\"\\".as_slice()] {
            assert_eq!(
                validate_json_nesting(incomplete)
                    .err()
                    .map(|error| error.reason()),
                Some(CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_MALFORMED_JSON)
            );
        }
    }

    #[test]
    fn proto_json_scanner_leaves_non_number_punctuation_to_the_parser() {
        // This pre-scan only rejects fractional syntax after a number token.
        // The full JSON parser rejects a standalone period later.
        assert!(validate_json_nesting(b".").is_ok());
    }
}
