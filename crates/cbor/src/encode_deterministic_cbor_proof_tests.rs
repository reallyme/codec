// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{
    encode_bool, encode_map, preflight_map, preflight_simple, try_vec_with_capacity, Preflight,
};
use crate::{DeterministicCborMapEntry, DeterministicCborMapKey, DeterministicCborValue};
use zeroize::Zeroizing;

#[kani::proof]
#[kani::unwind(4)]
fn outer_layer_passes_inner_map_to_child_after_its_prefix() {
    let bit: bool = kani::any();
    let inner = DeterministicCborValue::Map(vec![DeterministicCborMapEntry::new(
        DeterministicCborMapKey::text(String::new()),
        DeterministicCborValue::Bool(bit),
    )]);
    let entries = core::mem::ManuallyDrop::new([DeterministicCborMapEntry::new(
        DeterministicCborMapKey::text(String::new()),
        inner,
    )]);
    let preflight = preflight_map(&*entries, 0, |value, depth| {
        assert!(core::ptr::eq(value, entries[0].value()) && depth == 1);
        Ok(Preflight {
            encoded_len: 3,
            nodes: 3,
            ..Preflight::default()
        })
    });
    let Ok(preflight) = preflight else {
        kani::assert(false, "outer preflight succeeds");
        return;
    };
    assert!(preflight.encoded_len == 5 && preflight.nodes == 5);

    let Some(mut output) = proof_output(preflight.encoded_len) else {
        return;
    };
    assert!(
        encode_map(&*entries, &mut output, 0, |value, output, depth| {
            assert!(core::ptr::eq(value, entries[0].value()) && depth == 1);
            assert!(output.len() == 2 && output.capacity() == 5);
            assert!(output[0] == 0xa1 && output[1] == 0x60);
            output.extend_from_slice(&[0xa1, 0x60, 0xf4 | u8::from(bit)]);
            Ok(())
        })
        .is_ok()
    );
    // The inner writer check establishes this suffix. The outer writer must
    // leave the complete result in place after the callback returns.
    assert_nested_wire(&output, bit);
    kani::cover!(bit, "outer map with a true leaf in its child");
    kani::cover!(!bit, "outer map with a false leaf in its child");
}

#[kani::proof]
#[kani::unwind(4)]
fn single_map_layer_writes_header_key_and_child() {
    let bit: bool = kani::any();
    let entries = [DeterministicCborMapEntry::new(
        DeterministicCborMapKey::text(String::new()),
        DeterministicCborValue::Bool(bit),
    )];
    let stats = preflight_map(&entries, 1, |value, depth| {
        assert!(matches!(value, DeterministicCborValue::Bool(actual) if *actual == bit));
        assert!(depth == 2);
        preflight_simple()
    });
    assert!(matches!(
        stats,
        Ok(stats) if stats.encoded_len == 3 && stats.nodes == 3
            && stats.aggregate_text_bytes == 0 && stats.aggregate_byte_string_bytes == 0
    ));
    // The inner writer starts after the outer map header and key.
    let Some(mut output) = proof_output(5) else {
        return;
    };
    output.extend_from_slice(&[0xa1, 0x60]);
    assert!(
        encode_map(&entries, &mut output, 1, |value, output, depth| {
            let DeterministicCborValue::Bool(actual) = value else {
                kani::assert(false, "encoder receives the boolean leaf");
                return Err(super::DeterministicCborError::PreflightLengthMismatch);
            };
            assert!(depth == 2);
            encode_bool(*actual, output)
        })
        .is_ok()
    );
    assert_nested_wire(&output, bit);
    kani::cover!(bit, "true leaf written after key");
    kani::cover!(!bit, "false leaf written after key");
}

fn proof_output(capacity: usize) -> Option<core::mem::ManuallyDrop<Zeroizing<Vec<u8>>>> {
    let Ok(output) = try_vec_with_capacity(capacity) else {
        kani::assert(false, "bounded output reservation succeeds in the verifier");
        return None;
    };
    // The proof observes returned bytes. The ordinary owner is tested
    // separately for erasure when dropped.
    Some(core::mem::ManuallyDrop::new(Zeroizing::new(output)))
}

fn assert_nested_wire(output: &[u8], bit: bool) {
    assert!(output.len() == 5);
    assert!(output[0] == 0xa1 && output[1] == 0x60 && output[2] == 0xa1 && output[3] == 0x60);
    assert!(output[4] == (0xf4 | u8::from(bit)));
}
