// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use buffa::{DecodeOptions, Message};
use serde::de::DeserializeOwned;
use zeroize::Zeroizing;

use crate::error::{
    malformed_json_error, malformed_protobuf_error, resource_limit_error, CodecWireResult,
};
use crate::limits::{
    CODEC_PROTO_RECURSION_LIMIT, CODEC_PROTO_UNKNOWN_FIELD_LIMIT, MAX_CODEC_PROTO_JSON_BYTES,
    MAX_CODEC_PROTO_JSON_NESTING_DEPTH, MAX_CODEC_PROTO_MESSAGE_BYTES,
};

/// Encodes a protobuf message with Buffa.
#[must_use]
pub fn encode_protobuf<M: Message>(message: &M) -> Zeroizing<Vec<u8>> {
    Zeroizing::new(message.encode_to_vec())
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
    let encoded = encode_protobuf(&message);
    if encoded.len() > MAX_CODEC_PROTO_MESSAGE_BYTES {
        return Err(resource_limit_error());
    }
    Ok(message)
}

fn validate_json_nesting(bytes: &[u8]) -> CodecWireResult<()> {
    let mut depth = 0_usize;
    let mut in_string = false;
    let mut escaped = false;
    for byte in bytes {
        if in_string {
            if escaped {
                escaped = false;
            } else if *byte == b'\\' {
                escaped = true;
            } else if *byte == b'"' {
                in_string = false;
            }
            continue;
        }

        match *byte {
            b'"' => in_string = true,
            b'{' | b'[' => {
                depth = depth.checked_add(1).ok_or_else(resource_limit_error)?;
                if depth > MAX_CODEC_PROTO_JSON_NESTING_DEPTH {
                    return Err(resource_limit_error());
                }
            }
            b'}' | b']' => {
                depth = depth.checked_sub(1).ok_or_else(malformed_json_error)?;
            }
            _ => {}
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
        .with_unknown_field_limit(CODEC_PROTO_UNKNOWN_FIELD_LIMIT)
        .decode_from_slice(bytes)
        .map_err(|_| malformed_protobuf_error())
}
