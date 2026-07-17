<!--
SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved

SPDX-License-Identifier: Apache-2.0
-->

# Dependency Updates

Review codec dependency updates one at a time when they affect parsing,
serialization, protobuf generation, WASM bindings, JNI, or native ABI behavior.

Every update must pass the relevant Rust, TypeScript, Swift, and Kotlin codec
tests. Updates that change accepted input, canonical output, or error mapping
require focused negative tests.

The JVM and Android builds use locked dependency graphs and strict SHA-256
verification metadata. Do not accept a checksum merely because Gradle generated
it: corroborate new artifact hashes through a publisher-controlled source or an
independent trusted mirror and record that provenance during review.

PGP verification must not be enabled by blindly committing Gradle's
auto-trusted or auto-ignored bootstrap keys. Enabling it requires independently
verifying every trusted full fingerprint, scoping each key to its publisher,
committing the reviewed public-key ring, and retaining SHA-256 verification as
the integrity control and fallback for unsigned artifacts.
