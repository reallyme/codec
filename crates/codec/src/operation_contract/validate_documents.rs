// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

#[derive(Default)]
struct DeterministicProtoLimits {
    nodes: usize,
    aggregate_text_bytes: usize,
    aggregate_byte_string_bytes: usize,
}

impl DeterministicProtoLimits {
    fn add_node(&mut self) -> Result<(), CodecWireError> {
        self.nodes = checked_proto_limit_add(self.nodes, 1)?;
        ensure_proto_limit(self.nodes, MAX_DETERMINISTIC_CBOR_NODES)
    }

    fn add_text(&mut self, len: usize) -> Result<(), CodecWireError> {
        self.aggregate_text_bytes = checked_proto_limit_add(self.aggregate_text_bytes, len)?;
        ensure_proto_limit(
            self.aggregate_text_bytes,
            MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES,
        )
    }

    fn add_bytes(&mut self, len: usize) -> Result<(), CodecWireError> {
        self.aggregate_byte_string_bytes =
            checked_proto_limit_add(self.aggregate_byte_string_bytes, len)?;
        ensure_proto_limit(
            self.aggregate_byte_string_bytes,
            MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
        )
    }
}

fn validate_deterministic_value(
    value: &CodecDeterministicCborValue,
    depth: usize,
    limits: &mut DeterministicProtoLimits,
) -> Result<(), CodecWireError> {
    reject_unknown_fields(&value.__buffa_unknown_fields)?;
    limits.add_node()?;
    match value.value.as_ref() {
        Some(codec_deterministic_cbor_value::Value::NullValue(value)) => {
            reject_unknown_fields(&value.__buffa_unknown_fields)
        }
        Some(codec_deterministic_cbor_value::Value::BoolValue(value)) => {
            reject_unknown_fields(&value.__buffa_unknown_fields)
        }
        Some(codec_deterministic_cbor_value::Value::IntegerValue(value)) => {
            validate_deterministic_integer(value)
        }
        Some(codec_deterministic_cbor_value::Value::TextValue(value)) => {
            reject_unknown_fields(&value.__buffa_unknown_fields)?;
            limits.add_text(value.value.len())
        }
        Some(codec_deterministic_cbor_value::Value::BytesValue(value)) => {
            reject_unknown_fields(&value.__buffa_unknown_fields)?;
            limits.add_bytes(value.value.len())
        }
        Some(codec_deterministic_cbor_value::Value::ArrayValue(value)) => {
            reject_unknown_fields(&value.__buffa_unknown_fields)?;
            ensure_proto_limit(value.values.len(), MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES)?;
            let child_depth = deterministic_proto_child_depth(depth)?;
            for child in &value.values {
                validate_deterministic_value(child, child_depth, limits)?;
            }
            Ok(())
        }
        Some(codec_deterministic_cbor_value::Value::MapValue(value)) => {
            reject_unknown_fields(&value.__buffa_unknown_fields)?;
            ensure_proto_limit(
                value.entries.len(),
                MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES,
            )?;
            let child_depth = deterministic_proto_child_depth(depth)?;
            for entry in &value.entries {
                reject_unknown_fields(&entry.__buffa_unknown_fields)?;
                validate_deterministic_key(&entry.key, limits)?;
                let Some(entry_value) = entry.value.as_option() else {
                    return Err(malformed_request_wire_error());
                };
                validate_deterministic_value(entry_value, child_depth, limits)?;
            }
            Ok(())
        }
        None => Err(malformed_request_wire_error()),
    }
}

fn validate_dag_cbor_proto_value(
    value: &CodecDeterministicCborValue,
    depth: usize,
    limits: &mut DeterministicProtoLimits,
) -> Result<(), CodecWireError> {
    reject_unknown_fields(&value.__buffa_unknown_fields)?;
    limits.add_node()?;
    match value.value.as_ref() {
        Some(codec_deterministic_cbor_value::Value::NullValue(value)) => {
            reject_unknown_fields(&value.__buffa_unknown_fields)
        }
        Some(codec_deterministic_cbor_value::Value::BoolValue(value)) => {
            reject_unknown_fields(&value.__buffa_unknown_fields)
        }
        Some(codec_deterministic_cbor_value::Value::IntegerValue(value)) => {
            validate_dag_cbor_proto_integer(value)
        }
        Some(codec_deterministic_cbor_value::Value::TextValue(value)) => {
            reject_unknown_fields(&value.__buffa_unknown_fields)?;
            limits.add_text(value.value.len())
        }
        Some(codec_deterministic_cbor_value::Value::BytesValue(value)) => {
            reject_unknown_fields(&value.__buffa_unknown_fields)?;
            limits.add_bytes(value.value.len())
        }
        Some(codec_deterministic_cbor_value::Value::ArrayValue(value)) => {
            reject_unknown_fields(&value.__buffa_unknown_fields)?;
            ensure_proto_limit(value.values.len(), MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES)?;
            let child_depth = deterministic_proto_child_depth(depth)?;
            for child in &value.values {
                validate_dag_cbor_proto_value(child, child_depth, limits)?;
            }
            Ok(())
        }
        Some(codec_deterministic_cbor_value::Value::MapValue(value)) => {
            reject_unknown_fields(&value.__buffa_unknown_fields)?;
            ensure_proto_limit(
                value.entries.len(),
                MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES,
            )?;
            let child_depth = deterministic_proto_child_depth(depth)?;
            for entry in &value.entries {
                reject_unknown_fields(&entry.__buffa_unknown_fields)?;
                validate_dag_cbor_proto_key(&entry.key, limits)?;
                let Some(entry_value) = entry.value.as_option() else {
                    return Err(malformed_request_wire_error());
                };
                validate_dag_cbor_proto_value(entry_value, child_depth, limits)?;
            }
            Ok(())
        }
        None => Err(malformed_request_wire_error()),
    }
}

