#!/usr/bin/env node
// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(fileURLToPath(new URL("..", import.meta.url)));
const codecPackageVersion = "0.1.21";
const codecProtoPackageVersion = "0.1.21";
const releasePackagesMode = process.argv.includes("--release-packages");

const readText = (path) => readFileSync(resolve(root, path), "utf8");
const readJson = (path) => JSON.parse(readText(path));

const fail = (message) => {
  console.error(`release readiness check failed: ${message}`);
  process.exit(1);
};

const assertContains = (path, needle) => {
  if (!readText(path).includes(needle)) {
    fail(`${path} does not contain ${needle}`);
  }
};

const assertNotContains = (path, needle) => {
  if (readText(path).includes(needle)) {
    fail(`${path} unexpectedly contains ${needle}`);
  }
};

const assertMinOccurrences = (path, needle, expectedMin) => {
  const count = readText(path).split(needle).length - 1;
  if (count < expectedMin) {
    fail(`${path} contains ${needle} ${count} time(s), expected at least ${expectedMin}`);
  }
};

const rootCargo = readText("Cargo.toml");
for (const member of [
  '"crates/codec"',
  '"crates/codec/ffi"',
  '"crates/codec/wasm-package"',
  '"crates/proto/codec"',
]) {
  assertContains("Cargo.toml", member);
}
if (rootCargo.includes("crates/crypto") || rootCargo.includes("crates/proto/crypto")) {
  fail("root Cargo.toml still references removed crypto crates");
}
assertContains("Cargo.toml", 'repository = "https://github.com/reallyme/codec"');

const codecCargo = readText("crates/codec/Cargo.toml");
if (!codecCargo.includes(`version = "${codecPackageVersion}"`)) {
  fail(`crates/codec/Cargo.toml is not versioned ${codecPackageVersion}`);
}
assertContains(
  "crates/codec/Cargo.toml",
  'include = ["/src/**/*.rs", "/Cargo.toml", "/README.md", "/LICENSE", "/NOTICE"]',
);
assertContains("crates/codec/README.md", `reallyme-codec = "${codecPackageVersion}"`);

assertContains("crates/codec/ffi/Cargo.toml", 'name = "reallyme-codec-ffi"');
assertContains("crates/codec/ffi/Cargo.toml", "publish = false");
assertContains("crates/codec/ffi/src/codec.rs", "rm_codec_process_proto");
assertContains("crates/codec/wasm-package/Cargo.toml", 'name = "reallyme-codec-wasm"');
assertContains("crates/codec/wasm-package/src/proto_output.rs", "multikey_parse_proto");
assertContains(".github/workflows/code-checks.yml", "cargo-deny@0.19.6");
assertContains(".github/workflows/code-checks.yml", "node-version: '24'");
assertMinOccurrences(".github/workflows/crates-release.yml", "node-version: '24'", 2);

const codecProtoCargo = readText("crates/proto/codec/Cargo.toml");
if (!codecProtoCargo.includes(`version = "${codecProtoPackageVersion}"`)) {
  fail(`crates/proto/codec/Cargo.toml is not versioned ${codecProtoPackageVersion}`);
}
assertContains("crates/proto/codec/Cargo.toml", 'name = "reallyme-codec-proto"');
assertContains(
  "crates/proto/codec/Cargo.toml",
  '"/proto/**/*.proto"',
);
assertContains(
  "crates/proto/codec/Cargo.toml",
  '"/tests/**/*.rs"',
);
assertContains(
  "crates/proto/codec/README.md",
  `reallyme-codec-proto = { version = "${codecProtoPackageVersion}", features = ["generated"] }`,
);

