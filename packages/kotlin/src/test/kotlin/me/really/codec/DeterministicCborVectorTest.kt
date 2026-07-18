// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

package me.really.codec

import com.google.gson.JsonElement
import com.google.gson.JsonObject
import com.google.gson.JsonParser
import java.io.File
import java.util.Base64
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith
import kotlin.test.assertIs
import kotlin.test.assertNotNull
import kotlin.test.assertTrue

class DeterministicCborVectorTest {
    private companion object {
        private const val TEST_LIBRARY_PROPERTY = "reallyme.codec.testLibraryPath"
        private const val MAXIMUM_CBOR_BYTES = 1_048_576
        private const val CBOR_U32_LENGTH_HEADER_BYTES = 5
    }

    @Test
    fun sharedPositiveNegativeAndEquivalentVectors() {
        val codec = configuredCodec()
        val vectors = deterministicCborVectors()
        assertEquals(
            "rfc8949-core-deterministic-reallyme-0.2.0",
            vectors.requiredString("profile"),
        )
        val fixtureClasses = vectors.requiredObject("fixtureClasses")
        assertEquals("golden", fixtureClasses.requiredString("positive"))
        assertEquals("rejection-fixture", fixtureClasses.requiredString("negative"))
        assertEquals("construction-recipe", fixtureClasses.requiredString("resourceRejections"))
        assertEquals("interop-fixture", fixtureClasses.requiredString("interoperability"))

        for (vectorElement in vectors.requiredArray("positive")) {
            val vector = vectorElement.requiredObject()
            val name = vector.requiredString("name")
            val expected = vector.requiredString("hex").hexToByteArray()
            val value = sdkValue(vector.required("value"))
            assertContentEquals(expected, codec.deterministicCborEncode(value), name)
            val decoded = codec.deterministicCborDecode(expected)
            assertContentEquals(expected, codec.deterministicCborEncode(decoded), name)
        }

        for (vectorElement in vectors.requiredArray("negative")) {
            val vector = vectorElement.requiredObject()
            assertFailsWith<ReallyMeCodecException.InvalidInput>(
                message = vector.requiredString("name") + ":" + vector.requiredString("reason"),
            ) {
                codec.deterministicCborDecode(vector.requiredString("hex").hexToByteArray())
            }
        }

        for (vectorElement in vectors.requiredArray("equivalentInputOrders")) {
            val vector = vectorElement.requiredObject()
            val expected = vector.requiredString("hex").hexToByteArray()
            for (inputElement in vector.requiredArray("inputs")) {
                val entries = inputElement.requiredArray().map { entryElement ->
                    sdkMapEntry(entryElement.requiredObject())
                }
                assertContentEquals(
                    expected,
                    codec.deterministicCborEncode(ReallyMeDeterministicCborValue.Map(entries)),
                    vector.requiredString("name"),
                )
            }
        }
    }

    @Test
    fun typedBuildersPreserveDeterministicAndDagProfiles() {
        val codec = configuredCodec()
        val deterministic = ReallyMeDeterministicCbor.mapInt(
            listOf(
                2UL to ReallyMeDeterministicCbor.text("b"),
                1UL to ReallyMeDeterministicCbor.text("a"),
            ),
        )
        assertContentEquals(
            "a2016161026162".hexToByteArray(),
            codec.deterministicCborEncode(deterministic),
        )

        val dag = ReallyMeDagCbor.mapText(
            listOf(
                "b" to ReallyMeDagCbor.unsignedLong(2),
                "a" to ReallyMeDagCbor.bytes(byteArrayOf(0, 1, 2)),
            ),
        )
        assertContentEquals(
            "a2616143000102616202".hexToByteArray(),
            codec.dagCborEncode(dag),
        )
    }

