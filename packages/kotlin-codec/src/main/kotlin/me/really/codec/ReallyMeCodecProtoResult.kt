// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

package me.really.codec

public enum class ReallyMeCodecProtoStatus {
    RESULT,
    CODEC_ERROR,
}

public data class ReallyMeCodecProtoResult(
    public val status: ReallyMeCodecProtoStatus,
    public val bytes: ByteArray,
) {
    public val isCodecError: Boolean
        get() = status == ReallyMeCodecProtoStatus.CODEC_ERROR

    override fun equals(other: Any?): Boolean {
        if (this === other) {
            return true
        }
        if (other !is ReallyMeCodecProtoResult) {
            return false
        }
        return status == other.status && bytes.contentEquals(other.bytes)
    }

    override fun hashCode(): Int {
        return 31 * status.hashCode() + bytes.contentHashCode()
    }

    override fun toString(): String =
        "ReallyMeCodecProtoResult(status=$status, bytes=<redacted>)"
}
