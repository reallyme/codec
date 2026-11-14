// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{MapKeyOrder, MapKeyOrderError};
use core::cmp::Ordering;

#[kani::proof]
#[kani::unwind(6)]
fn complete_map_admission_matches_strict_global_key_order() {
    const MAX_KEYS: usize = 4;
    let bytes: [[u8; 2]; MAX_KEYS] = kani::any();
    let lengths: [bool; MAX_KEYS] = kani::any();
    let count: usize = kani::any();
    kani::assume(count <= MAX_KEYS);

    let mut ranks = [0; MAX_KEYS];
    let mut order = MapKeyOrder::default();
    let mut actual = Ok(());
    for index in 0..count {
        let length = 1 + usize::from(lengths[index]);
        let key = &bytes[index][..length];
        // The rank stores the length above the big-endian byte value.
        ranks[index] = if lengths[index] {
            0x1_0000 + u32::from(u16::from_be_bytes(bytes[index]))
        } else {
            u32::from(bytes[index][0])
        };
        if actual.is_ok() {
            actual = order.admit(key);
        }
    }

    // Check every pair independently. This proves global strict order and
    // uniqueness, including repeat keys separated by intervening entries.
    let mut strictly_ordered = true;
    for left in 0..count {
        for right in left + 1..count {
            strictly_ordered &= ranks[left] < ranks[right];
        }
    }
    assert_eq!(actual.is_ok(), strictly_ordered);

    // The first invalid adjacent transition determines the typed error.
    let first_violation = ranks[..count]
        .windows(2)
        .map(|pair| pair[0].cmp(&pair[1]))
        .find(|&order| order != Ordering::Less);
    let expected = match first_violation {
        Some(Ordering::Equal) => Err(MapKeyOrderError::Duplicate),
        Some(Ordering::Greater) => Err(MapKeyOrderError::Descending),
        _ => Ok(()),
    };
    assert_eq!(actual, expected);

    kani::cover!(count == 0 && actual.is_ok(), "empty map");
    kani::cover!(
        count == 4 && actual.is_ok() && lengths[0] != lengths[3],
        "canonical mixed-width map"
    );
    kani::cover!(
        count == 4
            && ranks[0] < ranks[1]
            && ranks[1] < ranks[2]
            && ranks[2] == ranks[3]
            && actual == Err(MapKeyOrderError::Duplicate),
        "duplicate after a valid prefix"
    );
    kani::cover!(
        count == 4
            && ranks[0] < ranks[1]
            && ranks[1] < ranks[2]
            && ranks[0] == ranks[3]
            && actual == Err(MapKeyOrderError::Descending),
        "non-adjacent duplicate forces descending order"
    );
}
