// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{
    collect_map_entries, encode_bool, encode_map, encode_map_key, preflight_map, preflight_simple,
    sort_map_entries, try_vec_with_capacity, write_map_entries, DecoratedMapEntry, Preflight,
};
use crate::nested_map_proof_tests::{
    assert_mixed_length_wire, TwoEntryMap, MIXED_LENGTH_CANONICAL,
};
use crate::{DeterministicCborMapEntry, DeterministicCborMapKey, DeterministicCborValue};
use core::mem::ManuallyDrop;
use zeroize::Zeroizing;

#[kani::proof]
#[kani::unwind(4)]
fn mixed_length_key_writer_emits_preferred_headers_and_payloads() {
    verify_mixed_length_key_writer("z", b"\x61z");
    kani::cover!(true, "one-byte text key encoded");
}

#[kani::proof]
#[kani::unwind(4)]
fn mixed_length_long_key_writer_emits_preferred_header_and_payload() {
    verify_mixed_length_key_writer("aa", b"\x62aa");
    kani::cover!(true, "two-byte text key encoded");
}

fn verify_mixed_length_key_writer(text: &str, expected: &[u8]) {
    let key = ManuallyDrop::new(DeterministicCborMapKey::text(String::from(text)));
    let Some(mut output) = proof_output(expected.len()) else {
        return;
    };
    assert!(super::encode_key(&key, &mut output).is_ok());
    assert!(output.as_slice() == expected);
}

#[kani::proof]
#[kani::unwind(4)]
fn mixed_length_preflight_reserves_each_key_width() {
    let long: bool = kani::any();
    let text = if long { "aa" } else { "z" };
    let expected = mixed_length_key_bytes(long);
    let key = ManuallyDrop::new(DeterministicCborMapKey::text(String::from(text)));
    let encoded = ManuallyDrop::new(encode_map_key(&key, |actual, output| {
        assert!(core::ptr::eq(actual, &*key));
        assert!(output.is_empty() && output.capacity() == expected.len());
        // The separate key-writer check establishes this output.
        output.extend_from_slice(expected);
        Ok(())
    }));
    assert!(matches!(&*encoded, Ok(bytes) if bytes.as_slice() == expected));
    kani::cover!(long && encoded.is_ok(), "three-byte encoded key reserved");
    kani::cover!(!long && encoded.is_ok(), "two-byte encoded key reserved");
}

#[kani::proof]
#[kani::unwind(4)]
fn mixed_length_preparation_pairs_both_encoded_keys_with_their_values() {
    let long_first: bool = kani::any();
    let entries = ManuallyDrop::new(mixed_length_entries(long_first));
    let mut visited = 0;
    let prepared = ManuallyDrop::new(collect_map_entries(&*entries, |key| {
        assert!(visited < entries.len() && core::ptr::eq(key, entries[visited].key()));
        let long = (visited == 0) == long_first;
        visited += 1;
        // The key-writer check establishes these bytes for this entry.
        Ok(Zeroizing::new(mixed_length_key_bytes(long).to_vec()))
    }));
    let Ok(prepared) = &*prepared else {
        kani::assert(false, "both map entries are prepared");
        return;
    };
    assert!(visited == 2);
    assert_mixed_length_pair(prepared, &entries, long_first);
    kani::cover!(long_first, "long key prepared first");
    kani::cover!(!long_first, "short key prepared first");
}

#[kani::proof]
#[kani::unwind(4)]
fn mixed_length_sort_moves_the_whole_short_entry_first() {
    let long_first: bool = kani::any();
    let entries = ManuallyDrop::new(mixed_length_entries(long_first));
    let mut prepared = mixed_length_prepared(&entries, long_first);
    assert!(sort_map_entries(&mut prepared).is_ok());
    assert!(core::ptr::eq(
        prepared[0].entry,
        &entries[usize::from(long_first)]
    ));
    assert!(core::ptr::eq(
        prepared[1].entry,
        &entries[usize::from(!long_first)]
    ));
    assert!(prepared[0].encoded_key.as_slice() == b"\x61z");
    assert!(prepared[1].encoded_key.as_slice() == b"\x62aa");
    kani::cover!(
        long_first,
        "sorting moves the short entry ahead of the long entry"
    );
    kani::cover!(!long_first, "sorting preserves canonical order");
}

#[kani::proof]
#[kani::unwind(5)]
fn mixed_length_inner_writer_reproduces_every_byte_from_sorted_entries() {
    let entries = ManuallyDrop::new(mixed_length_entries(false));
    let prepared = Vec::from(mixed_length_prepared(&entries, false));
    let Some(mut output) = proof_output(MIXED_LENGTH_CANONICAL.len()) else {
        return;
    };
    output.extend_from_slice(&[0xa1, 0x60]);
    assert!(
        write_map_entries(prepared, &mut output, 2, |child, output, depth| {
            let DeterministicCborValue::Bool(bit) = child else {
                kani::assert(false, "inner writer receives a boolean");
                return Err(super::DeterministicCborError::PreflightLengthMismatch);
            };
            assert!(depth == 2);
            encode_bool(*bit, output)
        })
        .is_ok()
    );
    assert_mixed_length_wire(&output);
    kani::cover!(output.len() == 10, "complete nested wire reproduced");
}

