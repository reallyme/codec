// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import Foundation
import SwiftProtobuf

let maxDeterministicCborNestingDepth = 64
let maxDeterministicCborNodes = 65_536
let maxDeterministicCborContainerEntries = 16_384
let maxDeterministicCborAggregateTextBytes = 1_048_576
let maxDeterministicCborAggregateByteStringBytes = 1_048_576
// The recursive protobuf representation can be materially larger than the
// canonical CBOR it carries. This transport limit is derived from the same
// semantic budgets as the Rust protobuf boundary so every valid semantic
// value remains reachable without making the public scalar FFI cap larger.
private let maxCodecProtoStructuralBytesPerDeterministicCborNode = 128
private let maxCodecProtoFixedDeterministicCborOperationBytes = 4_096
let maxDeterministicCborProtoMessageBytes =
    maxDeterministicCborAggregateTextBytes
    + maxDeterministicCborAggregateByteStringBytes
    + (maxDeterministicCborNodes * maxCodecProtoStructuralBytesPerDeterministicCborNode)
    + maxCodecProtoFixedDeterministicCborOperationBytes
// One semantic map level expands to Value -> Map -> MapEntry. Five outer/key
// wrappers cover the deepest generated request/result path. SwiftProtobuf's
// default of 100 cannot carry the documented semantic depth of 64. The fully
// discriminated response adds OperationResponse and OperationResult outside
// the operation-specific result; the generated limit includes that path.
let maxDeterministicCborProtoMessageDepth = 198

struct DeterministicCborValidationState {
    var nodes = 0
    var textBytes = 0
    var byteStringBytes = 0
}

func validateDeterministicCborValue(
    _ value: ReallyMeDeterministicCborValue
) throws(ReallyMeCodecError) {
    var state = DeterministicCborValidationState()
    try validateDeterministicCborValue(value, depth: 0, state: &state)
}

private func validateDeterministicCborValue(
    _ value: ReallyMeDeterministicCborValue,
    depth: Int,
    state: inout DeterministicCborValidationState
) throws(ReallyMeCodecError) {
    try addDeterministicCborCount(&state.nodes, 1, maximum: maxDeterministicCborNodes)
    switch value {
    case .null, .bool, .integer:
        return
    case .text(let value):
        try addDeterministicCborTextBytes(value, state: &state)
    case .bytes(let value):
        try addDeterministicCborCount(
            &state.byteStringBytes,
            value.count,
            maximum: maxDeterministicCborAggregateByteStringBytes
        )
    case .array(let values):
        try validateDeterministicCborContainer(values.count)
        let childDepth = try deterministicCborChildDepth(depth)
        for child in values {
            try validateDeterministicCborValue(child, depth: childDepth, state: &state)
        }
    case .map(let entries):
        try validateDeterministicCborContainer(entries.count)
        let childDepth = try deterministicCborChildDepth(depth)
        for entry in entries {
            try addDeterministicCborCount(
                &state.nodes,
                1,
                maximum: maxDeterministicCborNodes
            )
            if case .text(let key) = entry.key {
                try addDeterministicCborTextBytes(key, state: &state)
            }
            try validateDeterministicCborValue(
                entry.value,
                depth: childDepth,
                state: &state
            )
        }
    }
}

private func addDeterministicCborTextBytes(
    _ text: String,
    state: inout DeterministicCborValidationState
) throws(ReallyMeCodecError) {
    let (remaining, overflow) =
        maxDeterministicCborAggregateTextBytes
        .subtractingReportingOverflow(state.textBytes)
    guard !overflow else {
        throw ReallyMeCodecError.invalidInput
    }
    let increment = try boundedDeterministicCborUtf8Length(
        text,
        maximum: remaining,
        error: .invalidInput
    )
    try addDeterministicCborCount(
        &state.textBytes,
        increment,
        maximum: maxDeterministicCborAggregateTextBytes
    )
}

private func validateDeterministicCborContainer(_ count: Int) throws(ReallyMeCodecError) {
    guard count <= maxDeterministicCborContainerEntries else {
        throw ReallyMeCodecError.invalidInput
    }
}

private func deterministicCborChildDepth(_ depth: Int) throws(ReallyMeCodecError) -> Int {
    let (childDepth, overflow) = depth.addingReportingOverflow(1)
    guard !overflow, childDepth <= maxDeterministicCborNestingDepth else {
        throw ReallyMeCodecError.invalidInput
    }
    return childDepth
}

private func addDeterministicCborCount(
    _ count: inout Int,
    _ increment: Int,
    maximum: Int
) throws(ReallyMeCodecError) {
    let (next, overflow) = count.addingReportingOverflow(increment)
    guard !overflow, next <= maximum else {
        throw ReallyMeCodecError.invalidInput
    }
    count = next
}

/// Validates a generated provider tree before creating a second managed owner
/// graph. This is boundary-shape and resource validation only; Rust remains
/// authoritative for deterministic-CBOR semantics and canonical bytes.
