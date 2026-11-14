// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{
    encode_bool, encode_map, preflight_map, preflight_simple, sort_map_entries,
    try_vec_with_capacity, write_map_entries, DecoratedMapEntry, Preflight,
};
use crate::nested_map_proof_tests::TwoEntryMap;
use crate::{DeterministicCborMapEntry, DeterministicCborMapKey, DeterministicCborValue};
use core::mem::ManuallyDrop;
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

#[kani::proof]
#[kani::unwind(5)]
fn two_entry_inner_writer_preserves_the_canonical_wire() {
    let example = TwoEntryMap::symbolic();
    kani::assume(example.first < example.second);
    let entries = ManuallyDrop::new(two_entry_values(&example));
    let decorated = vec![
        DecoratedMapEntry {
            encoded_key: Zeroizing::new(vec![0x61, example.first]),
            entry: &entries[0],
        },
        DecoratedMapEntry {
            encoded_key: Zeroizing::new(vec![0x61, example.second]),
            entry: &entries[1],
        },
    ];
    let Some(mut output) = proof_output(TwoEntryMap::WIRE_LENGTH) else {
        return;
    };
    output.extend_from_slice(&[0xa1, 0x60]);
    assert!(
        write_map_entries(decorated, &mut output, 2, |value, output, depth| {
            let DeterministicCborValue::Bool(bit) = value else {
                kani::assert(false, "inner writer receives a boolean leaf");
                return Err(super::DeterministicCborError::PreflightLengthMismatch);
            };
            assert!(depth == 2);
            encode_bool(*bit, output)
        })
        .is_ok()
    );
    example.assert_wire(&output);
    kani::cover!(
        example.first == 0 && example.second == 0x7f,
        "boundary ASCII keys"
    );
}

#[kani::proof]
#[kani::unwind(5)]
fn outer_writer_preserves_the_two_entry_inner_suffix() {
    let example = TwoEntryMap::symbolic();
    kani::assume(example.first < example.second);
    let entries = ManuallyDrop::new([DeterministicCborMapEntry::new(
        DeterministicCborMapKey::text(String::new()),
        DeterministicCborValue::Map(Vec::from(two_entry_values(&example))),
    )]);
    let preflight = preflight_map(&*entries, 0, |value, depth| {
        assert!(core::ptr::eq(value, entries[0].value()) && depth == 1);
        // Two encoded keys, two boolean leaves and one map header account
        // for seven bytes. Each entry contributes two nodes and the map one.
        Ok(Preflight {
            encoded_len: 7,
            nodes: 5,
            aggregate_text_bytes: 2,
            aggregate_byte_string_bytes: 0,
        })
    });
    let Ok(preflight) = preflight else {
        kani::assert(false, "outer preflight succeeds");
        return;
    };
    assert!(preflight.encoded_len == TwoEntryMap::WIRE_LENGTH && preflight.nodes == 7);

    let Some(mut output) = proof_output(preflight.encoded_len) else {
        return;
    };
    assert!(
        encode_map(&*entries, &mut output, 0, |value, output, depth| {
            assert!(core::ptr::eq(value, entries[0].value()) && depth == 1);
            assert!(output.capacity() == TwoEntryMap::WIRE_LENGTH);
            assert!(matches!(output.as_slice(), [0xa1, 0x60]));
            output.extend_from_slice(&example.wire()[2..]);
            Ok(())
        })
        .is_ok()
    );
    example.assert_wire(&output);
    kani::cover!(
        example.first == 0 && example.second == 0x7f,
        "boundary ASCII keys"
    );
}

fn two_entry_values(example: &TwoEntryMap) -> [DeterministicCborMapEntry; 2] {
    [
        DeterministicCborMapEntry::new(
            DeterministicCborMapKey::text(String::from(char::from(example.first))),
            DeterministicCborValue::Bool(false),
        ),
        DeterministicCborMapEntry::new(
            DeterministicCborMapKey::text(String::from(char::from(example.second))),
            DeterministicCborValue::Bool(true),
        ),
    ]
}

#[kani::proof]
#[kani::unwind(4)]
fn one_byte_text_key_preflights_and_encodes_exact_bytes() {
    let example = TwoEntryMap::symbolic();
    let key = ManuallyDrop::new(DeterministicCborMapKey::text(String::from(char::from(
        example.first,
    ))));
    let stats = super::preflight_key(&key);
    assert!(
        matches!(stats, Ok(stats) if stats.encoded_len == 2 && stats.nodes == 1
        && stats.aggregate_text_bytes == 1 && stats.aggregate_byte_string_bytes == 0)
    );
    let Some(mut output) = proof_output(2) else {
        return;
    };
    assert!(super::encode_key(&key, &mut output).is_ok());
    assert!(matches!(output.as_slice(), [0x61, byte] if *byte == example.first));
    kani::cover!(example.first == 0x7f, "highest single ASCII byte");
}

#[kani::proof]
#[kani::unwind(4)]
fn two_prepared_keys_preserve_canonical_order_when_sorted() {
    let example = TwoEntryMap::symbolic();
    kani::assume(example.first < example.second);
    let entries = ManuallyDrop::new(two_entry_values(&example));
    let mut decorated = [
        DecoratedMapEntry {
            encoded_key: Zeroizing::new(vec![0x61, example.first]),
            entry: &entries[0],
        },
        DecoratedMapEntry {
            encoded_key: Zeroizing::new(vec![0x61, example.second]),
            entry: &entries[1],
        },
    ];
    assert!(sort_map_entries(&mut decorated).is_ok());
    assert!(core::ptr::eq(decorated[0].entry, &entries[0]));
    assert!(core::ptr::eq(decorated[1].entry, &entries[1]));
    assert!(matches!(decorated[0].encoded_key.as_slice(), [0x61, key] if *key == example.first));
    assert!(matches!(decorated[1].encoded_key.as_slice(), [0x61, key] if *key == example.second));
    kani::cover!(
        example.first == 0 && example.second == 0x7f,
        "boundary ASCII keys"
    );
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
