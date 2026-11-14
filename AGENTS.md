# ReallyMe Codec Agent Rules

This repository is production-grade, security-sensitive codec infrastructure. Treat every change as audit-facing and long-lived.

## Operating Principles

- Do not write demo, placeholder, shortcut, or temporary production code.
- Preserve user work. Never revert unrelated local changes unless explicitly asked.
- Prefer clarity, correctness, and explicit boundaries over cleverness or brevity.
- Keep primitive code separate from dispatch, FFI, protocols, and SDK facades.
- Keep `lib.rs` and `mod.rs` thin. Put substantive logic in focused, verb-named files.
- Avoid wildcard imports and wildcard exports.
- Keep ReallyMe copyright notices. Existing SPDX headers must pair `SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited` with `SPDX-License-Identifier: MIT OR Apache-2.0`; Markdown and text files must not carry SPDX headers. Package license metadata remains `MIT OR Apache-2.0`.
- Public documentation must read like release-quality infrastructure documentation: clear, restrained, and free of internal process notes.

## Protobuf Contract

`crates/proto/proto/reallyme/codec/v1/codec.proto` is the single source of truth for the public codec API across Rust, FFI, WebAssembly, Swift, Kotlin, and TypeScript. Implementations follow the contract; none defines a parallel one.

- Define every public operation in protobuf before implementing or extending it. Each codec operation has a request message, a result message, and a matching `CodecApiOperationResult` branch. Host provider lifecycle operations use `CodecProviderOperation*`; their I/O and opaque runtime handles stay in the host adapter. Preserve the original `CodecOperation*` envelopes as wire-compatible facades; new codec operations belong to the expanded `CodecApiOperation*` envelope.
- Define every public enum in protobuf: codecs, multicodec names, base encodings, PEM labels, key types, canonicalization modes, policy values, and status values. Do not add a hand-maintained Rust, Swift, Kotlin, or TypeScript enum that restates a schema enum. Where Rust needs a closed type, convert from the generated enum through a single total, tested `TryFrom` that rejects `UNSPECIFIED` and unknown values with the typed error. That conversion is the only place the value set may be restated.
- Define every public request, result, options, and policy structure as a protobuf message. Public structs in any lane carrying the same data are thin views over, or constructors of, the generated message, not independent definitions.
- Use native protobuf presence (`optional`, message fields, or `oneof`) when absence differs from a default value. Do not add hand-written presence booleans. Treat `UNSPECIFIED` as absence only when the operation explicitly permits it. Preserve published default semantics and reserve retired field numbers and names during migrations.
- Define every public failure in `CodecError`, in the branch for its area, with its origin. Each `thiserror` variant maps to exactly one schema error, and a test asserts that the mapping is total. Do not add an error variant without its schema code. SDK lanes surface the generated error, not a hand-written equivalent.
- Allocate new field numbers, enum values, operation identifiers, and error codes in the band reserved for their family. Never renumber or reuse; reserve retired numbers and names. `buf lint` and `buf breaking` must pass.
- Keep implementation behind the generated operation contract. Borrowed Rust, FFI, WebAssembly, and SDK signatures are permitted only as thin adapters sharing the contract's semantic implementation; do not maintain a separate operation or policy path in any lane.
- Do not expose a public operation, enum, or error in any lane that is not reachable from the schema. Release-readiness checks enforce this boundary.
- Mark secret-bearing schema fields `SENSITIVE`. Every sensitive field must be covered by native cleanup, guarded field storage, explicit JSON policies, and the sensitivity classification audit. Adding a sensitive field requires corresponding cleanup and regression coverage in the same change.
- Keep sensitive messages in the Buffa `borrowed_views_only_in` policy. Buffa applies the restriction to containing messages as well.
- Generated code for every lane is produced by the pinned generators in `buf.gen.yaml` and checked in. Do not edit it by hand. Generated Rust must be the unmodified output of the native generator. The separate SDK safeguarding pass may modify only Swift/Java presentation and admission behavior that their generators cannot enforce.
- Add generated/native parity coverage for bytes, metadata, and typed failures in every lane that exposes the operation. Preserve independent vectors and negative assertions when adapting tests.

## Architectural Ownership And Toolchain

