// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Native generated cleanup must erase retired allocations, including failures.

#![cfg(feature = "generated")]
#![allow(unsafe_code, reason = "inspect initialized synthetic test allocations")]
#![allow(clippy::unwrap_used, reason = "fixed synthetic regression vectors")]

#[path = "support/allocator.rs"]
mod allocator;

use buffa::bytes::Buf;
use buffa::{DecodeOptions, MessageView};
use reallyme_codec_proto::generated::proto::reallyme::codec::v1 as proto;
use reallyme_codec_proto::guarded::{SecretBytes, SecretString};
use reallyme_codec_proto::{decode_json, decode_protobuf};
use serde::de::value::{Error as ValueError, StringDeserializer};
use serde::Deserialize;

#[global_allocator]
static ALLOCATOR: allocator::Probe = allocator::Probe;

const PAYLOAD_LENGTH: usize = 64;
const PAYLOAD_BYTE: u8 = 0xa5;
const DER_FIELD: u8 = 0x12;

#[test]
fn standalone_enum_json_wipes_names_transferred_by_serde() {
    for (name, valid) in [
        ("CODEC_TLV_PROFILE_ISO7816_DEFINITE_BER", true),
        ("synthetic-private-enum-name", false),
    ] {
        let mut token = String::with_capacity(PAYLOAD_LENGTH);
        // Initialize the spare allocation too so the probe can verify it.
        token.extend(std::iter::repeat_n('x', PAYLOAD_LENGTH));
        token.clear();
        token.push_str(name);
        allocator::watch(token.as_ptr().cast_mut(), PAYLOAD_LENGTH);
        let result =
            proto::CodecTlvProfile::deserialize(StringDeserializer::<ValueError>::new(token));
        allocator::assert_erased(None);
        assert_eq!(result.is_ok(), valid);
    }
}

fn wire_bytes(field: u8) -> Vec<u8> {
    let mut wire = vec![field, 64];
    wire.extend([PAYLOAD_BYTE; PAYLOAD_LENGTH]);
    wire
}

#[test]
fn storage_drop_and_clear_erase_live_and_initialized_spare_bytes() {
    let mut allocation = vec![PAYLOAD_BYTE; PAYLOAD_LENGTH];
    allocation.truncate(16);
    let mut bytes = SecretBytes::from(allocation);
    let pointer = bytes.as_ptr();
    bytes.clear();
    assert!(bytes.is_empty());
    for offset in 0..PAYLOAD_LENGTH {
        // SAFETY: The allocation was completely initialized before truncation,
        // remains owned by bytes, and clear preserves its capacity.
        assert_eq!(unsafe { pointer.add(offset).read_volatile() }, 0);
    }
    allocator::watch(pointer.cast_mut(), PAYLOAD_LENGTH);
    drop(bytes);
    allocator::assert_erased(None);

    let mut allocation = String::from_utf8(vec![b'S'; PAYLOAD_LENGTH]).unwrap();
    allocation.truncate(16);
    let text = SecretString::from(allocation);
    allocator::watch(text.as_ptr().cast_mut(), PAYLOAD_LENGTH);
    drop(text);
    allocator::assert_erased(None);
}

#[test]
fn clones_have_independent_wipe_owners() {
    let original = SecretBytes::from(vec![PAYLOAD_BYTE; PAYLOAD_LENGTH]);
    let copy = original.clone();
    assert_ne!(original.as_ptr(), copy.as_ptr());
    allocator::watch(copy.as_ptr().cast_mut(), PAYLOAD_LENGTH);
    drop(copy);
    allocator::assert_erased(None);
    assert_eq!(original, [PAYLOAD_BYTE; PAYLOAD_LENGTH]);

    let original = SecretString::from("synthetic private text");
    let copy = original.clone();
    assert_ne!(original.as_ptr(), copy.as_ptr());
    allocator::watch(copy.as_ptr().cast_mut(), copy.len());
    drop(copy);
    allocator::assert_erased(None);
    assert_eq!(original, "synthetic private text");
}

#[test]
fn every_fragment_boundary_decodes_directly_into_one_guarded_byte_owner() {
    let wire = wire_bytes(DER_FIELD);
    for split in 0..=wire.len() {
        let mut fragments = wire[..split].chain(&wire[split..]);
        allocator::arm(PAYLOAD_LENGTH, 1, PAYLOAD_LENGTH);
        let decoded = DecodeOptions::new()
            .reject_unknown_fields()
            .decode::<proto::CodecPemEncodeRequest>(&mut fragments)
            .unwrap();
        assert_eq!(decoded.der, [PAYLOAD_BYTE; PAYLOAD_LENGTH]);
        assert_eq!(fragments.remaining(), 0);
        drop(decoded);
        allocator::assert_erased(Some(1));
    }
}

