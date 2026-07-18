// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use codec_cbor::{
    decode_deterministic_cbor, encode_deterministic_cbor, DeterministicCborError,
    DeterministicCborInteger, DeterministicCborMapEntry, DeterministicCborMapKey,
    DeterministicCborValue, MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES,
    MAX_DETERMINISTIC_CBOR_INPUT_LEN, MAX_DETERMINISTIC_CBOR_NESTING_DEPTH,
};
use std::sync::{Arc, Barrier};

const PROPERTY_CASES: u64 = 512;
const PROPERTY_MAX_DEPTH: usize = 5;
const CONCURRENT_WORKERS: u64 = 8;
const CONCURRENT_CASES_PER_WORKER: u64 = 128;

/// Small deterministic generator used for semantic property tests.
///
/// Keeping this generator local avoids adding a release dependency solely for
/// tests while still making every failing case reproducible from its seed.
struct PropertyGenerator {
    state: u64,
}

impl PropertyGenerator {
    fn new(seed: u64) -> Self {
        // Xorshift has an absorbing all-zero state. Mixing the public test seed
        // with a fixed nonzero value keeps the generator useful for seed zero.
        Self {
            state: seed ^ 0x9e37_79b9_7f4a_7c15,
        }
    }

    fn next_u64(&mut self) -> u64 {
        let mut value = self.state;
        value ^= value << 13;
        value ^= value >> 7;
        value ^= value << 17;
        self.state = value;
        value
    }

    fn bounded_usize(&mut self, exclusive_upper_bound: usize) -> usize {
        let bound = u64::try_from(exclusive_upper_bound).unwrap();
        usize::try_from(self.next_u64() % bound).unwrap()
    }
}

fn generated_text(generator: &mut PropertyGenerator) -> String {
    let length = generator.bounded_usize(24);
    let mut value = String::with_capacity(length);
    for _ in 0..length {
        let offset = u8::try_from(generator.next_u64() % 26).unwrap();
        value.push(char::from(b'a' + offset));
    }
    value
}

fn generated_bytes(generator: &mut PropertyGenerator) -> Vec<u8> {
    let length = generator.bounded_usize(32);
    (0..length)
        .map(|_| u8::try_from(generator.next_u64() & 0xff).unwrap())
        .collect()
}

fn generated_value(
    generator: &mut PropertyGenerator,
    remaining_depth: usize,
) -> DeterministicCborValue {
    let variant_count = if remaining_depth == 0 { 6 } else { 8 };
    let variant = generator.bounded_usize(variant_count);
    assert!(variant < variant_count);
    match variant {
        0 => DeterministicCborValue::Null,
        1 => DeterministicCborValue::Bool(generator.next_u64() & 1 == 1),
        2 => DeterministicCborValue::Integer(DeterministicCborInteger::unsigned(
            generator.next_u64(),
        )),
        3 => {
            let offset = i64::try_from(generator.next_u64() & 0x7fff_ffff_ffff_ffff).unwrap();
            let negative = i64::MIN.saturating_add(offset);
            DeterministicCborValue::Integer(DeterministicCborInteger::negative(negative).unwrap())
        }
        4 => DeterministicCborValue::Text(generated_text(generator)),
        5 => DeterministicCborValue::Bytes(generated_bytes(generator)),
        6 => {
            let child_count = generator.bounded_usize(5);
            DeterministicCborValue::Array(
                (0..child_count)
                    .map(|_| generated_value(generator, remaining_depth - 1))
                    .collect(),
            )
        }
        7 => {
            let entry_count = generator.bounded_usize(5);
            let key_prefix = generator.next_u64() & 0xffff_ffff_ffff_0000;
            DeterministicCborValue::Map(
                (0..entry_count)
                    .map(|index| {
                        let key_suffix = u64::try_from(index).unwrap();
                        DeterministicCborMapEntry::new(
                            DeterministicCborMapKey::Integer(DeterministicCborInteger::unsigned(
                                key_prefix | key_suffix,
                            )),
                            generated_value(generator, remaining_depth - 1),
                        )
                    })
                    .collect(),
            )
        }
        // Keep the test generator total without exempting it from the
        // repository-wide ban on panic-style unreachable paths. The explicit
        // range assertion above makes this fallback independently auditable.
        _ => DeterministicCborValue::Null,
    }
}

