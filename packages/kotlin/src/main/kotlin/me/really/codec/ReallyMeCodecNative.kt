// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

package me.really.codec

internal object ReallyMeCodecNative {
    external fun probeNative(): Int

    external fun abiVersionNative(): Int

    external fun maxFfiInputBytesNative(): Long

    external fun maxFfiOutputBytesNative(): Long

    external fun maxOperationResponseBytesNative(): Long

    external fun processNative(
        operation: Int,
        first: ByteArray,
        second: ByteArray,
        third: ByteArray,
    ): ByteArray

    external fun processOperationNative(request: ByteArray): ByteArray

    external fun processOperationJsonNative(requestJson: ByteArray): ByteArray

    external fun processBoolNative(
        operation: Int,
        first: ByteArray,
        second: ByteArray,
    ): Int
}
