// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

package me.really.codec

internal object ReallyMeCodecNative {
    external fun probeNative(): Int

    external fun processNative(
        operation: Int,
        first: ByteArray,
        second: ByteArray,
        third: ByteArray,
    ): ByteArray

    external fun processProtoNative(request: ByteArray): ByteArray

    external fun processProtoJsonNative(requestJson: ByteArray): ByteArray

    external fun processBoolNative(
        operation: Int,
        first: ByteArray,
        second: ByteArray,
    ): Int

    external fun processProtoResultNative(request: ByteArray): ReallyMeCodecProtoResult
}