- Buf is the only schema lint, breaking-change, and generation orchestrator. The approved Rust runtime and generators are the published ReallyMe Buffa family at exact `=0.1.1`. Preserve Rust dependency aliases (`buffa`, `buffa-types`, and so on); add only packages actually used. Upstream Buffa and competing Rust protobuf runtimes are prohibited without explicit architectural approval.
- Install `reallyme-protoc-gen-buffa` and `reallyme-protoc-gen-buffa-packaging` with `cargo install --version '=0.1.1' --locked`. Run `node scripts/generate_codec_proto.mjs`; its guards require the renamed executable names and exact version output. Coordinate any toolchain change across runtime features, generators, manifests, lockfiles, enforcement, documentation, and every generated lane.
- Protobuf owns serialized application contracts, API requests/results/errors, messages, events, and persisted records. Use generated serialization and ProtoJSON. Do not add parallel DTOs, hand-written protobuf wire/JSON mappings, or hand-made presence flags.
- Preserve genuine validated Rust invariants: private validated wrappers, fixed-size keys/digests, borrowed readers, and runtime ownership guards are appropriate where generated mutable messages cannot enforce them. Do not duplicate their serialization model or introduce schemas for locks, runtime handles, or parser state.
- Consumers use the corresponding ReallyMe Crypto, COSE, JOSE, and Codec libraries. This foundational codec repository owns its codec primitives, CID hashing, provider integrity checks, and package implementations; appropriate underlying dependencies and its C ABI/JNI/WASM bridges belong here. Do not add unrelated signing, encryption, COSE, or JOSE implementations.
- Applications consume published language-native ReallyMe crates, npm/Maven artifacts, and Swift packages. Do not add an FFI bridge merely to bypass a suitable published native package. Verify capabilities, platforms, features, security, and behavior before replacing a dependency. Internal workspace paths, fuzzing, and feature probes intentionally test this repository's current source and must not be replaced with a dependency on their own published package.
- Prefer published crates.io dependencies over Git, path, or vendored alternatives outside that internal development workflow. Missing shared capabilities require a precise upstream finding; do not silently invent substitute packages or workaround architectures.
- Release Readiness 0.7.0 is the shared enforcement authority. Run its immutable official runner, authenticate its exact package inputs, invoke all applicable Rust/TypeScript/Swift/Kotlin source policies, and reuse shared checks. Do not vendor or modify the core. Local policy may cover only documented shared capability gaps, including native guarded ReallyMe Buffa generation; do not modify another repository without authorization.
- Authored production files must stay at or below 500 lines, separate tests at or below 800, and module/facade files at or below 100. Keep tests outside production files. Generated exclusions must identify actual generated paths, never authored implementation.
- The approved temporary exception is `packages/ts/src/serializeExactBinary.ts`: its generated-descriptor serializer preserves one exact wipe-owned output because protobuf-es 2.17.0 lacks an equivalent writer hook or internal-buffer cleanup. Preserve all parity/allocation/failure tests. This exception expires when the selected runtime provides equivalent demonstrated cleanup. Do not broaden the exception.
- New exceptions require explicit approval, exact affected files, technical alternatives, compatibility/security implications, and a remediation or expiry condition. Never self-approve an exception or treat old violations as grandfathered in.

## Rust

- `cargo fmt --check` and `cargo clippy --workspace --all-targets --all-features -- -D warnings` must pass unless an exception is explicitly accepted.
- No `unwrap`, `expect`, `panic`, `unreachable`, `todo`, `unimplemented`, `dbg!`, `println!`, `anyhow`, `eyre`, `Result<T, String>`, `Result<T, &str>`, `Result<T, ()>`, or `type Err = ()` in production paths.
- Test-only lint allowances must be narrow and local.
- Use domain-specific typed errors with `thiserror`. Error variants must not carry secrets, PII, raw buffers, arbitrary strings, or heap-erased errors.
- Use enums over strings for codecs, algorithms, suites, modes, and status values, defined in the protobuf schema.
- Name security-sensitive constants: lengths, suite IDs, protocol versions, prefixes, and limits.
- Use checked arithmetic for buffer sizes, offsets, and capacities. Use `TryFrom` for sensitive length conversions; do not use `as` there.
- Be conservative with `pub`; expose only the supported API surface.
- New crates inherit `publish = false` unless they are part of the approved public package set.
- Member crates must inherit workspace lints with `[lints] workspace = true`.
- `unsafe_code` is denied by default. FFI boundary modules may opt in only with narrow scope and `SAFETY:` comments that state pointer, length, lifetime, aliasing, and ownership assumptions.

## Secrets And Memory

