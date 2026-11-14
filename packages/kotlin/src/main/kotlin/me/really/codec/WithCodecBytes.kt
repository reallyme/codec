// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec

import com.google.protobuf.ByteString
import com.google.protobuf.UnsafeByteOperations
import java.nio.ByteBuffer
import java.nio.charset.CharacterCodingException
import java.nio.charset.CodingErrorAction

internal fun borrowedByteString(bytes: ByteArray): ByteString = UnsafeByteOperations.unsafeWrap(bytes)

internal fun bytes(text: String): ByteArray {
    utf8Length(text)
    val encoded = text.toByteArray(Charsets.UTF_8)
    if (encoded.size > ffiInputLimit()) {
        encoded.fill(0)
        throw ReallyMeCodecException.InvalidInput()
    }
    return encoded
}

/**
 * Limits the lifetime of a mutable UTF-8 copy created from an immutable
 * JVM string. The caller retains the original string, but every additional
 * buffer created by this facade is wiped on success and failure paths.
 */
internal fun <T> withTextBytes(
    text: String,
    action: (ByteArray) -> T,
): T {
    val encoded = bytes(text)
    return try {
        action(encoded)
    } finally {
        encoded.fill(0)
    }
}

internal fun <T> withTextBytes(
    first: String,
    second: String,
    action: (ByteArray, ByteArray) -> T,
): T =
    withTextBytes(first) { firstBytes ->
        withTextBytes(second) { secondBytes ->
            action(firstBytes, secondBytes)
        }
    }

internal fun <T> withTextBytes(
    first: String,
    second: String,
    third: String,
    action: (ByteArray, ByteArray, ByteArray) -> T,
): T =
    withTextBytes(first, second) { firstBytes, secondBytes ->
        withTextBytes(third) { thirdBytes ->
            action(firstBytes, secondBytes, thirdBytes)
        }
    }

internal fun utf8Length(text: String): Int {
    val maximum = ffiInputLimit()
    if (text.length > maximum) {
        throw ReallyMeCodecException.InvalidInput()
    }
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
        if (length > maximum - increment) {
            throw ReallyMeCodecException.InvalidInput()
        }
        length += increment
        index += 1
    }
    return length
}

internal fun requireBoundaryAggregate(vararg lengths: Int) {
    val maximum = ffiInputLimit()
    var aggregate = 0L
    for (length in lengths) {
        aggregate += length.toLong()
        if (aggregate > maximum.toLong()) {
            throw ReallyMeCodecException.InvalidInput()
        }
    }
}

private fun ffiInputLimit(): Int = ReallyMeCodecRustNativeProvider.ffiInputLimit()

internal fun text(bytes: ByteArray): String {
    val decoder =
        Charsets.UTF_8
            .newDecoder()
            .onMalformedInput(CodingErrorAction.REPORT)
            .onUnmappableCharacter(CodingErrorAction.REPORT)
    return try {
        val characters = decoder.decode(ByteBuffer.wrap(bytes))
        try {
            characters.toString()
        } finally {
            if (characters.hasArray()) {
                characters.array().fill('\u0000')
            } else if (!characters.isReadOnly) {
                for (index in 0 until characters.limit()) {
                    characters.put(index, '\u0000')
                }
            }
        }
    } catch (_: CharacterCodingException) {
        throw ReallyMeCodecException.ProviderFailure()
    } finally {
        bytes.fill(0)
    }
}
