// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![cfg(feature = "json")]
#![allow(missing_docs)]

use std::fmt;

use reallyme_codec::json::from_slice;
use serde::de::{DeserializeOwned, IgnoredAny, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer};

// Deliberately permissive visitors expose shape checks that ordinary derived
// targets would otherwise enforce themselves. The requested deserializer
// method must still reject incompatible JSON, as the streaming parser does.
struct Permissive;
impl<'de> Visitor<'de> for Permissive {
    type Value = ();

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a test value")
    }
    fn visit_unit<E>(self) -> Result<(), E> {
        Ok(())
    }
    fn visit_bool<E>(self, _: bool) -> Result<(), E> {
        Ok(())
    }
    fn visit_i64<E>(self, _: i64) -> Result<(), E> {
        Ok(())
    }
    fn visit_u64<E>(self, _: u64) -> Result<(), E> {
        Ok(())
    }
    fn visit_i128<E>(self, _: i128) -> Result<(), E> {
        Ok(())
    }
    fn visit_u128<E>(self, _: u128) -> Result<(), E> {
        Ok(())
    }
    fn visit_f64<E>(self, _: f64) -> Result<(), E> {
        Ok(())
    }
    fn visit_str<E>(self, _: &str) -> Result<(), E> {
        Ok(())
    }
    fn visit_bytes<E>(self, _: &[u8]) -> Result<(), E> {
        Ok(())
    }
    fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<(), A::Error> {
        while access.next_element::<IgnoredAny>()?.is_some() {}
        Ok(())
    }
    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<(), A::Error> {
        while access.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
        Ok(())
    }
}

macro_rules! requested_shape {
    ($name:ident, $method:ident $(, $argument:expr)*) => {
        struct $name;
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
                deserializer.$method($($argument,)* Permissive).map(|()| Self)
            }
        }
    };
}
requested_shape!(Boolean, deserialize_bool);
requested_shape!(Signed, deserialize_i64);
requested_shape!(Unsigned, deserialize_u64);
requested_shape!(WideSigned, deserialize_i128);
requested_shape!(WideUnsigned, deserialize_u128);
requested_shape!(Float, deserialize_f64);
requested_shape!(SingleFloat, deserialize_f32);
requested_shape!(Text, deserialize_str);
requested_shape!(OwnedText, deserialize_string);
requested_shape!(Character, deserialize_char);
requested_shape!(Identifier, deserialize_identifier);
requested_shape!(Sequence, deserialize_seq);
requested_shape!(Tuple, deserialize_tuple, 1);
requested_shape!(TupleStruct, deserialize_tuple_struct, "TupleStruct", 1);
requested_shape!(Map, deserialize_map);
requested_shape!(Record, deserialize_struct, "Record", &["x"]);
requested_shape!(Unit, deserialize_unit);
requested_shape!(UnitStruct, deserialize_unit_struct, "UnitStruct");
requested_shape!(Bytes, deserialize_bytes);
requested_shape!(ByteBuffer, deserialize_byte_buf);

fn matches_streaming_parser<T: DeserializeOwned>() {
    for input in [
        "null",
        "true",
        "0",
        "-1",
        "1.25",
        "\"text\"",
        "\"a\\u0000b\"",
        "[]",
        "[1]",
        "{}",
        "{\"x\":1}",
    ] {
        assert_eq!(
            from_slice::<T>(input.as_bytes(), None).is_ok(),
            serde_json::from_str::<T>(input).is_ok(),
            "{} accepts a different shape for {input}",
            std::any::type_name::<T>()
        );
    }
}

#[test]
fn requested_shapes_match_streaming_serde_with_permissive_visitors() {
    matches_streaming_parser::<Boolean>();
    matches_streaming_parser::<Signed>();
    matches_streaming_parser::<Unsigned>();
    matches_streaming_parser::<WideSigned>();
    matches_streaming_parser::<WideUnsigned>();
    matches_streaming_parser::<Float>();
    matches_streaming_parser::<SingleFloat>();
    matches_streaming_parser::<Text>();
    matches_streaming_parser::<OwnedText>();
    matches_streaming_parser::<Character>();
    matches_streaming_parser::<Identifier>();
    matches_streaming_parser::<Sequence>();
    matches_streaming_parser::<Tuple>();
    matches_streaming_parser::<TupleStruct>();
    matches_streaming_parser::<Map>();
    matches_streaming_parser::<Record>();
    matches_streaming_parser::<Unit>();
    matches_streaming_parser::<UnitStruct>();
    matches_streaming_parser::<Bytes>();
    matches_streaming_parser::<ByteBuffer>();
}
