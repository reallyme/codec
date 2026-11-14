// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec

internal class DeterministicCborValidationState {
    var nodes: Int = 0
    var textBytes: Int = 0
    var byteStringBytes: Int = 0
}

internal fun validateDeterministicCborValue(value: ReallyMeDeterministicCborValue) {
    val state = DeterministicCborValidationState()
    validateDeterministicCborValue(value, 0, state)
}

internal fun validateDeterministicCborValue(
    value: ReallyMeDeterministicCborValue,
    depth: Int,
    state: DeterministicCborValidationState,
) {
    state.nodes =
        addDeterministicCborCount(
            state.nodes,
            1,
            MAX_DETERMINISTIC_CBOR_NODES,
        )
    when (value) {
        ReallyMeDeterministicCborValue.Null,
        is ReallyMeDeterministicCborValue.Bool,
        is ReallyMeDeterministicCborValue.Integer,
        -> Unit
        is ReallyMeDeterministicCborValue.Text -> {
            state.textBytes =
                addDeterministicCborCount(
                    state.textBytes,
                    deterministicCborUtf8Length(value.value),
                    MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES,
                )
        }
        is ReallyMeDeterministicCborValue.Bytes -> {
            state.byteStringBytes =
                addDeterministicCborCount(
                    state.byteStringBytes,
                    value.borrowedBytes().size,
                    MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
                )
        }
        is ReallyMeDeterministicCborValue.Array -> {
            validateDeterministicCborContainer(value.values.size)
            val childDepth = deterministicCborChildDepth(depth)
            for (child in value.values) {
                validateDeterministicCborValue(child, childDepth, state)
            }
        }
        is ReallyMeDeterministicCborValue.Map -> {
            validateDeterministicCborContainer(value.entries.size)
            val childDepth = deterministicCborChildDepth(depth)
            for (entry in value.entries) {
                state.nodes =
                    addDeterministicCborCount(
                        state.nodes,
                        1,
                        MAX_DETERMINISTIC_CBOR_NODES,
                    )
                val key = entry.key
                if (key is ReallyMeDeterministicCborMapKey.Text) {
                    state.textBytes =
                        addDeterministicCborCount(
                            state.textBytes,
                            deterministicCborUtf8Length(key.value),
                            MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES,
                        )
                }
                validateDeterministicCborValue(entry.value, childDepth, state)
            }
        }
    }
}

private fun validateDeterministicCborContainer(count: Int) {
    if (count > MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES) {
        throw ReallyMeCodecException.InvalidInput()
    }
}

private fun deterministicCborChildDepth(depth: Int): Int {
    if (depth >= MAX_DETERMINISTIC_CBOR_NESTING_DEPTH) {
        throw ReallyMeCodecException.InvalidInput()
    }
    return depth + 1
}

private fun addDeterministicCborCount(
    current: Int,
    increment: Int,
    maximum: Int,
): Int {
    if (increment < 0 || current > maximum - increment) {
        throw ReallyMeCodecException.InvalidInput()
    }
    return current + increment
}

internal fun deterministicCborUtf8Length(text: String): Int {
    var length = 0
    var index = 0
    while (index < text.length) {
        val character = text[index]
        val increment =
            when {
                character.code <= 0x7f -> 1
                character.code <= 0x7ff -> 2
                Character.isHighSurrogate(character) &&
                    index + 1 < text.length &&
                    Character.isLowSurrogate(text[index + 1]) -> {
                    index += 1
                    4
                }
                Character.isSurrogate(character) ->
                    throw ReallyMeCodecException.InvalidInput()
                else -> 3
            }
        length =
            addDeterministicCborCount(
                length,
                increment,
                MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES,
            )
        index += 1
    }
    return length
}
