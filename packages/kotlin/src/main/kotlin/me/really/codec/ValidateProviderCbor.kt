// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec

import me.really.codec.v1.CodecDeterministicCborInteger
import me.really.codec.v1.CodecDeterministicCborMapEntry
import me.really.codec.v1.CodecDeterministicCborMapKey
import me.really.codec.v1.CodecDeterministicCborValue

/**
 * Validates the generated provider tree before creating a second managed
 * owner graph. This is boundary-shape and resource validation only; Rust
 * remains authoritative for deterministic-CBOR semantics and bytes.
 */
internal fun validateProviderCborTree(value: CodecDeterministicCborValue) {
    val state = DeterministicCborValidationState()
    validateProviderDeterministicCborValue(value, 0, state)
}

private fun validateProviderDeterministicCborValue(
    value: CodecDeterministicCborValue,
    depth: Int,
    state: DeterministicCborValidationState,
) {
    requireNoProviderUnknownFields(value.reallyMeHasUnknownFieldsForValidation())
    state.nodes =
        addProviderDeterministicCborCount(
            state.nodes,
            1,
            MAX_DETERMINISTIC_CBOR_NODES,
        )
    when (value.valueCase) {
        CodecDeterministicCborValue.ValueCase.NULL_VALUE,
        ->
            requireNoProviderUnknownFields(
                value.nullValue.reallyMeHasUnknownFieldsForValidation(),
            )
        CodecDeterministicCborValue.ValueCase.BOOL_VALUE ->
            requireNoProviderUnknownFields(
                value.boolValue.reallyMeHasUnknownFieldsForValidation(),
            )
        CodecDeterministicCborValue.ValueCase.INTEGER_VALUE ->
            validateProviderDeterministicCborInteger(value.integerValue)
        CodecDeterministicCborValue.ValueCase.TEXT_VALUE -> {
            requireNoProviderUnknownFields(
                value.textValue.reallyMeHasUnknownFieldsForValidation(),
            )
            state.textBytes =
                addProviderDeterministicCborCount(
                    state.textBytes,
                    providerDeterministicCborUtf8Length(value.textValue.value),
                    MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES,
                )
        }
        CodecDeterministicCborValue.ValueCase.BYTES_VALUE -> {
            requireNoProviderUnknownFields(
                value.bytesValue.reallyMeHasUnknownFieldsForValidation(),
            )
            state.byteStringBytes =
                addProviderDeterministicCborCount(
                    state.byteStringBytes,
                    value.bytesValue.value.size(),
                    MAX_DETERMINISTIC_CBOR_AGGREGATE_BYTE_STRING_BYTES,
                )
        }
        CodecDeterministicCborValue.ValueCase.ARRAY_VALUE -> {
            requireNoProviderUnknownFields(
                value.arrayValue.reallyMeHasUnknownFieldsForValidation(),
            )
            validateProviderDeterministicCborContainer(value.arrayValue.valuesCount)
            val childDepth = providerDeterministicCborChildDepth(depth)
            for (child in value.arrayValue.valuesList) {
                validateProviderDeterministicCborValue(child, childDepth, state)
            }
        }
        CodecDeterministicCborValue.ValueCase.MAP_VALUE -> {
            requireNoProviderUnknownFields(
                value.mapValue.reallyMeHasUnknownFieldsForValidation(),
            )
            validateProviderDeterministicCborContainer(value.mapValue.entriesCount)
            val childDepth = providerDeterministicCborChildDepth(depth)
            for (entry in value.mapValue.entriesList) {
                requireNoProviderUnknownFields(
                    entry.reallyMeHasUnknownFieldsForValidation(),
                )
                if (!entry.hasKey() || !entry.hasValue()) {
                    throw ReallyMeCodecException.ProviderFailure()
                }
                state.nodes =
                    addProviderDeterministicCborCount(
                        state.nodes,
                        1,
                        MAX_DETERMINISTIC_CBOR_NODES,
                    )
                validateProviderDeterministicCborKey(entry.key, state)
            }
            rejectDuplicateProviderDeterministicCborMapKeys(
                value.mapValue.entriesList,
            )
            for (entry in value.mapValue.entriesList) {
                validateProviderDeterministicCborValue(entry.value, childDepth, state)
            }
        }
        CodecDeterministicCborValue.ValueCase.VALUE_NOT_SET,
        null,
        -> throw ReallyMeCodecException.ProviderFailure()
    }
}

private fun rejectDuplicateProviderDeterministicCborMapKeys(entries: List<CodecDeterministicCborMapEntry>) {
    val keys = ArrayList<CodecDeterministicCborMapKey>(entries.size)
    for (entry in entries) {
        keys.add(entry.key)
    }
    keys.sortWith(::compareProviderDeterministicCborMapKeys)
    for (index in 1 until keys.size) {
        if (compareProviderDeterministicCborMapKeys(keys[index - 1], keys[index]) == 0) {
            throw ReallyMeCodecException.ProviderFailure()
        }
    }
}

private fun compareProviderDeterministicCborMapKeys(
    left: CodecDeterministicCborMapKey,
    right: CodecDeterministicCborMapKey,
): Int {
    val leftRank = providerDeterministicCborMapKeyRank(left.keyCase)
    val rightRank = providerDeterministicCborMapKeyRank(right.keyCase)
    if (leftRank != rightRank) {
        return leftRank.compareTo(rightRank)
    }
    return when (left.keyCase) {
        CodecDeterministicCborMapKey.KeyCase.INTEGER_KEY ->
            compareProviderDeterministicCborIntegers(left.integerKey, right.integerKey)
        CodecDeterministicCborMapKey.KeyCase.TEXT_KEY ->
            left.textKey.value.compareTo(right.textKey.value)
        CodecDeterministicCborMapKey.KeyCase.KEY_NOT_SET,
        null,
        -> 0
    }
}

