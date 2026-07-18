// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

fn deterministic_value_proto(
    value: &DeterministicCborValue,
) -> Result<CodecDeterministicCborValue, CodecWireError> {
    let value = match value {
        DeterministicCborValue::Null => CodecDeterministicCborNull {
            __buffa_unknown_fields: Default::default(),
        }
        .into(),
        DeterministicCborValue::Bool(value) => CodecDeterministicCborBool {
            value: *value,
            __buffa_unknown_fields: Default::default(),
        }
        .into(),
        DeterministicCborValue::Integer(value) => deterministic_integer_proto(value)?.into(),
        DeterministicCborValue::Text(value) => CodecDeterministicCborText {
            value: try_copy_deterministic_text(value)?,
            __buffa_unknown_fields: Default::default(),
        }
        .into(),
        DeterministicCborValue::Bytes(value) => CodecDeterministicCborBytes {
            value: try_copy_deterministic_bytes(value)?,
            __buffa_unknown_fields: Default::default(),
        }
        .into(),
        DeterministicCborValue::Array(values) => {
            let mut proto_values = try_deterministic_vec(values.len())?;
            for value in values {
                proto_values.push(deterministic_value_proto(value)?);
            }
            CodecDeterministicCborArray {
                values: proto_values,
                __buffa_unknown_fields: Default::default(),
            }
            .into()
        }
        DeterministicCborValue::Map(entries) => {
            let mut proto_entries = try_deterministic_vec(entries.len())?;
            for entry in entries {
                proto_entries.push(CodecDeterministicCborMapEntry {
                    key: buffa::MessageField::some(deterministic_key_proto(entry.key())?),
                    value: buffa::MessageField::some(deterministic_value_proto(entry.value())?),
                    __buffa_unknown_fields: Default::default(),
                });
            }
            CodecDeterministicCborMap {
                entries: proto_entries,
                __buffa_unknown_fields: Default::default(),
            }
            .into()
        }
        _ => return Err(internal_wire_error()),
    };

    Ok(CodecDeterministicCborValue {
        value: Some(value),
        __buffa_unknown_fields: Default::default(),
    })
}

fn deterministic_integer_proto(
    value: &DeterministicCborInteger,
) -> Result<CodecDeterministicCborInteger, CodecWireError> {
    let value = match value {
        DeterministicCborInteger::Unsigned(value) => CodecDeterministicCborUnsignedInteger {
            value: *value,
            __buffa_unknown_fields: Default::default(),
        }
        .into(),
        DeterministicCborInteger::Negative(value) => CodecDeterministicCborNegativeInteger {
            value: value.value(),
            __buffa_unknown_fields: Default::default(),
        }
        .into(),
        _ => return Err(internal_wire_error()),
    };
    Ok(CodecDeterministicCborInteger {
        value: Some(value),
        __buffa_unknown_fields: Default::default(),
    })
}

fn deterministic_key_proto(
    value: &DeterministicCborMapKey,
) -> Result<CodecDeterministicCborMapKey, CodecWireError> {
    let key = match value {
        DeterministicCborMapKey::Integer(value) => deterministic_integer_proto(value)?.into(),
        DeterministicCborMapKey::Text(value) => CodecDeterministicCborText {
            value: try_copy_deterministic_text(value)?,
            __buffa_unknown_fields: Default::default(),
        }
        .into(),
        _ => return Err(internal_wire_error()),
    };
    Ok(CodecDeterministicCborMapKey {
        key: Some(key),
        __buffa_unknown_fields: Default::default(),
    })
}

fn dag_cbor_value_proto(value: &CborValue) -> Result<CodecDeterministicCborValue, CodecWireError> {
    let value = match value {
        CborValue::Null => CodecDeterministicCborNull {
            __buffa_unknown_fields: Default::default(),
        }
        .into(),
        CborValue::Bool(value) => CodecDeterministicCborBool {
            value: *value,
            __buffa_unknown_fields: Default::default(),
        }
        .into(),
        CborValue::Int(value) => dag_cbor_integer_proto(*value).into(),
        CborValue::String(value) => CodecDeterministicCborText {
            value: try_copy_deterministic_text(value)?,
            __buffa_unknown_fields: Default::default(),
        }
        .into(),
        CborValue::Bytes(value) => CodecDeterministicCborBytes {
            value: try_copy_deterministic_bytes(value)?,
            __buffa_unknown_fields: Default::default(),
        }
        .into(),
        CborValue::Array(values) => {
            let mut proto_values = try_deterministic_vec(values.len())?;
            for value in values {
                proto_values.push(dag_cbor_value_proto(value)?);
            }
            CodecDeterministicCborArray {
                values: proto_values,
                __buffa_unknown_fields: Default::default(),
            }
            .into()
        }
        CborValue::Map(entries) => {
            let mut proto_entries = try_deterministic_vec(entries.len())?;
            for (key, value) in entries {
                proto_entries.push(CodecDeterministicCborMapEntry {
                    key: buffa::MessageField::some(dag_cbor_key_proto(key)?),
                    value: buffa::MessageField::some(dag_cbor_value_proto(value)?),
                    __buffa_unknown_fields: Default::default(),
                });
            }
            CodecDeterministicCborMap {
                entries: proto_entries,
                __buffa_unknown_fields: Default::default(),
            }
            .into()
        }
        _ => return Err(internal_wire_error()),
    };

    Ok(CodecDeterministicCborValue {
        value: Some(value),
        __buffa_unknown_fields: Default::default(),
    })
}

fn dag_cbor_integer_proto(value: i64) -> CodecDeterministicCborInteger {
    let value = if value >= 0 {
        CodecDeterministicCborUnsignedInteger {
            value: value.unsigned_abs(),
            __buffa_unknown_fields: Default::default(),
        }
        .into()
    } else {
        CodecDeterministicCborNegativeInteger {
            value,
            __buffa_unknown_fields: Default::default(),
        }
        .into()
    };
    CodecDeterministicCborInteger {
        value: Some(value),
        __buffa_unknown_fields: Default::default(),
    }
}

fn dag_cbor_key_proto(value: &str) -> Result<CodecDeterministicCborMapKey, CodecWireError> {
    Ok(CodecDeterministicCborMapKey {
        key: Some(
            CodecDeterministicCborText {
                value: try_copy_deterministic_text(value)?,
                __buffa_unknown_fields: Default::default(),
            }
            .into(),
        ),
        __buffa_unknown_fields: Default::default(),
    })
}