#[test]
fn fragmented_strings_and_invalid_utf8_leave_no_unowned_scratch() {
    for valid in [true, false] {
        let mut wire = wire_bytes(0x0a);
        if valid {
            wire[2..].fill(b'S');
        }
        for split in 0..=wire.len() {
            let mut fragments = wire[..split].chain(&wire[split..]);
            allocator::arm(PAYLOAD_LENGTH, 1, PAYLOAD_LENGTH);
            let decoded = DecodeOptions::new()
                .reject_unknown_fields()
                .decode::<proto::CodecTextRequest>(&mut fragments);
            if valid {
                let decoded = decoded.unwrap();
                assert_eq!(decoded.value.len(), PAYLOAD_LENGTH);
                assert!(decoded.value.bytes().all(|byte| byte == b'S'));
                drop(decoded);
            } else {
                assert!(decoded.is_err());
            }
            allocator::assert_erased(Some(1));
        }
    }
}

#[test]
fn eager_view_to_owned_creates_a_guarded_copy_and_preserves_the_source_owner() {
    let wire = zeroize::Zeroizing::new(wire_bytes(DER_FIELD));
    let view = DecodeOptions::new()
        .reject_unknown_fields()
        .decode_view::<proto::CodecPemEncodeRequestView<'_>>(&wire)
        .unwrap();
    allocator::arm(PAYLOAD_LENGTH, 1, PAYLOAD_LENGTH);
    let owned = view.to_owned_message().unwrap();
    assert_eq!(owned.der, [PAYLOAD_BYTE; PAYLOAD_LENGTH]);
    assert_ne!(owned.der.as_ptr(), view.der.as_ptr());
    drop(owned);
    allocator::assert_erased(Some(1));
    assert_eq!(&wire[2..], &[PAYLOAD_BYTE; PAYLOAD_LENGTH]);
}

#[test]
fn oneof_replacement_erases_the_previous_nested_owner() {
    // bytes_value (field 5) contains bytes (field 1). A later null_value
    // changes the oneof and must retire the complete earlier message tree.
    let nested = wire_bytes(0x0a);
    let mut wire = vec![0x2a, 66];
    wire.extend(nested);
    wire.extend([0x0a, 0]);
    allocator::arm(PAYLOAD_LENGTH, 1, PAYLOAD_LENGTH);
    let decoded = decode_protobuf::<proto::CodecDeterministicCborValue>(&wire).unwrap();
    assert!(matches!(
        decoded.value.as_ref(),
        Some(proto::codec_deterministic_cbor_value::Value::NullValue(_))
    ));
    allocator::assert_erased(Some(1));
}

#[test]
fn malformed_later_binary_field_erases_the_earlier_owner() {
    let mut wire = wire_bytes(DER_FIELD);
    wire.extend([0x1a, 4, 0]); // truncated options message
    allocator::arm(PAYLOAD_LENGTH, 1, PAYLOAD_LENGTH);
    assert!(decode_protobuf::<proto::CodecPemEncodeRequest>(&wire).is_err());
    allocator::assert_erased(Some(1));
}

#[test]
fn json_unknown_and_duplicate_failures_erase_decoded_bytes_and_spare_capacity() {
    // 64 bytes encode to 88 base64 characters. The bounded decoder initializes
    // all 66 output bytes before decoding and then truncates to 64.
    let encoded = base64::Engine::encode(
        &base64::engine::general_purpose::STANDARD,
        [PAYLOAD_BYTE; PAYLOAD_LENGTH],
    );
    for tail in [r#", "unexpected":true}"#, r#", "der":"AA=="}"#] {
        let json = format!(r#"{{"der":"{encoded}"{tail}"#);
        allocator::arm(66, 1, 66);
        assert!(decode_json::<proto::CodecPemEncodeRequest>(json.as_bytes()).is_err());
        allocator::assert_erased(Some(1));
    }
}

#[test]
fn late_json_failure_erases_an_already_parsed_inline_number() {
    let layout = std::alloc::Layout::new::<proto::CodecGeneralCborUnsigned>();
    // This particular generated message contains exactly its initialized u64;
    // do not extend this observation to messages with padding.
    assert_eq!(layout.size(), std::mem::size_of::<u64>());
    allocator::arm(layout.size(), layout.align(), layout.size());
    let result = decode_json::<proto::CodecGeneralCborValue>(
        br#"{"unsignedValue":{"argument":"11936128518282651045"},"unexpected":true}"#,
    );
    assert!(result.is_err());
    allocator::assert_erased(Some(1));
}
