// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! DER resource ceilings and nonrecoverable failure-state tests.

#![cfg(feature = "der")]

#[path = "der/support/mod.rs"]
mod support;

use reallyme_codec::der::{
    decode_der, decode_unsigned_integer_pair, DerError, DerErrorReason, DerLimits, DerReader,
    MAX_DER_ELEMENTS, MAX_DER_INPUT_LEN, MAX_DER_INTEGER_LEN, MAX_DER_NESTING_DEPTH,
};
use support::{pair, tlv};

fn read_sequence_chain(reader: &mut DerReader<'_>) -> Result<(), DerError> {
    reader.read_sequence(|nested| {
        if !nested.is_finished() {
            read_sequence_chain(nested)?;
        }
        Ok(())
    })
}

#[test]
fn policies_reject_zero_overflowing_and_above_hard_limit_values() {
    for tuple in [
        (0, 1, 1, 1),
        (1, 0, 1, 1),
        (1, 1, 0, 1),
        (1, 1, 1, 0),
        (usize::MAX, 1, 1, 1),
        (1, usize::MAX, 1, 1),
        (1, 1, usize::MAX, 1),
        (1, 1, 1, usize::MAX),
        (MAX_DER_INPUT_LEN + 1, 1, 1, 1),
        (1, MAX_DER_NESTING_DEPTH + 1, 1, 1),
        (1, 1, MAX_DER_ELEMENTS + 1, 1),
        (1, 1, 1, MAX_DER_INTEGER_LEN + 1),
    ] {
        assert_eq!(
            DerLimits::new(tuple.0, tuple.1, tuple.2, tuple.3),
            Err(DerError::new(DerErrorReason::InvalidLimits))
        );
    }
}

#[test]
fn exact_input_limit_is_accepted_and_one_extra_byte_rejected() -> Result<(), DerError> {
    let input = tlv(0x04, &vec![0x5a; MAX_DER_INPUT_LEN - 5]);
    assert_eq!(input.len(), MAX_DER_INPUT_LEN);
    assert_eq!(
        decode_der(&input, DerLimits::default(), |r| r.read_octet_string())?.len(),
        MAX_DER_INPUT_LEN - 5
    );
    let mut oversized = input;
    oversized.push(0);
    assert_eq!(
        decode_der(&oversized, DerLimits::default(), |r| r.read_null()),
        Err(DerError::new(DerErrorReason::InputTooLarge))
    );
    Ok(())
}

#[test]
fn exact_container_depth_is_accepted_and_one_extra_level_rejected() -> Result<(), DerError> {
    let mut input = vec![0x30, 0];
    for _ in 1..MAX_DER_NESTING_DEPTH {
        input = tlv(0x30, &input);
    }
    decode_der(&input, DerLimits::default(), read_sequence_chain)?;
    let too_deep = tlv(0x30, &input);
    assert_eq!(
        decode_der(&too_deep, DerLimits::default(), read_sequence_chain),
        Err(DerError::new(DerErrorReason::NestingTooDeep))
    );
    let malicious = (0..1_000).fold(vec![0x30, 0], |inner, _| tlv(0x30, &inner));
    assert_eq!(
        decode_der(&malicious, DerLimits::default(), read_sequence_chain),
        Err(DerError::new(DerErrorReason::NestingTooDeep))
    );
    Ok(())
}

#[test]
fn exact_element_budget_is_shared_by_all_siblings() -> Result<(), DerError> {
    let contents = [0x05, 0].repeat(MAX_DER_ELEMENTS - 1);
    let input = tlv(0x30, &contents);
    let read_nulls = |r: &mut DerReader<'_>| {
        r.read_sequence(|s| {
            while !s.is_finished() {
                s.read_null()?;
            }
            Ok(())
        })
    };
    decode_der(&input, DerLimits::default(), read_nulls)?;
    let too_many = tlv(0x30, &[0x05, 0].repeat(MAX_DER_ELEMENTS));
    assert_eq!(
        decode_der(&too_many, DerLimits::default(), read_nulls),
        Err(DerError::new(DerErrorReason::TooManyElements))
    );

    let mut siblings = pair(&[1], &[2]);
    siblings.extend_from_slice(&pair(&[3], &[4]));
    let input = tlv(0x30, &siblings);
    assert_eq!(
        decode_der(&input, DerLimits::new(32, 2, 6, 1)?, |r| r.read_sequence(
            |outer| {
                for _ in 0..2 {
                    outer.read_sequence(|s| {
                        s.read_unsigned_integer()?;
                        s.read_unsigned_integer()?;
                        Ok(())
                    })?;
                }
                Ok(())
            }
        )),
        Err(DerError::new(DerErrorReason::TooManyElements))
    );
    Ok(())
}

#[test]
fn caller_can_tighten_each_limit() -> Result<(), DerError> {
    let input = pair(&[1], &[2]);
    assert!(decode_unsigned_integer_pair(&input, DerLimits::new(8, 1, 3, 1)?).is_ok());
    assert!(
        matches!(decode_unsigned_integer_pair(&input, DerLimits::new(7, 1, 3, 1)?),
        Err(error) if error.reason() == DerErrorReason::InputTooLarge)
    );
    assert!(
        matches!(decode_unsigned_integer_pair(&input, DerLimits::new(8, 1, 2, 1)?),
        Err(error) if error.reason() == DerErrorReason::TooManyElements)
    );
    Ok(())
}

#[test]
fn errors_cannot_be_ignored_or_recovered_into_success() -> Result<(), DerError> {
    let result = decode_der(&[0x05, 0], DerLimits::default(), |r| {
        let first = r.read_unsigned_integer().err();
        assert_eq!(first, Some(DerError::new(DerErrorReason::UnexpectedTag)));
        assert_eq!(r.read_null().err(), first);
        assert!(!r.is_finished());
        assert_eq!(r.finish().err(), first);
        Ok(())
    });
    assert_eq!(result, Err(DerError::new(DerErrorReason::UnexpectedTag)));

    let result = decode_der(&[0x30, 2, 0x05, 0], DerLimits::default(), |r| {
        let _ignored = r.read_sequence(|nested| {
            let _ignored = nested.read_boolean();
            Ok(())
        });
        Ok(())
    });
    assert_eq!(result, Err(DerError::new(DerErrorReason::UnexpectedTag)));

    let input = pair(&[1, 2], &[1]);
    let result = decode_der(&input, DerLimits::new(32, 1, 3, 1)?, |r| {
        let _ignored = r.read_sequence(|nested| {
            let _ignored = nested.read_unsigned_integer();
            Ok(())
        });
        Ok(())
    });
    assert_eq!(result, Err(DerError::new(DerErrorReason::IntegerTooLarge)));
    Ok(())
}

#[test]
fn root_and_nested_containers_require_complete_consumption() {
    assert_eq!(
        decode_der(&[0x05, 0, 0x05, 0], DerLimits::default(), |r| {
            r.read_null()?;
            r.read_null()
        }),
        Err(DerError::new(DerErrorReason::TrailingData))
    );
    assert_eq!(
        decode_der(&[0x05, 0], DerLimits::default(), |_| Ok(())),
        Err(DerError::new(DerErrorReason::TrailingData))
    );
    assert_eq!(
        decode_der(&[0x30, 2, 0x05, 0], DerLimits::default(), |r| r
            .read_sequence(|_| Ok(()))),
        Err(DerError::new(DerErrorReason::TrailingData))
    );
}
