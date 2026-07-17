// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

package me.really.codec;

import static org.junit.jupiter.api.Assertions.assertArrayEquals;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.junit.jupiter.api.Assertions.fail;

import java.io.File;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.util.regex.Matcher;
import java.util.regex.Pattern;
import me.really.codec.v1.CodecMulticodecTableResult;
import me.really.codec.v1.CodecProtoResultEnvelope;
import me.really.codec.v1.CodecProtoResultStatus;
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
    void javaCallersCanUseWipeablePemBuffers() {
        loadConfiguredLibrary();

        byte[] der = new byte[] {0x30, 0x03, 0x02, 0x01, 0x01};
        byte[] pem = ReallyMeCodec.encodePem("PRIVATE KEY", der);
        byte[] decoded = ReallyMeCodec.decodePem(pem);
        String decodedJson = new String(decoded, StandardCharsets.UTF_8);

        assertTrue(new String(pem, StandardCharsets.UTF_8).contains("BEGIN PRIVATE KEY"));
        assertTrue(decodedJson.contains("\"label\":\"PRIVATE KEY\""));
        assertTrue(decodedJson.contains("\"der\":\"MAMCAQE\""));
    }

    @Test
    void javaCallersCanProcessSharedProtoVector() throws Exception {
        loadConfiguredLibrary();

        byte[] binaryEnvelope = ReallyMeCodec.processProto(
            hexToBytes(vectorString("protoMulticodecTableRequestHex"))
        );
        byte[] jsonEnvelope = ReallyMeCodec.processProtoJson(
            vectorString("protoMulticodecTableRequestJson").getBytes(StandardCharsets.UTF_8)
        );

        assertArrayEquals(binaryEnvelope, jsonEnvelope);
        CodecProtoResultEnvelope decodedEnvelope =
            CodecProtoResultEnvelope.parseFrom(binaryEnvelope);
        assertEquals(
            CodecProtoResultStatus.CODEC_PROTO_RESULT_STATUS_RESULT,
            decodedEnvelope.getStatus()
        );
        String requiredName = vectorString("multicodecTableRequiredName");
        assertTrue(
            CodecMulticodecTableResult.parseFrom(decodedEnvelope.getPayload())
                .getEntriesList()
                .stream()
                .anyMatch(entry -> entry.getName().equals(requiredName))
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

    private static String vectorString(String key) throws Exception {
        String text = Files.readString(vectorPath().toPath(), StandardCharsets.UTF_8);
        Pattern pattern = Pattern.compile(
            "\"" + Pattern.quote(key) + "\"\\s*:\\s*\"((?:\\\\.|[^\"\\\\])*)\""
        );
        Matcher matcher = pattern.matcher(text);
        if (!matcher.find()) {
            fail("missing codec vector string: " + key);
        }
        return jsonUnescaped(matcher.group(1));
    }

    private static File vectorPath() {
        File root = new File(System.getProperty("user.dir"));
        File repoRelative = new File(root, "test-vectors/codec-vectors.json");
        if (repoRelative.isFile()) {
            return repoRelative;
        }
        return new File(root, "../../test-vectors/codec-vectors.json");
    }

    private static byte[] hexToBytes(String hex) {
        if ((hex.length() % 2) != 0) {
            fail("hex vector has odd length");
        }
        byte[] bytes = new byte[hex.length() / 2];
        for (int index = 0; index < bytes.length; index += 1) {
            int offset = index * 2;
            bytes[index] = (byte) Integer.parseInt(hex.substring(offset, offset + 2), 16);
        }
        return bytes;
    }

    private static String jsonUnescaped(String value) {
        StringBuilder output = new StringBuilder(value.length());
        for (int index = 0; index < value.length(); index += 1) {
            char ch = value.charAt(index);
            if (ch != '\\') {
                output.append(ch);
                continue;
            }
            index += 1;
            if (index >= value.length()) {
                fail("truncated JSON escape");
            }
            char escaped = value.charAt(index);
            switch (escaped) {
                case '"':
                    output.append('"');
                    break;
                case '\\':
                    output.append('\\');
                    break;
                case '/':
                    output.append('/');
                    break;
                case 'b':
                    output.append('\b');
                    break;
                case 'f':
                    output.append('\f');
                    break;
                case 'n':
                    output.append('\n');
                    break;
                case 'r':
                    output.append('\r');
                    break;
                case 't':
                    output.append('\t');
                    break;
                case 'u':
                    if (index + 4 >= value.length()) {
                        fail("truncated unicode escape");
                    }
                    output.append(
                        (char) Integer.parseInt(value.substring(index + 1, index + 5), 16)
                    );
                    index += 4;
                    break;
                default:
                    fail("unsupported JSON escape");
            }
        }
        return output.toString();
    }
}
