// SPDX-FileCopyrightText: 2026 ReallyMe LLC
//
// SPDX-License-Identifier: MIT OR Apache-2.0

// The schema currently has three PemLabel values. More entries cannot add a
// capability, but a packed field can allocate millions of generated enum
// wrappers before the semantic request is examined.
const MAX_PEM_ALLOWED_LABELS: usize = 3;
const PEM_DECODE_OPERATION_FIELD: u64 = 4000;
const PEM_DECODE_OPTIONS_FIELD: u64 = 2;
const PEM_ALLOWED_LABELS_FIELD: u64 = 1;
const OPERATION_FIELD_NUMBERS: [u64; 11] = [
    1000, 1001, 1002, 2000, 3000, 3001, 3002, 4000, 4001, 5000, 5001,
];
const WIRE_VARINT: u64 = 0;
const WIRE_LENGTH_DELIMITED: u64 = 2;

fn pem_label_limit_error() -> CodecWireError {
    wire_error(
        CodecWireErrorBranch::Boundary,
        CodecErrorReason::CODEC_ERROR_REASON_BOUNDARY_RESOURCE_LIMIT_EXCEEDED,
    )
}

struct WireField<'a> {
    number: u64,
    wire_type: u64,
    bytes: &'a [u8],
}

fn read_wire_varint(bytes: &[u8], offset: &mut usize) -> Result<u64, CodecWireError> {
    let mut value = 0_u64;
    for index in 0_u32..10 {
        let byte = *bytes.get(*offset).ok_or_else(malformed_request_wire_error)?;
        *offset = offset
            .checked_add(1)
            .ok_or_else(malformed_request_wire_error)?;
        let bits = u64::from(byte & 0x7f);
        if index == 9 && bits > 1 {
            return Err(malformed_request_wire_error());
        }
        let shift = index
            .checked_mul(7)
            .ok_or_else(malformed_request_wire_error)?;
        value |= bits
            .checked_shl(shift)
            .ok_or_else(malformed_request_wire_error)?;
        if byte & 0x80 == 0 {
            return Ok(value);
        }
    }
    Err(malformed_request_wire_error())
}

fn read_wire_field<'a>(bytes: &'a [u8], offset: &mut usize) -> Result<WireField<'a>, CodecWireError> {
    let tag = read_wire_varint(bytes, offset)?;
    let number = tag >> 3;
    let wire_type = tag & 7;
    if number == 0 {
        return Err(malformed_request_wire_error());
    }
    let payload = match wire_type {
        WIRE_VARINT => {
            let _ = read_wire_varint(bytes, offset)?;
            &bytes[0..0]
        }
        1 | 5 => {
            let width = if wire_type == 1 { 8_usize } else { 4_usize };
            let end = offset
                .checked_add(width)
                .ok_or_else(malformed_request_wire_error)?;
            let slice = bytes
                .get(*offset..end)
                .ok_or_else(malformed_request_wire_error)?;
            *offset = end;
            slice
        }
        WIRE_LENGTH_DELIMITED => {
            let length = usize::try_from(read_wire_varint(bytes, offset)?)
                .map_err(|_| malformed_request_wire_error())?;
            let end = offset
                .checked_add(length)
                .ok_or_else(malformed_request_wire_error)?;
            let slice = bytes
                .get(*offset..end)
                .ok_or_else(malformed_request_wire_error)?;
            *offset = end;
            slice
        }
        _ => return Err(malformed_request_wire_error()),
    };
    Ok(WireField { number, wire_type, bytes: payload })
}

fn count_pem_labels(bytes: &[u8], count: &mut usize) -> Result<(), CodecWireError> {
    let mut offset = 0_usize;
    while offset < bytes.len() {
        let field = read_wire_field(bytes, &mut offset)?;
        if field.number != PEM_ALLOWED_LABELS_FIELD {
            continue;
        }
        match field.wire_type {
            WIRE_VARINT => {
                *count = count.checked_add(1).ok_or_else(pem_label_limit_error)?;
            }
            WIRE_LENGTH_DELIMITED => {
                let mut packed_offset = 0_usize;
                while packed_offset < field.bytes.len() {
                    let _ = read_wire_varint(field.bytes, &mut packed_offset)?;
                    *count = count.checked_add(1).ok_or_else(pem_label_limit_error)?;
                    if *count > MAX_PEM_ALLOWED_LABELS {
                        return Err(pem_label_limit_error());
                    }
                }
            }
            _ => return Err(malformed_request_wire_error()),
        }
        if *count > MAX_PEM_ALLOWED_LABELS {
            return Err(pem_label_limit_error());
        }
    }
    Ok(())
}

fn preflight_binary_request(bytes: &[u8]) -> Result<(), CodecWireError> {
    if bytes.len() > MAX_CODEC_PROTO_MESSAGE_BYTES {
        return Err(pem_label_limit_error());
    }
    let mut offset = 0_usize;
    let mut label_count = 0_usize;
    let mut seen_operation = false;
    while offset < bytes.len() {
        let operation = read_wire_field(bytes, &mut offset)?;
        if !OPERATION_FIELD_NUMBERS.contains(&operation.number) {
            continue;
        }
        if seen_operation || operation.wire_type != WIRE_LENGTH_DELIMITED {
            return Err(malformed_request_wire_error());
        }
        seen_operation = true;
        let mut request_offset = 0_usize;
        let mut seen_request_fields = 0_u64;
        while request_offset < operation.bytes.len() {
            let request_field = read_wire_field(operation.bytes, &mut request_offset)?;
            if request_field.number < 64 {
                let shift = u32::try_from(request_field.number)
                    .map_err(|_| malformed_request_wire_error())?;
                let bit = 1_u64
                    .checked_shl(shift)
                    .ok_or_else(malformed_request_wire_error)?;
                if seen_request_fields & bit != 0 {
                    // Protobuf merge semantics can replace a secret-bearing
                    // field and leave the first allocation unwiped. This
                    // contract uses no repeated request fields.
                    return Err(malformed_request_wire_error());
                }
                seen_request_fields |= bit;
            }
            if operation.number == PEM_DECODE_OPERATION_FIELD
                && request_field.number == PEM_DECODE_OPTIONS_FIELD
                && request_field.wire_type == WIRE_LENGTH_DELIMITED
            {
                count_pem_labels(request_field.bytes, &mut label_count)?;
            }
        }
    }
    Ok(())
}
