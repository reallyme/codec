// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec

import com.google.protobuf.ByteString
import me.really.codec.v1.CodecApiOperationResponse
import me.really.codec.v1.CodecTlvProfile
import me.really.codec.v1.codecApiOperationRequest
import me.really.codec.v1.codecTlvLimits
import me.really.codec.v1.codecTlvOptions
import me.really.codec.v1.codecTlvSourceRequest
import java.util.Base64
import kotlin.test.Test
import kotlin.test.assertContentEquals
import kotlin.test.assertEquals

class TlvContractTest {
    @Test
    fun tlvAcceptsAnExplicitSixteenMiBSourceThroughJni() {
        val maximum = 16 * 1024 * 1024
        val source = ByteArray(maximum)
        source[0] = 4
        val request =
            codecApiOperationRequest {
                tlvReadHeader =
                    codecTlvSourceRequest {
                        encoded = ByteString.copyFrom(source)
                        options =
                            codecTlvOptions {
                                profile = CodecTlvProfile.CODEC_TLV_PROFILE_ISO7816_DEFINITE_BER
                                limits = codecTlvLimits { inputBytes = maximum.toLong() }
                            }
                    }
            }
        val encoded = Base64.getEncoder().encodeToString(source)
        val json =
            """
            {"tlvReadHeader":{"encoded":"$encoded","options":{"profile":"CODEC_TLV_PROFILE_ISO7816_DEFINITE_BER",
            "limits":{"inputBytes":"$maximum"}}}}
            """.trimIndent().encodeToByteArray()
        val binary = ReallyMeCodec.processOperation(request.toByteArray())
        assertContentEquals(binary, ReallyMeCodec.processOperationJson(json))
        val response = CodecApiOperationResponse.parseFrom(binary)
        assertEquals(2L, response.result.tlvReadHeader.consumedBytes)
        assertEquals(0L, response.result.tlvReadHeader.declaredValueLength)
    }
}