- Secret-bearing material needs an explicit owner model and zeroization on drop.
- Prefer fixed-size arrays for fixed-size keys and outputs.
- Use `Zeroizing<T>`, `ZeroizeOnDrop`, or an equivalent wrapper for long-lived keys, passwords, KDF inputs, shared secrets, plaintext, seeds, salts with privacy impact, and derived keys.
- Avoid long-lived raw `Vec<u8>` for private keys, passwords, seed material, shared secrets, and derived keys.
- Secret-bearing types must not derive `Debug`, `Display`, `Serialize`, `Deserialize`, `Clone`, or `Copy` unless the behavior is deliberately safe and documented.
- FFI and platform code must not transfer private key material or decrypted plaintext into unmanaged memory without a matching destroy or zeroize path.

## Runtime Lanes

- The codec core stays pure Rust and must not depend on Swift, Kotlin, TypeScript, WebAssembly glue, JNI glue, or C ABI glue.
- Native SDKs call Rust through the C ABI/JNI surface in `crates/ffi`.
- The TypeScript package calls Rust through the WebAssembly crate in `crates/wasm`.
- Rust crates must not expose cargo features named `swift` or `kotlin`.
- Swift and Kotlin provider selection belongs in `packages/swift` and `packages/kotlin`, not in Rust cargo features.
- A missing SDK provider must fail closed with a typed unsupported operation or unavailable provider error. No silent fallback.
- Provider policy changes require code, tests, and matrix updates together.

## FFI

- Use the shared pointer/length validation helpers. Do not add ad hoc boundary validation.
- Catch panics at every exported boundary and map them to deterministic status codes.
- Validate null pointers, short buffers, oversized buffers, output length mismatches, and aliasing.
- Never put secrets, raw inputs, or backend exception text in FFI errors.
- Zeroize secret input/output buffers when ownership and API semantics permit it.

## TypeScript

- Use strict mode. Do not use `any` or `@ts-ignore`.
- Avoid `as` casts except in tests or provably safe narrowing.
- Treat external data, JSON, I/O, and WASM/FFI input as `unknown` until validated.
- For codec byte and enum boundaries, prefer small hand-written validators over broad schema dependencies. Use schema libraries only when a structured untrusted JSON boundary genuinely needs them.
- Use typed domain errors or typed `Result` values for expected failures. Do not throw strings or generic errors from core paths.
- Do not include PII, secrets, raw buffers, or untrusted input in error messages or telemetry metadata.
- Use `Uint8Array` for sensitive bytes and call `.fill(0)` as soon as practical.
- Prefer `Readonly<T>` and `ReadonlyArray<T>` for domain objects.
- Use `Map` or `Object.create(null)` for ad hoc maps.
- Tests must cover malformed, oversized, and malicious inputs.

## Vectors And Tests

- Every exposed codec operation needs happy-path and negative coverage.
- Negative tests must assert stable typed errors, not backend-specific exception text.
- Cover invalid lengths and encodings, non-canonical encodings, malformed envelopes, wrong labels, invalid CIDs, unsupported codecs or algorithms, empty inputs, maximum accepted inputs, and boundary arithmetic.
- Cross-lane vectors should prove byte-for-byte parity across Rust, Swift, Kotlin, and TypeScript whenever a facade exposes the operation.
- If a lane intentionally does not implement an algorithm, test the typed unsupported behavior.

## Required Validation

Before claiming work is complete, run the relevant subset and report any skipped command with the residual risk:

```sh
cargo fmt --check
cargo check --workspace --all-features
RUSTFLAGS=-Dwarnings cargo check --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo nextest run --workspace --all-features
cargo test --workspace --all-features
cargo check -p reallyme-codec-wasm --target wasm32-unknown-unknown
node scripts/check_release_readiness.mjs
node --test --test-concurrency=4 "scripts/**/*.test.mjs" "packages/ts/scripts/*.test.mjs"
cargo deny check
cargo audit
```

When touching SDK packages, also run the matching package tests:

```sh
swift test
packages/kotlin/gradlew -p packages/kotlin test
npm --prefix packages/ts test
```

## Repository Attribution

Do not add AI assistant authorship, generation credit, identity markers, or branding to repository files, generated output, Git metadata, or collaboration artifacts. References to local assistant directories such as `/.claude/` and `/.codex/` are allowed in `.gitignore`, `.dockerignore`, and this rule when describing ignore behavior. Factual references to third-party projects, tools, maintainers, and source URLs are allowed where relevant, including README links such as `anthropics/buffa`; they are not AI authorship attribution. Apply attribution checks to proposed changes and current repository content without treating those permitted references in historical commits or tags as blockers or requiring history rewrites.