assertContains("buf.gen.yaml", "out: crates/proto/codec/src/generated/buffa");
assertContains("buf.gen.yaml", "out: packages/codec/src/proto/generated");
assertContains("crates/proto/codec/src/generated/buffa/mod.rs", "pub mod codec");
assertContains("crates/proto/codec/src/lib.rs", "pub struct CodecWireError");
assertContains("crates/proto/codec/src/lib.rs", "pub fn try_new");
assertContains("crates/proto/codec/src/lib.rs", "decode_codec_error_payload");
assertContains("crates/proto/codec/src/lib.rs", "MAX_CODEC_PROTO_MESSAGE_BYTES");
assertContains("crates/proto/codec/src/lib.rs", "MAX_CODEC_PROTO_ERROR_ENVELOPE_BYTES");
assertContains("crates/proto/codec/tests/generated_tests.rs", "malformed_codec_error_payloads_decode_as_backend_malformed_protobuf");
assertContains("crates/proto/codec/tests/generated_tests.rs", "codec_error_payload_decode_rejects_oversized_envelopes");
assertContains("crates/proto/codec/tests/generated_tests.rs", "json_decode_rejects_inputs_that_expand_past_binary_cap");
assertContains(".github/workflows/protobuf-ci.yml", "BUFFA_VERSION: 0.8.1");
assertContains(".github/workflows/protobuf-ci.yml", "cargo install protoc-gen-buffa-packaging");
assertContains(".github/workflows/protobuf-ci.yml", "buf lint");
assertContains(".github/workflows/protobuf-ci.yml", "buf breaking --against '.git#branch=origin/main'");
assertContains(".github/workflows/protobuf-ci.yml", "buf generate");
assertContains(".github/workflows/protobuf-ci.yml", "git diff --exit-code -- crates/proto/codec/proto crates/proto/codec/src/generated packages/codec/src/proto/generated gen");
assertContains(
  ".github/workflows/protobuf-ci.yml",
  "bufbuild/buf-setup-action@a47c93e0b1648d5651a065437926377d060baa99",
);
if (readText("buf.gen.yaml").includes("reallyme.crypto.v1")) {
  fail("buf.gen.yaml still generates crypto protos");
}

const tsCodecPackage = readJson("packages/codec/package.json");
if (tsCodecPackage.name !== "@reallyme/codec") {
  fail("packages/codec/package.json must publish @reallyme/codec");
}
if (tsCodecPackage.version !== codecPackageVersion) {
  fail(`packages/codec/package.json is not versioned ${codecPackageVersion}`);
}
if (tsCodecPackage.private === true) {
  fail("packages/codec/package.json is private and cannot be published to npm");
}
assertContains("packages/codec/README.md", "@reallyme/codec/wasm/reallyme_codec_wasm.js");

