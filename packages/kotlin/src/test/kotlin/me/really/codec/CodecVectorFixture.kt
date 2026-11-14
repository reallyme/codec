// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec

import java.io.File

private val jsonStringPattern =
    Regex(
        """"([A-Za-z0-9]+)"\s*:\s*"((?:\\.|[^"\\])*)"""",
    )
private val jsonNumberPattern =
    Regex(
        """"([A-Za-z0-9]+)"\s*:\s*([0-9]+)""",
    )

internal class CodecVectors(
    private val strings: Map<String, String>,
    private val numbers: Map<String, Int>,
) {
    fun string(key: String): String = strings[key] ?: error("missing codec vector string: $key")

    fun int(key: String): Int = numbers[key] ?: error("missing codec vector number: $key")

    fun hexBytes(key: String): ByteArray = string(key).hexToBytes()

    companion object {
        fun load(): CodecVectors {
            val root = File(System.getProperty("user.dir"))
            val candidates =
                listOf(
                    File(root, "vectors/codec-vectors.json"),
                    File(root, "../../vectors/codec-vectors.json"),
                )
            val file =
                candidates.firstOrNull { it.isFile }
                    ?: error("missing codec vector manifest")
            val text = file.readText(Charsets.UTF_8)
            val strings =
                jsonStringPattern.findAll(text).associate { match ->
                    match.groupValues[1] to match.groupValues[2].jsonUnescaped()
                }
            val numbers =
                jsonNumberPattern.findAll(text).associate { match ->
                    match.groupValues[1] to match.groupValues[2].toInt()
                }
            require(numbers["schemaVersion"] == 2) {
                "unsupported codec vector manifest schema"
            }
            return CodecVectors(strings, numbers)
        }
    }
}

internal fun String.hexToBytes(): ByteArray {
    require(length % 2 == 0)
    return ByteArray(length / 2) { index ->
        substring(index * 2, index * 2 + 2).toInt(16).toByte()
    }
}

internal fun ByteArray.toLowerHex(): String = joinToString(separator = "") { byte -> "%02x".format(byte.toInt() and 0xff) }

private fun String.jsonUnescaped(): String {
    val output = StringBuilder(length)
    var index = 0
    while (index < length) {
        val ch = this[index]
        if (ch != '\\') {
            output.append(ch)
            index += 1
            continue
        }
        index += 1
        require(index < length)
        when (val escaped = this[index]) {
            '"' -> output.append('"')
            '\\' -> output.append('\\')
            '/' -> output.append('/')
            'b' -> output.append('\b')
            'f' -> output.append('\u000C')
            'n' -> output.append('\n')
            'r' -> output.append('\r')
            't' -> output.append('\t')
            'u' -> {
                require(index + 4 < length)
                val codePoint = substring(index + 1, index + 5).toInt(16)
                output.append(codePoint.toChar())
                index += 4
            }
            else -> error("unsupported JSON escape: $escaped")
        }
        index += 1
    }
    return output.toString()
}
