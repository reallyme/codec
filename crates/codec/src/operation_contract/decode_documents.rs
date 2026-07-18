// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

fn process_deterministic_cbor_decode(
    encoded: &[u8],
) -> Result<CodecDeterministicCborDecodeResult, CodecWireError> {
    let value = decode_deterministic_cbor_value(encoded).map_err(deterministic_cbor_wire_error)?;
    Ok(CodecDeterministicCborDecodeResult {
        value: buffa::MessageField::some(deterministic_value_proto(&value)?),
        __buffa_unknown_fields: Default::default(),
    })
}

fn deterministic_value_from_field<P: buffa::ProtoBox<CodecDeterministicCborValue>>(
    field: &buffa::MessageField<CodecDeterministicCborValue, P>,
) -> Result<DeterministicCborValue, CodecWireError> {
    let Some(value) = field.as_option() else {
        return Err(malformed_request_wire_error());
    };
    deterministic_value_from_proto(value)
}

fn deterministic_value_from_proto(
    value: &CodecDeterministicCborValue,
) -> Result<DeterministicCborValue, CodecWireError> {
    match value.value.as_ref() {
        Some(codec_deterministic_cbor_value::Value::NullValue(_)) => {
            Ok(DeterministicCborValue::Null)
        }
        Some(codec_deterministic_cbor_value::Value::BoolValue(value)) => {
            Ok(DeterministicCborValue::Bool(value.value))
        }
        Some(codec_deterministic_cbor_value::Value::IntegerValue(value)) => Ok(
            DeterministicCborValue::Integer(deterministic_integer_from_proto(value)?),
        ),
        Some(codec_deterministic_cbor_value::Value::TextValue(value)) => Ok(
            DeterministicCborValue::Text(try_copy_deterministic_text(&value.value)?),
        ),
        Some(codec_deterministic_cbor_value::Value::BytesValue(value)) => Ok(
            DeterministicCborValue::Bytes(try_copy_deterministic_bytes(&value.value)?),
        ),
        Some(codec_deterministic_cbor_value::Value::ArrayValue(value)) => {
            let mut values = try_deterministic_vec(value.values.len())?;
            for child in &value.values {
                values.push(deterministic_value_from_proto(child)?);
            }
            Ok(DeterministicCborValue::Array(values))
        }
        Some(codec_deterministic_cbor_value::Value::MapValue(value)) => {
            let mut entries = try_deterministic_vec(value.entries.len())?;
            for entry in &value.entries {
                entries.push(DeterministicCborMapEntry::new(
                    deterministic_key_from_field(&entry.key)?,
                    deterministic_value_from_field(&entry.value)?,
                ));
            }
            Ok(DeterministicCborValue::Map(entries))
        }
        None => Err(malformed_request_wire_error()),
    }
}

fn dag_cbor_value_from_field<P: buffa::ProtoBox<CodecDeterministicCborValue>>(
    field: &buffa::MessageField<CodecDeterministicCborValue, P>,
) -> Result<CborValue, CodecWireError> {
    let Some(value) = field.as_option() else {
        return Err(malformed_request_wire_error());
    };
    dag_cbor_value_from_proto(value)
}

fn dag_cbor_value_from_proto(
    value: &CodecDeterministicCborValue,
) -> Result<CborValue, CodecWireError> {
    match value.value.as_ref() {
        Some(codec_deterministic_cbor_value::Value::NullValue(_)) => Ok(CborValue::Null),
        Some(codec_deterministic_cbor_value::Value::BoolValue(value)) => {
            Ok(CborValue::Bool(value.value))
        }
        Some(codec_deterministic_cbor_value::Value::IntegerValue(value)) => {
            Ok(CborValue::Int(dag_cbor_i64_from_proto(value)?))
        }
        Some(codec_deterministic_cbor_value::Value::TextValue(value)) => Ok(CborValue::String(
            try_copy_deterministic_text(&value.value)?,
        )),
        Some(codec_deterministic_cbor_value::Value::BytesValue(value)) => Ok(CborValue::Bytes(
            try_copy_deterministic_bytes(&value.value)?,
        )),
        Some(codec_deterministic_cbor_value::Value::ArrayValue(value)) => {
            let mut values = try_deterministic_vec(value.values.len())?;
            for child in &value.values {
                values.push(dag_cbor_value_from_proto(child)?);
            }
            Ok(CborValue::Array(values))
        }
        Some(codec_deterministic_cbor_value::Value::MapValue(value)) => {
            let mut entries = try_deterministic_vec(value.entries.len())?;
            for entry in &value.entries {
                entries.push((
                    dag_cbor_key_from_field(&entry.key)?,
                    dag_cbor_value_from_field(&entry.value)?,
                ));
            }
            Ok(CborValue::Map(entries))
        }
        None => Err(malformed_request_wire_error()),
    }
}

fn dag_cbor_i64_from_proto(value: &CodecDeterministicCborInteger) -> Result<i64, CodecWireError> {
    match value.value.as_ref() {
        Some(codec_deterministic_cbor_integer::Value::UnsignedValue(value)) => {
            i64::try_from(value.value).map_err(|_| malformed_request_wire_error())
        }
        Some(codec_deterministic_cbor_integer::Value::NegativeValue(value)) => {
            if value.value < 0 {
                Ok(value.value)
            } else {
                Err(malformed_request_wire_error())
            }
        }
        None => Err(malformed_request_wire_error()),
    }
}

fn dag_cbor_key_from_field<P: buffa::ProtoBox<CodecDeterministicCborMapKey>>(
    field: &buffa::MessageField<CodecDeterministicCborMapKey, P>,
) -> Result<String, CodecWireError> {
    let Some(value) = field.as_option() else {
        return Err(malformed_request_wire_error());
    };
    match value.key.as_ref() {
        Some(codec_deterministic_cbor_map_key::Key::TextKey(value)) => {
            try_copy_deterministic_text(&value.value)
        }
        Some(codec_deterministic_cbor_map_key::Key::IntegerKey(_)) => {
            Err(malformed_request_wire_error())
        }
        None => Err(malformed_request_wire_error()),
    }
}

fn deterministic_integer_from_proto(
    value: &CodecDeterministicCborInteger,
) -> Result<DeterministicCborInteger, CodecWireError> {
    match value.value.as_ref() {
        Some(codec_deterministic_cbor_integer::Value::UnsignedValue(value)) => {
            Ok(DeterministicCborInteger::unsigned(value.value))
        }
        Some(codec_deterministic_cbor_integer::Value::NegativeValue(value)) => {
            DeterministicCborInteger::negative(value.value)
                .map_err(|error| deterministic_cbor_wire_error(DeterministicCborError::from(error)))
        }
        None => Err(malformed_request_wire_error()),
    }
}

fn deterministic_key_from_field<P: buffa::ProtoBox<CodecDeterministicCborMapKey>>(
    field: &buffa::MessageField<CodecDeterministicCborMapKey, P>,
) -> Result<DeterministicCborMapKey, CodecWireError> {
    let Some(value) = field.as_option() else {
        return Err(malformed_request_wire_error());
    };
    match value.key.as_ref() {
        Some(codec_deterministic_cbor_map_key::Key::IntegerKey(value)) => Ok(
            DeterministicCborMapKey::Integer(deterministic_integer_from_proto(value)?),
        ),
        Some(codec_deterministic_cbor_map_key::Key::TextKey(value)) => Ok(
            DeterministicCborMapKey::text(try_copy_deterministic_text(&value.value)?),
        ),
        None => Err(malformed_request_wire_error()),
    }
}