const kotlinCodecBuild = readText("packages/kotlin-codec/build.gradle.kts");
if (!kotlinCodecBuild.includes(`version = "${codecPackageVersion}"`)) {
  fail(`packages/kotlin-codec/build.gradle.kts is not versioned ${codecPackageVersion}`);
}
assertContains("packages/kotlin-codec/build.gradle.kts", 'artifactId = "codec"');
assertContains("packages/kotlin-codec/build.gradle.kts", "Java, Kotlin, JVM, and Android");
assertContains("packages/kotlin-codec/build.gradle.kts", "https://github.com/reallyme/codec");
assertContains("packages/kotlin-codec/build.gradle.kts", "reallyme.codec.nativeResourcesDir");
assertContains("packages/kotlin-codec/build.gradle.kts", "reallyme.codec.requireFullNativeResources");
assertContains("packages/kotlin-codec/build.gradle.kts", "verifyBundledNativeResources");
assertContains("packages/kotlin-codec/build.gradle.kts", "verifyHostBundledNativeResource");
assertContains("packages/kotlin-codec/build.gradle.kts", "stageHostNativeResource");
assertContains("packages/kotlin-codec/build.gradle.kts", "native-manifest.json");
assertContains("packages/kotlin-codec/build.gradle.kts", "PublishToMavenRepository");
assertContains("scripts/write_native_manifest.mjs", "sha256");
assertContains(".github/workflows/package-release.yml", "Write native checksum manifest");
assertContains(".github/workflows/release-preflight.yml", "Write native checksum manifest");
assertContains(".github/workflows/jvm-native-resources.yml", "jvm native resources");
assertContains(".github/workflows/jvm-native-resources.yml", "kotlin-native-");
assertContains(".github/workflows/jvm-native-resources.yml", "build_kotlin_native_resource.sh");
assertContains(
  ".github/workflows/jvm-native-resources.yml",
  "actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a",
);
assertContains(".github/workflows/package-release.yml", "contents: read");
assertContains(".github/workflows/package-release.yml", "publish_swift:");
assertContains(".github/workflows/package-release.yml", "publish_maven:");
assertMinOccurrences(".github/workflows/package-release.yml", "node-version: '24'", 3);
assertMinOccurrences(".github/workflows/release-preflight.yml", "node-version: '24'", 5);
for (const workflowPath of [
  ".github/workflows/fuzz.yml",
  ".github/workflows/jvm-native-resources.yml",
  ".github/workflows/package-release.yml",
  ".github/workflows/release-preflight.yml",
]) {
  assertNotContains(workflowPath, "actions/upload-artifact@330a01c490aca151604b8cf639adc76d48f6c5d4");
  assertNotContains(workflowPath, "actions/download-artifact@634f93cb2916e3fdff6788551b99b062d0335ce0");
}
assertContains(
  ".github/workflows/package-release.yml",
  "actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a # v7.0.1",
);
assertContains(
  ".github/workflows/package-release.yml",
  "actions/download-artifact@3e5f45b2cfb9172054b4087a40e8e0b5a5461e7c # v8.0.1",
);
assertContains(".github/workflows/package-release.yml", "requireFullNativeResources=true");
assertContains(".github/workflows/release-preflight.yml", "requireFullNativeResources=true");
assertContains("packages/kotlin-codec/settings.gradle.kts", 'rootProject.name = "reallyme-codec"');
assertContains("packages/kotlin-codec/README.md", "me.really:codec:0.1.21");
assertContains("packages/kotlin-codec/README.md", "ships Rust JNI libraries as platform resources");
assertContains(
  "packages/kotlin-codec/src/main/kotlin/me/really/codec/RustNativeProvider.kt",
  "/me/really/codec/native",
);
assertContains(
  "packages/kotlin-codec/src/main/kotlin/me/really/codec/RustNativeProvider.kt",
  'ANDROID_LIBRARY_NAME: String = "reallyme_codec_ffi"',
);
assertContains(
  "packages/kotlin-codec/src/main/kotlin/me/really/codec/RustNativeProvider.kt",
  "System.loadLibrary(ANDROID_LIBRARY_NAME)",
);
assertContains("packages/kotlin-codec/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "@JvmStatic");
assertContains("packages/kotlin-codec/build.gradle.kts", "fun nonBlank(value: String?): String?");
assertContains("packages/kotlin-codec/build.gradle.kts", "verifyRemoteMavenPublishingConfigured");
assertContains("packages/kotlin-codec/build.gradle.kts", "remote Maven publishing is not configured");
assertContains("packages/kotlin-codec/build.gradle.kts", "remoteMavenRepositoryUrlValue != null");
assertContains(".github/workflows/package-release.yml", "-Preallyme.maven.requireRemote=true");
assertContains("packages/kotlin-codec/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "public fun tryParseCid(cid: String): String?");
assertContains("packages/kotlin-codec/src/main/kotlin/me/really/codec/ReallyMeCodec.kt", "public fun dagCborCodecCode(): Int");
assertContains("packages/kotlin-codec/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", 'assertNull(codec.tryParseCid("not-a-cid"))');
assertContains("packages/kotlin-codec/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", "assertEquals(0x71, codec.dagCborCodecCode())");
assertContains("packages/kotlin-codec/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", 'codec.base58btcDecode("")');
assertContains("packages/kotlin-codec/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", "codec.base58btcEncode(oversizedBase58Input)");
assertContains("packages/kotlin-codec/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", 'codec.multicodecPrefixForNameProto("not-a-codec")');
assertContains("packages/kotlin-codec/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", "codec.multicodecLookupPrefixProto(byteArrayOf(0, 0, 7))");
assertContains("packages/kotlin-codec/src/test/kotlin/me/really/codec/ReallyMeCodecTest.kt", 'codec.dagCborVerifyCid("", encoded)');
assertContains("crates/codec/multibase/src/base58btc.rs", "bytes.len() > MAX_BASE58BTC_INPUT_LEN");
assertContains("crates/codec/multibase/tests/base58btc_tests.rs", "rejects_inputs_above_encode_cap_before_base58_conversion");
assertContains(
  "packages/kotlin-codec/src/test/java/me/really/codec/ReallyMeCodecJavaTest.java",
  "ReallyMeCodec.base64urlEncode",
);
assertContains(
  "packages/kotlin-codec/src/test/java/me/really/codec/ReallyMeCodecJavaTest.java",
  'assertNull(ReallyMeCodec.tryParseCid("not-a-cid"))',
);
assertContains(
  "packages/kotlin-codec/src/test/java/me/really/codec/ReallyMeCodecJavaTest.java",
  "ReallyMeCodec.base58btcEncode(oversizedBase58Input)",
);

assertContains("Package.swift", 'name: "reallyme-codec"');
assertContains("Package.swift", 'name: "ReallyMeCodec"');
assertContains("Package.swift", 'name: "ReallyMeCodecProto"');
assertContains("Package.swift", 'name: "ReallyMeCodecFFI"');
assertContains("Package.swift", "ReallyMeCodecFFI.xcframework.zip");
assertContains("Package.swift", 'let ffiArtifactLocalPathOverride = ""');
assertNotContains("Package.swift", "build/swift/ReallyMeCodecFFI.xcframework");
assertNotContains("Package.swift", "FileManager.default.fileExists");
assertContains("Package.swift", "REALLYME_CODEC_SWIFTPM_RUNTIME_FFI");
assertContains("packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift", "public func tryParseCid(_ cid: String) throws -> String?");
assertContains("packages/swift/Sources/ReallyMeCodec/ReallyMeCodec.swift", "public func dagCborCodecCode() throws -> UInt32");
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", 'XCTAssertNil(try codec.tryParseCid("not-a-cid"))');
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", "XCTAssertEqual(try codec.dagCborCodecCode(), 0x71)");
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", 'codec.base58btcDecode("")');
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", "codec.base58btcEncode(oversizedBase58Input)");
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", 'codec.multicodecPrefixForNameProto("not-a-codec")');
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", "codec.multicodecLookupPrefixProto([0, 0, 7])");
assertContains("packages/swift/Tests/ReallyMeCodecTests/ReallyMeCodecTests.swift", 'codec.dagCborVerifyCid(cid: "", bytes: encoded)');
assertContains("scripts/build_swift_xcframework.sh", "xcodebuild -create-xcframework");
assertContains("scripts/build_swift_xcframework.sh", "Modules/module.modulemap");
assertNotContains("scripts/build_swift_xcframework.sh", "HEADERS_DIR}/module.modulemap");
assertContains("scripts/build_swift_xcframework.sh", "verify_xcframework_layout");
assertContains("scripts/build_swift_xcframework.sh", "Headers/module.modulemap");
assertContains("scripts/prepare_swift_binary_manifest.mjs", "ffiArtifactChecksum");
assertContains("scripts/prepare_swift_binary_manifest.mjs", "--local-artifact-path");
assertContains(".github/workflows/release-preflight.yml", "REALLYME_CODEC_SWIFTPM_RUNTIME_FFI");
assertContains(".github/workflows/release-preflight.yml", "Build SwiftPM binary artifact");
assertContains(".github/workflows/release-preflight.yml", "Prepare local SwiftPM binary manifest");
assertContains(".github/workflows/release-preflight.yml", "--local-artifact-path build/swift/ReallyMeCodecFFI.xcframework");
assertContains(".github/workflows/release-preflight.yml", "Reset SwiftPM package state");
assertContains(".github/workflows/release-preflight.yml", "swift package reset");
assertContains(".github/workflows/release-preflight.yml", "Test Swift package with linked binary target");
assertContains(".github/workflows/release-preflight.yml", "node scripts/check_release_readiness.mjs --release-packages");
assertContains("packages/codec/src/multiformat.ts", "ensureStringValue(encoded)");
assertContains("packages/codec/src/cbor.ts", "ensureStringValue(cid)");
assertContains("packages/codec/src/proto.ts", "CodecBackendErrorSchema");
assertContains("packages/codec/test/reallyme-codec.test.mjs", 'assert.deepEqual(base58btcDecode(""), bytes())');
assertContains("packages/codec/test/reallyme-codec.test.mjs", 'dagCborVerifyCid("", encoded)');
if (releasePackagesMode) {
  const swiftPackage = readText("Package.swift");
  if (swiftPackage.includes('let ffiArtifactChecksum = "0000000000000000000000000000000000000000000000000000000000000000"')) {
    fail("Package.swift still has the Swift binary artifact checksum placeholder");
  }
  if (!swiftPackage.includes('let ffiArtifactLocalPathOverride = ""')) {
    fail("Package.swift must use the release URL artifact in release package mode");
  }
  assertContains("Package.swift", 'codecTargetDependencies.append("ReallyMeCodecFFI")');
  assertContains("Package.swift", 'codecSwiftSettings.append(.define("REALLYME_CODEC_LINKED_FFI"))');
}
if (readText("Package.swift").includes("ReallyMeCrypto")) {
  fail("Package.swift still exposes crypto Swift products");
}

const androidCodecBuild = readText("packages/android-codec/build.gradle.kts");
if (!androidCodecBuild.includes(`version = "${codecPackageVersion}"`)) {
  fail(`packages/android-codec/build.gradle.kts is not versioned ${codecPackageVersion}`);
}
assertContains("packages/android-codec/settings.gradle.kts", 'rootProject.name = "reallyme-codec-android"');
assertContains("packages/android-codec/build.gradle.kts", 'id("com.android.library")');
assertContains("packages/android-codec/build.gradle.kts", 'id("com.android.library") version "9.3.0"');
assertContains("packages/android-codec/build.gradle.kts", 'artifactId = "codec-android"');
assertContains("packages/android-codec/build.gradle.kts", "jniLibs.directories");
assertContains("packages/android-codec/build.gradle.kts", "assets.directories");
assertContains("packages/android-codec/build.gradle.kts", "reallyme-codec/native-manifest.json");
assertContains("packages/android-codec/build.gradle.kts", "inputs.dir(jniLibsDir).optional()");
assertContains("packages/android-codec/build.gradle.kts", "verifyAndroidJniLibs");
assertContains("packages/android-codec/build.gradle.kts", "verifyReleaseAarContainsJniLibs");
assertContains("packages/android-codec/build.gradle.kts", "PublishToMavenRepository");
assertContains("packages/android-codec/build.gradle.kts", "fun nonBlank(value: String?): String?");
assertContains("packages/android-codec/build.gradle.kts", "verifyRemoteMavenPublishingConfigured");
assertContains("packages/android-codec/build.gradle.kts", "remote Maven publishing is not configured");
assertContains("packages/android-codec/build.gradle.kts", "remoteMavenRepositoryUrlValue != null");
assertContains("packages/android-codec/consumer-rules.pro", "ReallyMeCodecException$*");
assertContains("packages/android-codec/README.md", "me.really:codec-android:0.1.21");
assertContains("scripts/build_android_native_resources.sh", "aarch64-linux-android");
assertContains("scripts/build_android_native_resources.sh", "armv7-linux-androideabi");
assertContains("scripts/build_android_native_resources.sh", "x86_64-linux-android");
assertContains("scripts/build_android_native_resources.sh", "i686-linux-android");
assertContains(".github/workflows/package-release.yml", "ANDROID_NDK_HOME=${ANDROID_HOME}/ndk/29.0.14206865");
assertNotContains(".github/workflows/package-release.yml", "ANDROID_NDK_HOME: ${{ env.ANDROID_HOME }}/ndk/29.0.14206865");
assertContains(".github/workflows/release-preflight.yml", "ANDROID_NDK_HOME=${ANDROID_HOME}/ndk/29.0.14206865");
assertContains(".github/workflows/package-release.yml", "android-aar:");
assertContains(".github/workflows/package-release.yml", "Write Android native checksum manifest");
assertContains(".github/workflows/release-preflight.yml", "Write Android native checksum manifest");
assertContains(".github/workflows/package-release.yml", "verifyReleaseAarContainsJniLibs");
assertContains(".github/workflows/package-release.yml", "RELEASE_VERSION");
assertContains(".github/workflows/package-release.yml", "needs: swift-artifact");
assertContains(".github/workflows/package-release.yml", "if: inputs.publish_swift == true");
assertContains(".github/workflows/package-release.yml", "if: inputs.publish_maven == true");
assertNotContains(".github/workflows/package-release.yml", "if: inputs.publish == true");
assertContains(".github/workflows/release-preflight.yml", "android aar preflight");
assertContains(".github/workflows/release-preflight.yml", "requireAndroidJniLibs=true");

assertContains("README.md", "https://github.com/reallyme/codec");
assertContains("README.md", "https://www.npmjs.com/package/@reallyme/codec");
assertContains("README.md", "me.really:codec:0.1.21");
assertContains("README.md", "reallyme-codec-proto");
assertContains("README.md", "## Published Surfaces");
assertContains("README.md", "`me.really:codec-android` AAR");
assertContains("README.md", "## Source Map");
assertContains("CONTRACT.md", "reallyme/codec");
assertContains("SECURITY.md", "reallyme-codec");
assertContains("SECURITY_MEMORY_MODEL.md", "reallyme-codec");
assertContains("buf.yaml", "modules:");
assertContains("buf.yaml", "- path: crates/proto/codec/proto");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "package reallyme.codec.v1;");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "message CodecError");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "message CodecBaseEncodingError");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "message CodecPemError");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "message CodecMultiformatError");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "message CodecCanonicalizationError");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "message CodecBackendError");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "message CodecMulticodecSpec");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "message CodecMultikeyParseResult");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "message CodecPemDecodeResult");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "message CodecDagCborVerifyCidResult");
assertContains("crates/proto/codec/proto/reallyme/codec/v1/codec.proto", "enum CodecErrorReason");

console.log("codec release readiness checks passed");
