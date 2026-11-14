// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

fn deterministic_value_proto(
    value: &DeterministicCborValue,
) -> Result<CodecDeterministicCborValue, CodecWireError> {
    let value = match value {
        DeterministicCborValue::Null => CodecDeterministicCborNull {

        }
        .into(),
        DeterministicCborValue::Bool(value) => CodecDeterministicCborBool {
            value: *value,

        }
        .into(),
        DeterministicCborValue::Integer(value) => deterministic_integer_proto(value)?.into(),
        DeterministicCborValue::Text(value) => CodecDeterministicCborText {
            value: try_copy_deterministic_text(value)?.into(),

        }
        .into(),
        DeterministicCborValue::Bytes(value) => CodecDeterministicCborBytes {
            value: try_copy_deterministic_bytes(value)?.into(),

        }
        .into(),
        DeterministicCborValue::Array(values) => {
            let mut proto_values = try_deterministic_vec(values.len())?;
            for value in values {
                proto_values.push(deterministic_value_proto(value)?);
            }
            CodecDeterministicCborArray {
                values: proto_values.into(),

            }
            .into()
        }
        DeterministicCborValue::Map(entries) => {
            let mut proto_entries = try_deterministic_vec(entries.len())?;
            for entry in entries {
                proto_entries.push(CodecDeterministicCborMapEntry {
                    key: buffa::MessageField::some(deterministic_key_proto(entry.key())?),
                    value: buffa::MessageField::some(deterministic_value_proto(entry.value())?),

                });
            }
            CodecDeterministicCborMap {
                entries: proto_entries.into(),

            }
            .into()
        }
        _ => return Err(internal_wire_error()),
    };

    Ok(CodecDeterministicCborValue {
        value: Some(value),

    })
}

fn deterministic_integer_proto(
    value: &DeterministicCborInteger,
) -> Result<CodecDeterministicCborInteger, CodecWireError> {
    let value = match value {
        DeterministicCborInteger::Unsigned(value) => CodecDeterministicCborUnsignedInteger {
            value: *value,

        }
        .into(),
        DeterministicCborInteger::Negative(value) => CodecDeterministicCborNegativeInteger {
            value: value.value(),

        }
        .into(),
        _ => return Err(internal_wire_error()),
    };
    Ok(CodecDeterministicCborInteger {
        value: Some(value),

    })
}

fn deterministic_key_proto(
    value: &DeterministicCborMapKey,
) -> Result<CodecDeterministicCborMapKey, CodecWireError> {
    let key = match value {
        DeterministicCborMapKey::Integer(value) => deterministic_integer_proto(value)?.into(),
        DeterministicCborMapKey::Text(value) => CodecDeterministicCborText {
            value: try_copy_deterministic_text(value)?.into(),

        }
        .into(),
        _ => return Err(internal_wire_error()),
    };
    Ok(CodecDeterministicCborMapKey {
        key: Some(key),

    })
}

fn dag_cbor_value_proto(value: &CborValue) -> Result<CodecDeterministicCborValue, CodecWireError> {
    let value = match value {
        CborValue::Null => CodecDeterministicCborNull {

        }
        .into(),
        CborValue::Bool(value) => CodecDeterministicCborBool {
            value: *value,

        }
        .into(),
        CborValue::Int(value) => dag_cbor_integer_proto(*value).into(),
        CborValue::String(value) => CodecDeterministicCborText {
            value: try_copy_deterministic_text(value)?.into(),

        }
        .into(),
        CborValue::Bytes(value) => CodecDeterministicCborBytes {
            value: try_copy_deterministic_bytes(value)?.into(),

        }
        .into(),
        CborValue::Array(values) => {
            let mut proto_values = try_deterministic_vec(values.len())?;
            for value in values {
                proto_values.push(dag_cbor_value_proto(value)?);
            }
            CodecDeterministicCborArray {
                values: proto_values.into(),

            }
            .into()
        }
        CborValue::Map(entries) => {
            let mut proto_entries = try_deterministic_vec(entries.len())?;
            for (key, value) in entries {
                proto_entries.push(CodecDeterministicCborMapEntry {
                    key: buffa::MessageField::some(dag_cbor_key_proto(key)?),
                    value: buffa::MessageField::some(dag_cbor_value_proto(value)?),

                });
            }
            CodecDeterministicCborMap {
                entries: proto_entries.into(),

            }
            .into()
        }
        _ => return Err(internal_wire_error()),
    };

    Ok(CodecDeterministicCborValue {
        value: Some(value),

    })
}

fn dag_cbor_integer_proto(value: i64) -> CodecDeterministicCborInteger {
    let value = if value >= 0 {
        CodecDeterministicCborUnsignedInteger {
            value: value.unsigned_abs(),

        }
        .into()
    } else {
        CodecDeterministicCborNegativeInteger {
            value,

        }
        .into()
    };
    CodecDeterministicCborInteger {
        value: Some(value),

    }
}

fn dag_cbor_key_proto(value: &str) -> Result<CodecDeterministicCborMapKey, CodecWireError> {
    Ok(CodecDeterministicCborMapKey {
        key: Some(
            CodecDeterministicCborText {
                value: try_copy_deterministic_text(value)?.into(),

            }
            .into(),
        ),

    })
}
