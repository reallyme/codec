// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

package me.really.codec;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.fail;

import me.really.codec.v1.CodecError;
import me.really.codec.v1.CodecErrorReason;
import me.really.codec.v1.CodecPemDecodeResult;
import org.junit.jupiter.api.Test;

final class ReallyMeCodecJavaTest {
    private static final String TEST_LIBRARY_PROPERTY = "reallyme.codec.testLibraryPath";

    @Test
    void javaCallersUseStaticFacadeBackedByRustCodec() {
        loadConfiguredLibrary();

        byte[] input = new byte[] {1, 2, 3};
        String encoded = ReallyMeCodec.base64urlEncode(input);

        assertEquals("AQID", encoded);
        assertArrayEquals(input, ReallyMeCodec.base64urlDecode(encoded));
        assertEquals(0x71, ReallyMeCodec.dagCborCodecCode());
        assertNull(ReallyMeCodec.tryParseCid("not-a-cid"));

        byte[] oversizedBase58Input = new byte[8 * 1024 + 1];
        assertThrows(
            ReallyMeCodecException.InvalidInput.class,
            () -> ReallyMeCodec.base58btcEncode(oversizedBase58Input)
        );
        assertThrows(
            ReallyMeCodecException.InvalidInput.class,
            () -> ReallyMeCodec.multibaseBase58btcEncode(oversizedBase58Input)
        );
    }

    @Test
    void javaCallersCanCatchTypedRuntimeException() {
        loadConfiguredLibrary();

        try {
            ReallyMeCodec.lowerHexToBytes("DEADBEEF");
            fail("expected typed invalid input exception");
        } catch (ReallyMeCodecException.InvalidInput expected) {
            assertEquals("invalid input", expected.getMessage());
        }
    }

    @Test
    void javaCallersCanParseRustBackedProtobufOutputs() throws Exception {
        loadConfiguredLibrary();

        byte[] der = new byte[] {0x30, 0x03, 0x02, 0x01, 0x01};
        String pem = ReallyMeCodec.encodePem("PRIVATE KEY", der);
        CodecPemDecodeResult decoded = CodecPemDecodeResult.parseFrom(
            ReallyMeCodec.decodePemProto(pem)
        );
        ReallyMeCodecProtoResult decodedResult = ReallyMeCodec.decodePemProtoResult(pem);

        assertEquals("PRIVATE KEY", decoded.getLabel());
        assertEquals(ReallyMeCodecProtoStatus.RESULT, decodedResult.getStatus());
        assertArrayEquals(der, decoded.getDer().toByteArray());

        assertThrows(
            ReallyMeCodecException.InvalidInput.class,
            () -> ReallyMeCodec.decodePemProto(pem, "{\"allowedLabels\":[\"PUBLIC KEY\"]}")
        );
        ReallyMeCodecProtoResult pemErrorResult = ReallyMeCodec.decodePemProtoResult(
            pem,
            "{\"allowedLabels\":[\"PUBLIC KEY\"]}"
        );
        CodecError pemError = CodecError.parseFrom(pemErrorResult.getBytes());
        assertEquals(CodecError.ErrorCase.PEM, pemError.getErrorCase());
        assertEquals(ReallyMeCodecProtoStatus.CODEC_ERROR, pemErrorResult.getStatus());
        assertEquals(
            CodecErrorReason.CODEC_ERROR_REASON_PEM_UNSUPPORTED_LABEL,
            pemError.getPem().getReason()
        );
    }

    @Test
    void providerLoadingFailsClosedForJavaCallers() {
        assertThrows(
            ReallyMeCodecException.InvalidInput.class,
            () -> ReallyMeCodecRustNativeProvider.loadLibrary("")
        );
        assertThrows(
            ReallyMeCodecException.ProviderFailure.class,
            () -> ReallyMeCodecRustNativeProvider.loadLibrary("/tmp/reallyme-codec-missing-library.dylib")
        );
    }

    private static void loadConfiguredLibrary() {
        String libraryPath = System.getProperty(TEST_LIBRARY_PROPERTY);
        if (libraryPath != null && !libraryPath.isEmpty()) {
            ReallyMeCodecRustNativeProvider.loadLibrary(libraryPath);
        }
        ReallyMeCodec.requireSupportedMulticodec("ed25519-pub");
    }
}
