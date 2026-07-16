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