    @Test
    fun sharedResourceRecipesAndSemanticMaximum() {
        val codec = configuredCodec()
        val vectors = deterministicCborVectors()

        assertFailsWith<ReallyMeCodecException.InvalidInput>(
            message = "aggregate-text-limit-plus-one",
        ) {
            codec.deterministicCborEncode(
                ReallyMeDeterministicCborValue.Text("a".repeat(MAXIMUM_CBOR_BYTES + 1)),
            )
        }

        for (vectorElement in vectors.requiredArray("resourceRejections")) {
            val vector = vectorElement.requiredObject()
            val construction = vector.requiredObject("construction")
            when (construction.requiredString("kind")) {
                "encoded-byte-count" -> {
                    val fill = construction.requiredString("fillByteHex").hexToByteArray()
                    assertEquals(1, fill.size)
                    assertFailsWith<ReallyMeCodecException.InvalidInput>(
                        message = vector.requiredString("name"),
                    ) {
                        codec.deterministicCborDecode(
                            ByteArray(construction.requiredInt("count")) { fill[0] },
                        )
                    }
                }
                "byte-string-length" ->
                    assertFailsWith<ReallyMeCodecException.InvalidInput>(
                        message = vector.requiredString("name"),
                    ) {
                        codec.deterministicCborEncode(
                            ReallyMeDeterministicCborValue.Bytes(
                                ByteArray(construction.requiredInt("count")),
                            ),
                        )
                    }
                "balanced-array-tree" ->
                    assertFailsWith<ReallyMeCodecException.InvalidInput>(
                        message = vector.requiredString("name"),
                    ) {
                        codec.deterministicCborEncode(
                            balancedArrayTree(
                                branching = construction.requiredInt("branching"),
                                levels = construction.requiredInt("levels"),
                            ),
                        )
                    }
                "array-of-null" ->
                    assertFailsWith<ReallyMeCodecException.InvalidInput>(
                        message = vector.requiredString("name"),
                    ) {
                        codec.deterministicCborEncode(
                            ReallyMeDeterministicCborValue.Array(
                                List(construction.requiredInt("count")) {
                                    ReallyMeDeterministicCborValue.Null
                                },
                            ),
                        )
                    }
                "nested-singleton-arrays" -> {
                    var value: ReallyMeDeterministicCborValue =
                        ReallyMeDeterministicCborValue.Null
                    repeat(construction.requiredInt("depth")) {
                        value = ReallyMeDeterministicCborValue.Array(listOf(value))
                    }
                    assertFailsWith<ReallyMeCodecException.InvalidInput>(
                        message = vector.requiredString("name"),
                    ) {
                        codec.deterministicCborEncode(value)
                    }
                }
                else -> error("unknown deterministic-CBOR resource construction")
            }
        }

        val payloadCount = MAXIMUM_CBOR_BYTES - CBOR_U32_LENGTH_HEADER_BYTES
        val encoded = codec.deterministicCborEncode(
            ReallyMeDeterministicCborValue.Bytes(ByteArray(payloadCount)),
        )
        assertEquals(MAXIMUM_CBOR_BYTES, encoded.size)
        assertContentEquals(
            byteArrayOf(0x5a, 0x00, 0x0f, 0xff.toByte(), 0xfb.toByte()),
            encoded.copyOfRange(0, CBOR_U32_LENGTH_HEADER_BYTES),
        )
        val decoded = assertIs<ReallyMeDeterministicCborValue.Bytes>(
            codec.deterministicCborDecode(encoded),
        )
        val decodedBytes = decoded.bytes()
        try {
            assertEquals(payloadCount, decodedBytes.size)
        } finally {
            decodedBytes.fill(0)
            encoded.fill(0)
        }
    }

    @Test
    fun idkitInteroperabilityFixtureRoundTripsThroughTypedSdk() {
        val codec = configuredCodec()
        val fixtures = deterministicCborVectors()
            .requiredArray("interoperability")
            .map { it.requiredObject() }
        val names = fixtures.map { it.requiredString("name") }.toSet()
        assertTrue("idkit-ios-synthetic-passport-claims-v1" in names)
        assertTrue("idkit-ios-synthetic-passport-claims-null-place-of-birth-v1" in names)
        assertTrue("idkit-ios-synthetic-fingerprint-map-v1" in names)
        assertTrue("idkit-ios-synthetic-mixed-integer-claim-tags-v1" in names)

        for (fixture in fixtures) {
            val encoded = fixture.requiredString("hex").hexToByteArray()
            assertEquals("synthetic", fixture.requiredString("fixtureKind"))
            assertEquals("reallyme/idkit-ios", fixture.requiredString("sourceRepo"))
            assertEquals("content-hash-pinned", fixture.requiredString("sourceCommit"))
            assertTrue(fixture.requiredString("source").isNotEmpty())
            assertTrue(fixture.requiredString("explanation").isNotEmpty())
            val sourceFiles = fixture.requiredArray("sourceFiles")
            assertTrue(sourceFiles.size() > 0)
            for (sourceFileElement in sourceFiles) {
                val sourceFile = sourceFileElement.requiredObject()
                assertTrue(sourceFile.requiredString("path").isNotEmpty())
                assertEquals(64, sourceFile.requiredString("sha256").length)
            }
            assertEquals(64, fixture.requiredString("sha256").length)
            assertEquals(fixture.requiredInt("byteLength"), encoded.size)
            val decoded = assertIs<ReallyMeDeterministicCborValue.Map>(
                codec.deterministicCborDecode(encoded),
                fixture.requiredString("name"),
            )
            assertEquals(fixture.requiredInt("entryCount"), decoded.entries.size)
            assertContentEquals(encoded, codec.deterministicCborEncode(decoded))
        }
    }

    private fun configuredCodec(): ReallyMeCodec {
        System.getProperty(TEST_LIBRARY_PROPERTY)
            ?.takeIf { it.isNotEmpty() }
            ?.let { ReallyMeCodecRustNativeProvider.loadLibrary(it) }
        return ReallyMeCodec
    }

