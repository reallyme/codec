// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import Foundation
import SwiftProtobuf

private func bytes(_ text: String, maxFfiInputLength: Int) throws(ReallyMeCodecError) -> [UInt8] {
    try requireBoundaryAggregate([text.utf8.count], maxFfiInputLength: maxFfiInputLength)
    return Array(text.utf8)
}

/// Limits the lifetime of a mutable UTF-8 copy created from an immutable
/// Swift string. The caller still owns the original string, but the codec must
/// wipe every additional buffer it creates on both success and failure paths.
func withTextBytes<T>(
    _ text: String,
    maxFfiInputLength: Int,
    _ body: ([UInt8]) throws(ReallyMeCodecError) -> T
) throws(ReallyMeCodecError) -> T {
    var encoded = try bytes(text, maxFfiInputLength: maxFfiInputLength)
    defer {
        ReallyMeCodecMemory.clearOwned(&encoded)
    }
    return try body(encoded)
}

func withTextBytes<T>(
    _ first: String,
    _ second: String,
    maxFfiInputLength: Int,
    _ body: ([UInt8], [UInt8]) throws(ReallyMeCodecError) -> T
) throws(ReallyMeCodecError) -> T {
    try withTextBytes(first, maxFfiInputLength: maxFfiInputLength) { (firstBytes) throws(ReallyMeCodecError) in
        try withTextBytes(second, maxFfiInputLength: maxFfiInputLength) { (secondBytes) throws(ReallyMeCodecError) in
            try body(firstBytes, secondBytes)
        }
    }
}

func withTextBytes<T>(
    _ first: String,
    _ second: String,
    _ third: String,
    maxFfiInputLength: Int,
    _ body: ([UInt8], [UInt8], [UInt8]) throws(ReallyMeCodecError) -> T
) throws(ReallyMeCodecError) -> T {
    try withTextBytes(first, second, maxFfiInputLength: maxFfiInputLength) {
        (firstBytes, secondBytes) throws(ReallyMeCodecError) in
        try withTextBytes(third, maxFfiInputLength: maxFfiInputLength) { (thirdBytes) throws(ReallyMeCodecError) in
            try body(firstBytes, secondBytes, thirdBytes)
        }
    }
}

/// Limits the lifetime of an SDK-created serialized request. The consuming
/// parameter transfers the fresh array owner into this scope, and the closure
/// receives only a borrow that ends before the non-elidable wipe runs.
func withOwnedBytes<T>(
    _ bytes: consuming [UInt8],
    _ body: ([UInt8]) throws(ReallyMeCodecError) -> T
) throws(ReallyMeCodecError) -> T {
    var ownedBytes = consume bytes
    defer {
        ReallyMeCodecMemory.clearOwned(&ownedBytes)
    }
    return try body(ownedBytes)
}

func text(_ bytes: consuming [UInt8]) throws(ReallyMeCodecError) -> String {
    var ownedBytes = consume bytes
    defer {
        ReallyMeCodecMemory.clearOwned(&ownedBytes)
    }
    guard let value = String(bytes: ownedBytes, encoding: .utf8) else {
        throw ReallyMeCodecError.providerFailure
    }
    return value
}

func requireBoundaryAggregate(_ lengths: [Int], maxFfiInputLength: Int) throws(ReallyMeCodecError) {
    guard isBoundaryAggregateValid(lengths, maxFfiInputLength: maxFfiInputLength) else {
        throw ReallyMeCodecError.invalidInput
    }
}

private func isBoundaryAggregateValid(_ lengths: [Int], maxFfiInputLength: Int) -> Bool {
    var aggregate = 0
    for length in lengths {
        let (next, overflow) = aggregate.addingReportingOverflow(length)
        guard !overflow, next <= maxFfiInputLength else {
            return false
        }
        aggregate = next
    }
    return true
}
