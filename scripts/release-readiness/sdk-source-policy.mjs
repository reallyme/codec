// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

export function assertSdkSourcePolicies(context) {
  const nativeResources = process.env.REALLYME_CODEC_KOTLIN_NATIVE_RESOURCES_DIR;
  const kotlinTestArgs = ["ktlintMainSourceSetCheck", "ktlintTestSourceSetCheck", "test"];
  if (nativeResources !== undefined) {
    kotlinTestArgs.push(`-Preallyme.codec.nativeResourcesDir=${nativeResources}`);
  }
  const kotlinEnv = { ...process.env };
  // Swift uses the dylib directly. Kotlin exercises the same build through
  // its own bundled JNI provider and digest validation.
  delete kotlinEnv.REALLYME_CODEC_FFI_LIBRARY_PATH;
  context.assertTypeScriptSourcePolicy({
    generatedPrefixes: ["gen/es", "packages/ts/src/proto/generated"],
    facadeFiles: ["packages/ts/src/cbor.ts"],
    tsconfigPaths: ["packages/ts/tsconfig.json"],
    staticAnalysis: { files: [{
      path: "packages/ts/scripts/lint-typescript.mjs",
      required: ["typescript/unstable/sync", "node.forEachChild(inspect)",
        "SyntaxKind.AnyKeyword", "SyntaxKind.NonNullExpression", "SyntaxKind.AsExpression"],
    }] },
    verification: [
      // The build's pinned tsc invocation is the typecheck; doing it again
      // before the test build would also require a second WASM artifact build.
      { roles: ["typecheck", "test"], command: "npm", args: ["--prefix", "packages/ts", "test"] },
      { roles: ["lint"], command: "npm", args: ["--prefix", "packages/ts", "run", "lint"] },
    ],
  });
  context.assertSwiftSourcePolicy({
    generatedPrefixes: ["gen/swift", "packages/swift/Sources/ReallyMeCodec/Generated"],
    configuration: { files: [{
      path: ".swift-format",
      required: ['"NeverForceUnwrap": true', '"NeverUseForceTry": true', '"NeverUseImplicitlyUnwrappedOptionals": true'],
    }] },
    auditedConcurrencyBridges: [
      {
        path: "packages/swift/Sources/ReallyMeCodec/RustCAbiLibrary.swift",
        digest: "b11011ae3c9857be91b43caffae8eefdc329b0c79b54756152c53be0da8bfd34",
        reason: "The immutable dlopen handle is retained for every function call and closed only after the last owning reference ends.",
        tokens: ["@unchecked Sendable"],
      },
      {
        path: "packages/swift/Sources/ReallyMeCodec/StructuredResults.swift",
        digest: "062f35803af65430f981911568de13ea839dbae354455ec9e2cd5a6b16431e4a",
        reason: "The DER array is immutable after construction until its final reference is released; deinit wipes the owner without concurrent access.",
        tokens: ["@unchecked Sendable"],
      },
    ],
    verification: [
      { roles: ["format"], command: process.execPath, args: ["scripts/check_swift_source_style.mjs", "format"] },
      { roles: ["lint"], command: process.execPath, args: ["scripts/check_swift_source_style.mjs", "lint"] },
      { roles: ["build", "test"], command: "swift", args: ["test", "--arch", "arm64",
        "-Xswiftc", "-warnings-as-errors", "-Xswiftc", "-strict-concurrency=complete"] },
    ],
  });
  context.assertKotlinSourcePolicy({
    generatedPrefixes: ["gen/kotlin", "packages/kotlin/src/main/kotlin/me/really/codec/generated"],
    configuration: { files: [{
      path: "packages/kotlin/build.gradle.kts",
      required: ["explicitApi()", "allWarningsAsErrors.set(true)", 'version.set("1.5.0")'],
    }] },
    verification: [
      { roles: ["format", "static-analysis", "compile", "test"], command: "./gradlew",
        args: kotlinTestArgs, options: { cwd: "packages/kotlin", env: kotlinEnv } },
    ],
  });
}