private fun providerDeterministicCborMapKeyRank(keyCase: CodecDeterministicCborMapKey.KeyCase?): Int =
    when (keyCase) {
        CodecDeterministicCborMapKey.KeyCase.INTEGER_KEY -> 0
        CodecDeterministicCborMapKey.KeyCase.TEXT_KEY -> 1
        CodecDeterministicCborMapKey.KeyCase.KEY_NOT_SET,
        null,
        -> 2
    }

private fun compareProviderDeterministicCborIntegers(
    left: CodecDeterministicCborInteger,
    right: CodecDeterministicCborInteger,
): Int {
    val leftRank = providerDeterministicCborIntegerRank(left.valueCase)
    val rightRank = providerDeterministicCborIntegerRank(right.valueCase)
    if (leftRank != rightRank) {
        return leftRank.compareTo(rightRank)
    }
    return when (left.valueCase) {
        CodecDeterministicCborInteger.ValueCase.UNSIGNED_VALUE ->
            left.unsignedValue.value
                .toULong()
                .compareTo(right.unsignedValue.value.toULong())
        CodecDeterministicCborInteger.ValueCase.NEGATIVE_VALUE ->
            left.negativeValue.value.compareTo(right.negativeValue.value)
        CodecDeterministicCborInteger.ValueCase.VALUE_NOT_SET,
        null,
        -> 0
    }
}

private fun providerDeterministicCborIntegerRank(valueCase: CodecDeterministicCborInteger.ValueCase?): Int =
    when (valueCase) {
        CodecDeterministicCborInteger.ValueCase.UNSIGNED_VALUE -> 0
        CodecDeterministicCborInteger.ValueCase.NEGATIVE_VALUE -> 1
        CodecDeterministicCborInteger.ValueCase.VALUE_NOT_SET,
        null,
        -> 2
    }

private fun validateProviderDeterministicCborInteger(integer: CodecDeterministicCborInteger) {
    requireNoProviderUnknownFields(integer.reallyMeHasUnknownFieldsForValidation())
    when (integer.valueCase) {
        CodecDeterministicCborInteger.ValueCase.UNSIGNED_VALUE ->
            requireNoProviderUnknownFields(
                integer.unsignedValue.reallyMeHasUnknownFieldsForValidation(),
            )
        CodecDeterministicCborInteger.ValueCase.NEGATIVE_VALUE -> {
            requireNoProviderUnknownFields(
                integer.negativeValue.reallyMeHasUnknownFieldsForValidation(),
            )
            if (integer.negativeValue.value >= 0) {
                throw ReallyMeCodecException.ProviderFailure()
            }
        }
        CodecDeterministicCborInteger.ValueCase.VALUE_NOT_SET,
        null,
        -> throw ReallyMeCodecException.ProviderFailure()
    }
}

private fun validateProviderDeterministicCborKey(
    key: CodecDeterministicCborMapKey,
    state: DeterministicCborValidationState,
) {
    requireNoProviderUnknownFields(key.reallyMeHasUnknownFieldsForValidation())
    when (key.keyCase) {
        CodecDeterministicCborMapKey.KeyCase.INTEGER_KEY ->
            validateProviderDeterministicCborInteger(key.integerKey)
        CodecDeterministicCborMapKey.KeyCase.TEXT_KEY -> {
            requireNoProviderUnknownFields(
                key.textKey.reallyMeHasUnknownFieldsForValidation(),
            )
            state.textBytes =
                addProviderDeterministicCborCount(
                    state.textBytes,
                    providerDeterministicCborUtf8Length(key.textKey.value),
                    MAX_DETERMINISTIC_CBOR_AGGREGATE_TEXT_BYTES,
                )
        }
        CodecDeterministicCborMapKey.KeyCase.KEY_NOT_SET,
        null,
        -> throw ReallyMeCodecException.ProviderFailure()
    }
}

private fun requireNoProviderUnknownFields(hasUnknownFields: Boolean) {
    if (hasUnknownFields) {
        throw ReallyMeCodecException.ProviderFailure()
    }
}

private fun validateProviderDeterministicCborContainer(count: Int) {
    if (count > MAX_DETERMINISTIC_CBOR_CONTAINER_ENTRIES) {
        throw ReallyMeCodecException.ProviderFailure()
    }
}

private fun providerDeterministicCborChildDepth(depth: Int): Int {
    if (depth >= MAX_DETERMINISTIC_CBOR_NESTING_DEPTH) {
        throw ReallyMeCodecException.ProviderFailure()
    }
    return depth + 1
}

private fun addProviderDeterministicCborCount(
    current: Int,
    increment: Int,
    maximum: Int,
): Int {
    if (increment < 0 || current > maximum - increment) {
        throw ReallyMeCodecException.ProviderFailure()
    }
    return current + increment
}

private fun providerDeterministicCborUtf8Length(text: String): Int =
    try {
        deterministicCborUtf8Length(text)
    } catch (_: ReallyMeCodecException.InvalidInput) {
        throw ReallyMeCodecException.ProviderFailure()
    }