    private fun deterministicCborVectors(): JsonObject {
        val root = File(System.getProperty("user.dir"))
        val manifestFile = listOf(
            File(root, "vectors/codec-vectors.json"),
            File(root, "../../vectors/codec-vectors.json"),
        ).firstOrNull { it.isFile } ?: error("missing codec vector manifest")
        val manifest = JsonParser.parseString(manifestFile.readText(Charsets.UTF_8))
            .requiredObject()
        require(manifest.requiredInt("schemaVersion") == 2) {
            "unsupported codec vector manifest schema"
        }
        return manifest.requiredObject("deterministicCbor")
    }

    private fun sdkInteger(value: JsonObject): ReallyMeDeterministicCborInteger =
        when {
            value.has("unsigned") ->
                ReallyMeDeterministicCborInteger.Unsigned(
                    value.requiredString("unsigned").toULong(),
                )
            value.has("negative") ->
                ReallyMeDeterministicCborInteger.Negative.of(
                    value.requiredString("negative").toLong(),
                )
            else -> error("unknown deterministic-CBOR integer fixture branch")
        }

    private fun sdkMapKey(value: JsonObject): ReallyMeDeterministicCborMapKey =
        when {
            value.has("integer") ->
                ReallyMeDeterministicCborMapKey.Integer(
                    sdkInteger(value.requiredObject("integer")),
                )
            value.has("text") -> ReallyMeDeterministicCborMapKey.Text(value.requiredString("text"))
            else -> error("unknown deterministic-CBOR map-key fixture branch")
        }

    private fun sdkMapEntry(value: JsonObject): ReallyMeDeterministicCborMapEntry =
        ReallyMeDeterministicCborMapEntry(
            key = sdkMapKey(value.requiredObject("key")),
            value = sdkValue(value.required("value")),
        )

    private fun sdkValue(element: JsonElement): ReallyMeDeterministicCborValue {
        val value = element.requiredObject()
        return when {
            value.has("unsigned") ->
                ReallyMeDeterministicCborValue.Integer(sdkInteger(value))
            value.has("negative") ->
                ReallyMeDeterministicCborValue.Integer(sdkInteger(value))
            value.has("bytes") ->
                ReallyMeDeterministicCborValue.Bytes(
                    Base64.getDecoder().decode(value.requiredString("bytes")),
                )
            value.has("text") -> ReallyMeDeterministicCborValue.Text(value.requiredString("text"))
            value.has("bool") -> ReallyMeDeterministicCborValue.Bool(value.required("bool").asBoolean)
            value.has("null") -> {
                require(value.required("null").asBoolean) {
                    "deterministic-CBOR null marker must be true"
                }
                ReallyMeDeterministicCborValue.Null
            }
            value.has("array") ->
                ReallyMeDeterministicCborValue.Array(
                    value.requiredArray("array").map(::sdkValue),
                )
            value.has("map") ->
                ReallyMeDeterministicCborValue.Map(
                    value.requiredArray("map").map { sdkMapEntry(it.requiredObject()) },
                )
            else -> error("unknown deterministic-CBOR value fixture branch")
        }
    }

    private fun balancedArrayTree(
        branching: Int,
        levels: Int,
    ): ReallyMeDeterministicCborValue {
        var value: ReallyMeDeterministicCborValue = ReallyMeDeterministicCborValue.Null
        repeat(levels) {
            value = ReallyMeDeterministicCborValue.Array(List(branching) { value })
        }
        return value
    }
}

private fun JsonElement.requiredObject(): JsonObject {
    require(isJsonObject) { "codec vector value must be an object" }
    return asJsonObject
}

private fun JsonObject.required(name: String): JsonElement =
    get(name) ?: error("missing codec vector field")

private fun JsonObject.requiredObject(name: String): JsonObject = required(name).requiredObject()

private fun JsonObject.requiredArray(name: String) = required(name).requiredArray()

private fun JsonElement.requiredArray() = run {
    require(isJsonArray) { "codec vector value must be an array" }
    asJsonArray
}

private fun JsonObject.requiredString(name: String): String {
    val value = required(name)
    require(value.isJsonPrimitive && value.asJsonPrimitive.isString) {
        "codec vector value must be a string"
    }
    return value.asString
}

private fun JsonObject.requiredInt(name: String): Int {
    val value = required(name)
    require(value.isJsonPrimitive && value.asJsonPrimitive.isNumber) {
        "codec vector value must be a number"
    }
    return value.asInt
}

private fun String.hexToByteArray(): ByteArray {
    require(length % 2 == 0) { "hex vector must have even length" }
    return ByteArray(length / 2) { index ->
        substring(index * 2, index * 2 + 2).toInt(16).toByte()
    }
}
