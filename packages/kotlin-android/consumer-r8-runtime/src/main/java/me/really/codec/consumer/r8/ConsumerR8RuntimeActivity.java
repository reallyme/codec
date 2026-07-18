// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

package me.really.codec.consumer.r8;

import android.app.Activity;
import android.os.Bundle;
import android.util.Log;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;
import me.really.codec.ReallyMeCodec;
import me.really.codec.ReallyMeDagCborCidVerification;
import me.really.codec.ReallyMeMulticodecLookupResult;
import me.really.codec.ReallyMeMulticodecMetadata;
import me.really.codec.ReallyMeParsedMultikey;
import me.really.codec.ReallyMePemDocument;

/**
 * Release-only consumer fixture for proving R8 keeps JNI-resolved codec names.
 *
 * A normal debug JVM/unit test cannot prove generated protobuf operation
 * messages, JNI entrypoints, and public SDK domain owners survive consumer R8
 * optimization, so this activity runs typed public facade paths from a
 * minified Android application.
 */
public final class ConsumerR8RuntimeActivity extends Activity {
    public static final String RESULT_TAG = "ReallyMeCodecR8Gate";
    public static final String RESULT_PASS = "PASS";
    public static final String RESULT_FAIL = "FAIL";

    private static final byte[] ED25519_PREFIX = new byte[] {
        (byte) 0xed,
        0x01,
        0x01,
        0x02,
        0x03,
    };
    private static final String ED25519_MULTIKEY =
        "z6Mkhf61x4mqDGsJ2sppXtYicDku4rMnVJrhzU3hBZg5d2bQ";
    private static final byte[] DAG_CBOR_BYTES = new byte[] {
        (byte) 0xa1,
        0x63,
        0x6d,
        0x73,
        0x67,
        0x65,
        0x68,
        0x65,
        0x6c,
        0x6c,
        0x6f,
    };
    private static final String DAG_CBOR_CID =
        "bafyreiarnfytckas2ctcnkred2wocelmn23eaqrxl5fypj5lzrwq4wyyfq";
    private static final byte[] PUBLIC_KEY_PEM = (
        "-----BEGIN PUBLIC KEY-----\n" +
            "MCowBQYDK2VwAyEA4VbcafTMDY9s5+VH9h7Zjxg4zbqnO8D5YQ+UMJj3l2E=\n" +
            "-----END PUBLIC KEY-----\n"
    ).getBytes(StandardCharsets.UTF_8);

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);
        try {
            runGate();
            Log.i(RESULT_TAG, RESULT_PASS);
        } catch (RuntimeException error) {
            Log.e(RESULT_TAG, RESULT_FAIL, error);
        } finally {
            finish();
        }
    }

    private static void runGate() {
        requireArrayEquals(
            "base64url decode",
            new byte[] { 1, 2, 3 },
            ReallyMeCodec.base64urlDecode("AQID")
        );
        requireEquals("base64url encode", "AQID", ReallyMeCodec.base64urlEncode(new byte[] { 1, 2, 3 }));

        ReallyMeMulticodecMetadata metadata =
            ReallyMeCodec.multicodecPrefixForName("ed25519-pub");
        requireEquals("multicodec prefix name", "ed25519-pub", metadata.getName());
        requireArrayEquals(
            "multicodec prefix bytes",
            new byte[] { (byte) 0xed, 0x01 },
            metadata.prefix()
        );

        ReallyMeMulticodecLookupResult lookup = ReallyMeCodec.multicodecLookupPrefix(ED25519_PREFIX);
        requireEquals("multicodec lookup name", "ed25519-pub", lookup.getName());
        requireTrue("multicodec lookup prefix length", lookup.getPrefixLength() == 2L);
        requireTrue("multicodec table result", !ReallyMeCodec.multicodecTable().getEntries().isEmpty());

        ReallyMeParsedMultikey parsed = ReallyMeCodec.multikeyParse(ED25519_MULTIKEY);
        requireEquals("multikey codec", "ed25519-pub", parsed.getCodecName());
        requireTrue("multikey public key length", parsed.publicKey().length == 32);

        ReallyMeDagCborCidVerification verification =
            ReallyMeCodec.dagCborVerifyCid(DAG_CBOR_CID, DAG_CBOR_BYTES);
        requireTrue("DAG-CBOR CID result", verification.getValid());

        try (ReallyMePemDocument document = ReallyMeCodec.decodePem(PUBLIC_KEY_PEM)) {
            requireTrue("PEM decode result", document.der().length > 0);
        }

        requireArrayEquals(
            "deterministic CBOR decode/encode",
            new byte[] { (byte) 0xa1, 0x66, 0x61, 0x6e, 0x73, 0x77, 0x65, 0x72, 0x18, 0x2a },
            ReallyMeCodec.deterministicCborEncode(
                ReallyMeCodec.deterministicCborDecode(
                    new byte[] { (byte) 0xa1, 0x66, 0x61, 0x6e, 0x73, 0x77, 0x65, 0x72, 0x18, 0x2a }
                )
            )
        );
    }

    private static void requireTrue(String label, boolean condition) {
        if (!condition) {
            throw new IllegalStateException(label);
        }
    }

    private static void requireArrayEquals(String label, byte[] expected, byte[] actual) {
        if (!Arrays.equals(expected, actual)) {
            throw new IllegalStateException(label);
        }
    }

    private static void requireEquals(String label, String expected, String actual) {
        if (!expected.equals(actual)) {
            throw new IllegalStateException(label);
        }
    }
}