fn generated_map(seed: u64, reverse: bool) -> DeterministicCborValue {
    let mut generator = PropertyGenerator::new(seed);
    let entry_count = 2 + generator.bounded_usize(7);
    let mut entries: Vec<_> = (0..entry_count)
        .map(|index| {
            let key = if index & 1 == 0 {
                DeterministicCborMapKey::Integer(DeterministicCborInteger::unsigned(
                    u64::try_from(index).unwrap(),
                ))
            } else {
                DeterministicCborMapKey::text(format!("key-{index}"))
            };
            DeterministicCborMapEntry::new(
                key,
                generated_value(&mut generator, PROPERTY_MAX_DEPTH - 1),
            )
        })
        .collect();
    if reverse {
        entries.reverse();
    }
    DeterministicCborValue::Map(entries)
}

#[test]
fn generic_cbor_encodes_integer_boundaries() {
    let unsigned_max =
        DeterministicCborValue::Integer(DeterministicCborInteger::unsigned(u64::MAX));
    assert_eq!(
        encode_deterministic_cbor(&unsigned_max).unwrap().as_slice(),
        &[0x1b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
    );

    let negative_min =
        DeterministicCborValue::Integer(DeterministicCborInteger::negative(i64::MIN).unwrap());
    assert_eq!(
        encode_deterministic_cbor(&negative_min).unwrap().as_slice(),
        &[0x3b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
    );
}

#[test]
fn generic_cbor_sorts_mixed_integer_and_text_map_keys() {
    let value = DeterministicCborValue::Map(vec![
        DeterministicCborMapEntry::new(
            DeterministicCborMapKey::text("a".to_owned()),
            DeterministicCborValue::Integer(DeterministicCborInteger::unsigned(1)),
        ),
        DeterministicCborMapEntry::new(
            DeterministicCborMapKey::Integer(DeterministicCborInteger::unsigned(24)),
            DeterministicCborValue::Null,
        ),
        DeterministicCborMapEntry::new(
            DeterministicCborMapKey::Integer(DeterministicCborInteger::unsigned(0)),
            DeterministicCborValue::Text("zero".to_owned()),
        ),
    ]);

    let encoded = encode_deterministic_cbor(&value).unwrap();
    assert_eq!(
        encoded.as_slice(),
        &[0xa3, 0x00, 0x64, b'z', b'e', b'r', b'o', 0x18, 0x18, 0xf6, 0x61, b'a', 0x01,]
    );

    let decoded = decode_deterministic_cbor(&encoded).unwrap();
    assert_eq!(
        encode_deterministic_cbor(&decoded).unwrap().as_slice(),
        encoded.as_slice()
    );
}

#[test]
fn generic_cbor_decodes_full_unsigned_range() {
    let encoded = [0x1b, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff];
    let decoded = decode_deterministic_cbor(&encoded).unwrap();

    assert_eq!(
        encode_deterministic_cbor(&decoded).unwrap().as_slice(),
        encoded
    );
}

#[test]
fn generic_cbor_rejects_duplicate_keys_on_encode_and_decode() {
    let duplicate = DeterministicCborValue::Map(vec![
        DeterministicCborMapEntry::new(
            DeterministicCborMapKey::Integer(DeterministicCborInteger::unsigned(1)),
            DeterministicCborValue::Null,
        ),
        DeterministicCborMapEntry::new(
            DeterministicCborMapKey::Integer(DeterministicCborInteger::unsigned(1)),
            DeterministicCborValue::Bool(true),
        ),
    ]);
    assert_eq!(
        encode_deterministic_cbor(&duplicate).unwrap_err(),
        DeterministicCborError::DuplicateMapKey
    );

    assert_eq!(
        decode_deterministic_cbor(&[0xa2, 0x01, 0xf6, 0x01, 0xf5]).unwrap_err(),
        DeterministicCborError::DuplicateMapKey
    );
}

#[test]
fn generic_cbor_rejects_noncanonical_and_unsupported_inputs() {
    assert_eq!(
        decode_deterministic_cbor(&[0x18, 0x00]).unwrap_err(),
        DeterministicCborError::NonCanonicalInteger
    );
    assert_eq!(
        decode_deterministic_cbor(&[0x00, 0x00]).unwrap_err(),
        DeterministicCborError::TrailingBytes
    );
    assert_eq!(
        decode_deterministic_cbor(&[0x3b, 0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,])
            .unwrap_err(),
        DeterministicCborError::NegativeIntegerOutOfRange
    );
    assert_eq!(
        decode_deterministic_cbor(&[0xa1, 0x40, 0xf6]).unwrap_err(),
        DeterministicCborError::UnsupportedMapKeyType
    );
    assert_eq!(
        decode_deterministic_cbor(&[0xa2, 0x61, b'a', 0x01, 0x00, 0x02]).unwrap_err(),
        DeterministicCborError::MapKeysOutOfOrder
    );
    assert_eq!(
        decode_deterministic_cbor(&[0xf9, 0x00, 0x00]).unwrap_err(),
        DeterministicCborError::UnsupportedSimpleValue
    );
}

#[test]
fn generic_cbor_executes_shared_resource_rejection_recipes() {
    let oversized_input = vec![0_u8; MAX_DETERMINISTIC_CBOR_INPUT_LEN + 1];
    assert_eq!(
        decode_deterministic_cbor(&oversized_input).unwrap_err(),
        DeterministicCborError::InputTooLarge
    );

    let oversized_output =
        DeterministicCborValue::Bytes(vec![0_u8; MAX_DETERMINISTIC_CBOR_INPUT_LEN]);
    assert_eq!(
        encode_deterministic_cbor(&oversized_output).unwrap_err(),
        DeterministicCborError::OutputTooLarge
    );

    // Each key is independently below the output limit, but their aggregate
    // encoded size is not. The semantic preflight must reject the complete
    // tree before the encoder creates sortable copies of either large key.
    let individually_bounded_key_bytes = (MAX_DETERMINISTIC_CBOR_INPUT_LEN / 2) + 1;
    let oversized_map = DeterministicCborValue::Map(vec![
        DeterministicCborMapEntry::new(
            DeterministicCborMapKey::text("a".repeat(individually_bounded_key_bytes)),
            DeterministicCborValue::Null,
        ),
        DeterministicCborMapEntry::new(
            DeterministicCborMapKey::text("b".repeat(individually_bounded_key_bytes)),
            DeterministicCborValue::Null,
        ),
    ]);
    assert_eq!(
        encode_deterministic_cbor(&oversized_map).unwrap_err(),
        DeterministicCborError::OutputTooLarge
    );

    let balanced_tree = DeterministicCborValue::Array(
        (0..256)
            .map(|_| {
                DeterministicCborValue::Array(
                    (0..256).map(|_| DeterministicCborValue::Null).collect(),
                )
            })
            .collect(),
    );
    assert_eq!(
        encode_deterministic_cbor(&balanced_tree).unwrap_err(),
        DeterministicCborError::NodeLimitExceeded
    );

    let oversized_container = DeterministicCborValue::Array(
        (0..=MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES)
            .map(|_| DeterministicCborValue::Null)
            .collect(),
    );
    assert_eq!(
        encode_deterministic_cbor(&oversized_container).unwrap_err(),
        DeterministicCborError::ContainerEntriesExceeded
    );

    let mut excessive_depth = DeterministicCborValue::Null;
    for _ in 0..=MAX_DETERMINISTIC_CBOR_NESTING_DEPTH {
        excessive_depth = DeterministicCborValue::Array(vec![excessive_depth]);
    }
    assert_eq!(
        encode_deterministic_cbor(&excessive_depth).unwrap_err(),
        DeterministicCborError::DepthExceeded
    );
}

#[test]
fn generic_cbor_decoder_enforces_node_container_and_depth_recipes() {
    let mut balanced_tree_bytes = Vec::with_capacity(66_307);
    balanced_tree_bytes.extend_from_slice(&[0x99, 0x01, 0x00]);
    for _ in 0..256 {
        balanced_tree_bytes.extend_from_slice(&[0x99, 0x01, 0x00]);
        balanced_tree_bytes.extend((0..256).map(|_| 0xf6));
    }
    assert_eq!(
        decode_deterministic_cbor(&balanced_tree_bytes).unwrap_err(),
        DeterministicCborError::NodeLimitExceeded
    );

    assert_eq!(
        decode_deterministic_cbor(&[0x99, 0x40, 0x01]).unwrap_err(),
        DeterministicCborError::ContainerEntriesExceeded
    );

    let mut excessive_depth_bytes = vec![0x81; MAX_DETERMINISTIC_CBOR_NESTING_DEPTH + 1];
    excessive_depth_bytes.push(0xf6);
    assert_eq!(
        decode_deterministic_cbor(&excessive_depth_bytes).unwrap_err(),
        DeterministicCborError::DepthExceeded
    );
}

#[test]
fn bounded_arbitrary_trees_preserve_semantic_properties() {
    for seed in 0..PROPERTY_CASES {
        let mut generator = PropertyGenerator::new(seed);
        let value = generated_value(&mut generator, PROPERTY_MAX_DEPTH);
        let encoded = encode_deterministic_cbor(&value).unwrap();

        // Successful encoding proves the independently computed preflight
        // length agreed with emission; the production encoder returns a typed
        // PreflightLengthMismatch instead of a partially trusted result when
        // those traversals ever diverge.
        let decoded = decode_deterministic_cbor(&encoded).unwrap();
        let reencoded = encode_deterministic_cbor(&decoded).unwrap();
        assert_eq!(reencoded.as_slice(), encoded.as_slice(), "seed {seed}");

        let mut with_trailing_byte = encoded.to_vec();
        with_trailing_byte.push(0);
        assert_eq!(
            decode_deterministic_cbor(&with_trailing_byte).unwrap_err(),
            DeterministicCborError::TrailingBytes,
            "seed {seed}"
        );

        let forward = generated_map(seed, false);
        let reverse = generated_map(seed, true);
        assert_eq!(
            encode_deterministic_cbor(&forward).unwrap().as_slice(),
            encode_deterministic_cbor(&reverse).unwrap().as_slice(),
            "seed {seed}"
        );
    }
}

#[test]
fn simultaneous_operations_are_deterministic_and_cleanup_is_independent() {
    let barrier = Arc::new(Barrier::new(usize::try_from(CONCURRENT_WORKERS).unwrap()));
    std::thread::scope(|scope| {
        let mut workers = Vec::with_capacity(usize::try_from(CONCURRENT_WORKERS).unwrap());
        for worker in 0..CONCURRENT_WORKERS {
            let worker_barrier = Arc::clone(&barrier);
            workers.push(scope.spawn(move || {
                worker_barrier.wait();
                for case in 0..CONCURRENT_CASES_PER_WORKER {
                    let seed = (worker << 32) | case;
                    let mut generator = PropertyGenerator::new(seed);
                    let value = generated_value(&mut generator, PROPERTY_MAX_DEPTH);
                    let encoded = encode_deterministic_cbor(&value).unwrap();

                    // Each decode owns its recursive payload independently.
                    // Dropping one owner exercises its recursive cleanup while
                    // the sibling must remain usable and byte-identical.
                    let first = decode_deterministic_cbor(&encoded).unwrap();
                    let second = decode_deterministic_cbor(&encoded).unwrap();
                    drop(first);
                    let reencoded = encode_deterministic_cbor(&second).unwrap();
                    assert_eq!(reencoded.as_slice(), encoded.as_slice(), "seed {seed}");
                }
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }
    });
}