#[kani::proof]
#[kani::unwind(4)]
fn mixed_length_outer_writer_preserves_the_canonical_inner_suffix() {
    let entries = ManuallyDrop::new([DeterministicCborMapEntry::new(
        DeterministicCborMapKey::text(String::new()),
        DeterministicCborValue::Map(Vec::from(mixed_length_entries(false))),
    )]);
    let preflight = preflight_map(&*entries, 0, |child, depth| {
        assert!(core::ptr::eq(child, entries[0].value()) && depth == 1);
        // One map header, key widths of two and three, and two boolean leaves.
        Ok(Preflight {
            encoded_len: 8,
            nodes: 5,
            aggregate_text_bytes: 3,
            aggregate_byte_string_bytes: 0,
        })
    });
    let Ok(stats) = preflight else {
        kani::assert(false, "outer map preflights");
        return;
    };
    assert!(stats.encoded_len == 10 && stats.nodes == 7);
    let Some(mut output) = proof_output(stats.encoded_len) else {
        return;
    };
    assert!(
        encode_map(&*entries, &mut output, 0, |child, output, depth| {
            assert!(core::ptr::eq(child, entries[0].value()) && depth == 1);
            assert!(output.capacity() == 10 && output.as_slice() == [0xa1, 0x60]);
            // The inner writer check establishes the suffix for these entries.
            output.extend_from_slice(&MIXED_LENGTH_CANONICAL[2..]);
            Ok(())
        })
        .is_ok()
    );
    assert_mixed_length_wire(&output);
    kani::cover!(
        output.len() == 10,
        "outer map preserves complete nested wire"
    );
}

fn mixed_length_entries(long_first: bool) -> [DeterministicCborMapEntry; 2] {
    let short = DeterministicCborMapEntry::new(
        DeterministicCborMapKey::text(String::from("z")),
        DeterministicCborValue::Bool(false),
    );
    let long = DeterministicCborMapEntry::new(
        DeterministicCborMapKey::text(String::from("aa")),
        DeterministicCborValue::Bool(true),
    );
    if long_first {
        [long, short]
    } else {
        [short, long]
    }
}

fn mixed_length_key_bytes(long: bool) -> &'static [u8] {
    if long {
        b"\x62aa"
    } else {
        b"\x61z"
    }
}

fn mixed_length_prepared(
    entries: &[DeterministicCborMapEntry; 2],
    long_first: bool,
) -> [DecoratedMapEntry<'_>; 2] {
    [
        DecoratedMapEntry {
            encoded_key: Zeroizing::new(mixed_length_key_bytes(long_first).to_vec()),
            entry: &entries[0],
        },
        DecoratedMapEntry {
            encoded_key: Zeroizing::new(mixed_length_key_bytes(!long_first).to_vec()),
            entry: &entries[1],
        },
    ]
}

fn assert_mixed_length_pair(
    prepared: &[DecoratedMapEntry<'_>],
    entries: &[DeterministicCborMapEntry; 2],
    long_first: bool,
) {
    let [first, second] = prepared else {
        kani::assert(false, "prepared map retains both entries");
        return;
    };
    assert!(core::ptr::eq(first.entry, &entries[0]));
    assert!(core::ptr::eq(second.entry, &entries[1]));
    assert!(first.encoded_key.as_slice() == mixed_length_key_bytes(long_first));
    assert!(second.encoded_key.as_slice() == mixed_length_key_bytes(!long_first));
}

#[kani::proof]
#[kani::unwind(4)]
fn preparation_encodes_the_exact_key_bytes() {
    let example = TwoEntryMap::symbolic();
    let key = ManuallyDrop::new(DeterministicCborMapKey::text(String::from(char::from(
        example.first,
    ))));
    let result = ManuallyDrop::new(encode_map_key(&key, |actual, output| {
        // The existing key-encoding check establishes the bytes written.
        assert!(core::ptr::eq(actual, &*key));
        assert!(output.is_empty() && output.capacity() == 2);
        output.extend_from_slice(&[0x61, example.first]);
        Ok(())
    }));
    let Ok(bytes) = &*result else {
        kani::assert(false, "single map key prepares successfully");
        return;
    };
    assert!(matches!(bytes.as_slice(), [0x61, key] if *key == example.first));
    kani::cover!(example.first == 0, "lowest ASCII key prepares");
    kani::cover!(example.first == 0x7f, "highest ASCII key prepares");
}

#[kani::proof]
#[kani::unwind(4)]
fn preparation_collects_both_entries_in_source_order() {
    let example = TwoEntryMap::symbolic();
    let entries = ManuallyDrop::new(two_entry_values(&example));
    let mut calls = 0;
    let result = collect_map_entries(&*entries, |key| {
        // The key-preparation proof establishes the returned bytes. The loop
        // must pair those bytes with the entry whose key was passed in.
        assert!(calls < entries.len() && core::ptr::eq(key, entries[calls].key()));
        let byte = if calls == 0 {
            example.first
        } else {
            example.second
        };
        calls += 1;
        Ok(Zeroizing::new(vec![0x61, byte]))
    });
    let result = ManuallyDrop::new(result);
    let Ok(decorated) = &*result else {
        kani::assert(false, "both prepared entries are collected");
        return;
    };
    assert!(calls == 2);
    assert_prepared_pair(decorated, &entries, &example);
    kani::cover!(
        example.first == example.second,
        "duplicate keys reach sorting"
    );
    kani::cover!(
        example.first < example.second,
        "increasing keys reach sorting"
    );
    kani::cover!(
        example.first > example.second,
        "descending keys reach sorting"
    );
}

fn assert_prepared_pair(
    decorated: &[DecoratedMapEntry<'_>],
    entries: &[DeterministicCborMapEntry; 2],
    example: &TwoEntryMap,
) {
    let [first, second] = decorated else {
        kani::assert(false, "preparation retains both entries");
        return;
    };
    assert!(core::ptr::eq(first.entry, &entries[0]));
    assert!(core::ptr::eq(second.entry, &entries[1]));
    assert!(matches!(first.encoded_key.as_slice(), [0x61, key] if *key == example.first));
    assert!(matches!(second.encoded_key.as_slice(), [0x61, key] if *key == example.second));
}

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