fn validate_dag_cbor_proto_integer(
    value: &CodecDeterministicCborInteger,
) -> Result<(), CodecWireError> {
    reject_unknown_fields(&value.__buffa_unknown_fields)?;
    match value.value.as_ref() {
        Some(codec_deterministic_cbor_integer::Value::UnsignedValue(value)) => {
            reject_unknown_fields(&value.__buffa_unknown_fields)?;
            if i64::try_from(value.value).is_ok() {
                Ok(())
            } else {
                Err(malformed_request_wire_error())
            }
        }
        Some(codec_deterministic_cbor_integer::Value::NegativeValue(value)) => {
            reject_unknown_fields(&value.__buffa_unknown_fields)?;
            if value.value < 0 {
                Ok(())
            } else {
                Err(malformed_request_wire_error())
            }
        }
        None => Err(malformed_request_wire_error()),
    }
}

fn validate_dag_cbor_proto_key<P: buffa::ProtoBox<CodecDeterministicCborMapKey>>(
    field: &buffa::MessageField<CodecDeterministicCborMapKey, P>,
    limits: &mut DeterministicProtoLimits,
) -> Result<(), CodecWireError> {
    let Some(value) = field.as_option() else {
        return Err(malformed_request_wire_error());
    };
    reject_unknown_fields(&value.__buffa_unknown_fields)?;
    limits.add_node()?;
    match value.key.as_ref() {
        Some(codec_deterministic_cbor_map_key::Key::TextKey(value)) => {
            reject_unknown_fields(&value.__buffa_unknown_fields)?;
            limits.add_text(value.value.len())
        }
        Some(codec_deterministic_cbor_map_key::Key::IntegerKey(value)) => {
            reject_unknown_fields(&value.__buffa_unknown_fields)?;
            Err(malformed_request_wire_error())
        }
        None => Err(malformed_request_wire_error()),
    }
}

fn validate_deterministic_integer(
    value: &CodecDeterministicCborInteger,
) -> Result<(), CodecWireError> {
    reject_unknown_fields(&value.__buffa_unknown_fields)?;
    match value.value.as_ref() {
        Some(codec_deterministic_cbor_integer::Value::UnsignedValue(value)) => {
            reject_unknown_fields(&value.__buffa_unknown_fields)
        }
        Some(codec_deterministic_cbor_integer::Value::NegativeValue(value)) => {
            reject_unknown_fields(&value.__buffa_unknown_fields)?;
            if value.value < 0 {
                Ok(())
            } else {
                Err(malformed_request_wire_error())
            }
        }
        None => Err(malformed_request_wire_error()),
    }
}

fn validate_deterministic_key<P: buffa::ProtoBox<CodecDeterministicCborMapKey>>(
    field: &buffa::MessageField<CodecDeterministicCborMapKey, P>,
    limits: &mut DeterministicProtoLimits,
) -> Result<(), CodecWireError> {
    let Some(value) = field.as_option() else {
        return Err(malformed_request_wire_error());
    };
    reject_unknown_fields(&value.__buffa_unknown_fields)?;
    limits.add_node()?;
    match value.key.as_ref() {
        Some(codec_deterministic_cbor_map_key::Key::IntegerKey(value)) => {
            validate_deterministic_integer(value)
        }
        Some(codec_deterministic_cbor_map_key::Key::TextKey(value)) => {
            reject_unknown_fields(&value.__buffa_unknown_fields)?;
            limits.add_text(value.value.len())
        }
        None => Err(malformed_request_wire_error()),
    }
}

fn deterministic_proto_child_depth(depth: usize) -> Result<usize, CodecWireError> {
    let child_depth = checked_proto_limit_add(depth, 1)?;
    ensure_proto_limit(child_depth, MAX_DETERMINISTIC_CBOR_NESTING_DEPTH)?;
    Ok(child_depth)
}

fn checked_proto_limit_add(left: usize, right: usize) -> Result<usize, CodecWireError> {
    left.checked_add(right)
        .ok_or_else(resource_limit_wire_error)
}

fn ensure_proto_limit(value: usize, maximum: usize) -> Result<(), CodecWireError> {
    if value <= maximum {
        Ok(())
    } else {
        Err(resource_limit_wire_error())
    }
}

fn resource_limit_wire_error() -> CodecWireError {
    wire_error(
        CodecWireErrorBranch::Boundary,
        CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED,
    )
}
