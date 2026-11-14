// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![no_main]

use codec_cbor::general::{
    encode, frame_prefix, validate_exact, Cursor, ErrorReason, Float, LengthPolicy, Limits,
    MapEntry, Profile, ValueRef,
};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    let limits = match Limits::default().with_work(65_536) {
        Ok(limits) => limits,
        Err(_) => return,
    };
    if let Some(prefix) = data.get(..8) {
        if let Ok(octets) = <[u8; 8]>::try_from(prefix) {
            let argument = u64::from_be_bytes(octets);
            let float = Float::binary64(argument);
            let payload = ValueRef::Bytes(data);
            let entries = [
                MapEntry::new(ValueRef::Unsigned(argument), ValueRef::Negative(argument)),
                MapEntry::new(ValueRef::Bool(false), ValueRef::Float(&float)),
            ];
            let children = [ValueRef::Map(&entries), ValueRef::Tag(argument, &payload)];
            let value = ValueRef::Array(&children);
            if let Ok(encoded) = encode(&value, Profile::preferred(), limits) {
                assert!(validate_exact(encoded.as_bytes(), Profile::preferred(), limits).is_ok());
            }
        }
    }
    if let Ok((frame, rest)) = frame_prefix(data, limits, LengthPolicy::AllowIndefinite) {
        assert_eq!(
            frame.received_bytes().len().checked_add(rest.len()),
            Some(data.len())
        );
        for profile in [Profile::received(), Profile::preferred()] {
            if validate_exact(frame.received_bytes(), profile, limits).is_ok() {
                match encode(&ValueRef::Received(&frame), profile, limits) {
                    Ok(encoded) => {
                        assert_eq!(encoded.as_bytes(), frame.received_bytes());
                        assert!(validate_exact(encoded.as_bytes(), profile, limits).is_ok());
                    }
                    // Encoding shares its work budget across preflight, writing,
                    // and validity checks, so it may exhaust a smaller allowance.
                    Err(error) => assert_eq!(error.reason(), ErrorReason::WorkLimit),
                }
            }
        }
        if let Ok(tagged) = frame.tagged_bytes() {
            let _ = tagged.validate_embedded(Profile::received(), limits);
        }
        if let Ok(children) = frame.children() {
            for child in children {
                if child.is_err() {
                    break;
                }
            }
        }
    }
    if let Ok(mut cursor) = Cursor::new(data, limits, LengthPolicy::AllowIndefinite) {
        let mut consumed = 0_usize;
        while let Some(result) = cursor.next() {
            match result {
                Ok(frame) => {
                    assert!(!frame.received_bytes().is_empty());
                    consumed = consumed.checked_add(frame.received_bytes().len()).unwrap();
                }
                Err(_) => {
                    assert!(cursor.next().is_none());
                    break;
                }
            }
        }
        assert_eq!(cursor.consumed_bytes().len(), consumed);
        assert_eq!(
            consumed.checked_add(cursor.remaining_bytes().len()),
            Some(data.len())
        );
    }
});
