// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Exercise strict DER readers and exact integer-pair canonical round trips.

#![no_main]

use libfuzzer_sys::fuzz_target;
use reallyme_codec::der::{
    decode_der, decode_unsigned_integer_pair, DerError, DerLimits, DerReader,
};

fn encode_tlv(tag: u8, contents: &[u8]) -> Option<Vec<u8>> {
    let mut output = vec![tag];
    if contents.len() < 128 {
        output.push(u8::try_from(contents.len()).ok()?);
    } else {
        let length = contents.len().to_be_bytes();
        let first = length.iter().position(|byte| *byte != 0)?;
        let octets = length.get(first..)?;
        output.push(0x80 | u8::try_from(octets.len()).ok()?);
        output.extend_from_slice(octets);
    }
    output.len().checked_add(contents.len())?;
    output.extend_from_slice(contents);
    Some(output)
}

fn encode_unsigned(value: &[u8]) -> Option<Vec<u8>> {
    let mut contents = Vec::new();
    if value.first().is_some_and(|byte| byte & 0x80 != 0) {
        contents.push(0);
    }
    contents.len().checked_add(value.len())?;
    contents.extend_from_slice(value);
    encode_tlv(0x02, &contents)
}

fn read_chain(reader: &mut DerReader<'_>) -> Result<(), DerError> {
    reader.read_sequence(|nested| {
        if !nested.is_finished() {
            read_chain(nested)?;
        }
        Ok(())
    })
}

fuzz_target!(|input: &[u8]| {
    let limits = DerLimits::default();
    if let Ok(pair) = decode_unsigned_integer_pair(input, limits) {
        assert!(!pair.first().is_empty());
        assert!(!pair.second().is_empty());
        let first = encode_unsigned(pair.first());
        let second = encode_unsigned(pair.second());
        if let (Some(mut contents), Some(second)) = (first, second) {
            assert!(contents.len().checked_add(second.len()).is_some());
            contents.extend_from_slice(&second);
            assert_eq!(encode_tlv(0x30, &contents).as_deref(), Some(input));
        } else {
            panic!("accepted integer pair could not be encoded");
        }
    }
    let _ = decode_der(input, limits, |r| r.read_integer().map(|_| ()));
    let _ = decode_der(input, limits, |r| r.read_bit_string().map(|_| ()));
    let _ = decode_der(input, limits, |r| r.read_octet_string().map(|_| ()));
    let _ = decode_der(input, limits, |r| r.read_object_identifier().map(|_| ()));
    let _ = decode_der(input, limits, |r| r.read_utf8_string().map(|_| ()));
    let _ = decode_der(input, limits, |r| r.read_boolean().map(|_| ()));
    let _ = decode_der(input, limits, |r| r.read_null());
    let _ = decode_der(input, limits, read_chain);
    let _ = decode_der(input, limits, |r| {
        r.read_set_of(|set| {
            while !set.is_finished() {
                set.read_unsigned_integer()?;
            }
            Ok(())
        })
    });
});
