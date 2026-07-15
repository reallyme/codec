<!--
SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved

SPDX-License-Identifier: Apache-2.0
-->

# Protobuf Contracts

This directory contains the importable protobuf contract for ReallyMe codec
errors and codec-owned configuration values.

Generate code with:

```sh
cargo install protoc-gen-buffa --version 0.8.1 --locked
buf generate
```

The Buffa Rust output lives in `crates/proto/codec/src/generated/buffa`. SDK
outputs live under `gen/` and `packages/codec/src/proto/generated`.
