// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

package me.really.codec;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.google.protobuf.ByteString;
import com.google.protobuf.MessageLite;
import java.util.HashSet;
import java.util.Set;
import me.really.codec.v1.CodecBase64DecodeRequest;
import me.really.codec.v1.CodecJsonArray;
import me.really.codec.v1.CodecJsonValue;
import me.really.codec.v1.CodecPemDecodeResult;
import org.junit.jupiter.api.Test;

final class SensitiveGeneratedEqualityTest {
    @Test
    void scalarMessagesRetainValueEqualityWithRedactedHashes() {
        CodecBase64DecodeRequest first =
            CodecBase64DecodeRequest.newBuilder().setValue("YQ==").build();
        assertEqualityContract(
            first,
            first.toBuilder().build(),
            first.toBuilder().setValue("Yg==").build()
        );
    }

    @Test
    void recursiveMessagesRetainValueEqualityWithRedactedHashes() {
        CodecJsonArray first = CodecJsonArray.newBuilder()
            .addValues(CodecJsonValue.newBuilder().setStringValue("first"))
            .build();
        CodecJsonArray different = CodecJsonArray.newBuilder()
            .addValues(CodecJsonValue.newBuilder().setStringValue("second"))
            .build();
        assertEqualityContract(first, first.toBuilder().build(), different);
    }

    @Test
    void byteMessagesRetainValueEqualityWithRedactedHashes() {
        CodecPemDecodeResult first = CodecPemDecodeResult.newBuilder()
            .setLabel("CERTIFICATE")
            .setDer(ByteString.copyFrom(new byte[] {0x30, 0x00}))
            .build();
        assertEqualityContract(
            first,
            first.toBuilder().build(),
            first.toBuilder().setDer(ByteString.copyFrom(new byte[] {0x30, 0x01})).build()
        );
    }

    private static void assertEqualityContract(
        MessageLite first,
        MessageLite equal,
        MessageLite different
    ) {
        assertEquals(first, equal);
        assertEquals(equal, first);
        assertNotEquals(first, different);
        assertNotEquals(first, null);
        assertNotEquals(first, new Object());
        assertEquals(0x524d, first.hashCode());
        assertEquals(first.hashCode(), equal.hashCode());
        assertEquals(first.hashCode(), different.hashCode());

        Set<MessageLite> values = new HashSet<>();
        values.add(first);
        values.add(equal);
        values.add(different);
        assertEquals(2, values.size());
        assertTrue(values.contains(equal));
    }
}
