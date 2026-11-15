// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{lookup_codec_prefix, MULTICODEC_TABLE};

#[kani::proof]
#[kani::unwind(8)]
fn registered_multicodecs_have_distinct_minimal_prefixes() {
    let first: usize = kani::any();
    let second: usize = kani::any();
    kani::assume(first < MULTICODEC_TABLE.len() && second < MULTICODEC_TABLE.len());
    let a = MULTICODEC_TABLE[first].1.codec;
    let b = MULTICODEC_TABLE[second].1.codec;
    let Some((&last, continuation)) = a.split_last() else {
        kani::assert(false, "every registered codec has a nonempty prefix");
        return;
    };
    assert!(a.len() <= 3);
    assert!(continuation.iter().all(|byte| byte >= &128));
    assert!(last < 128 && (continuation.is_empty() || last > 0));
    if first != second {
        assert!(!a.starts_with(b) && !b.starts_with(a));
    }
    kani::cover!(first != second, "compare distinct registrations");
    kani::cover!(a.len() == 1, "one-octet registration");
    kani::cover!(a.len() == 3, "three-octet registration");
}

#[kani::proof]
#[kani::unwind(50)]
fn lookup_accepts_only_the_registered_terminated_prefix() {
    let bytes: [u8; 3] = kani::any();
    let len: usize = kani::any();
    kani::assume(len <= bytes.len());
    let wire = &bytes[..len];
    // The first terminating octet fixes the prefix independently of the
    // table search. No registered prefix can match past that boundary.
    let prefix = wire
        .iter()
        .position(|byte| byte < &128)
        .map(|end| &wire[..=end]);
    let expected = prefix.and_then(|prefix| {
        MULTICODEC_TABLE
            .iter()
            .find(|(_, spec)| spec.codec == prefix)
    });
    let actual = lookup_codec_prefix(wire);
    match (actual, expected) {
        (None, None) => {}
        (Some(actual), Some((name, spec))) => {
            assert_eq!(actual.name, *name);
            assert_eq!(actual.codec, spec.codec);
            assert_eq!(actual.key_material, spec.key_material);
            assert_eq!(actual.key_length, spec.key_length);
        }
        _ => kani::assert(
            false,
            "the registered terminated prefix determines the lookup",
        ),
    }
    kani::cover!(prefix.is_none() && len == 3, "unterminated prefix");
    kani::cover!(
        prefix.is_some() && expected.is_none(),
        "unknown terminated prefix"
    );
    kani::cover!(
        wire == [0x92, 0x00] && expected.is_none(),
        "overlong encoding of a registered code"
    );
    kani::cover!(
        matches!(expected, Some((_, spec)) if spec.codec.len() < len),
        "payload follows a registered prefix"
    );
    kani::cover!(
        matches!(expected, Some((_, spec)) if spec.codec.len() == 3),
        "longest registered prefix"
    );
}
