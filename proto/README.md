<!--
SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved

SPDX-License-Identifier: Apache-2.0
-->

# Protobuf Contracts

The importable protobuf contract for ReallyMe codec errors and codec-owned
configuration values is owned by the publishable proto crate at
`crates/proto/codec/proto/reallyme/codec/v1/codec.proto`.

Generate code with:

```sh
cargo install protoc-gen-buffa --version 0.8.1 --locked
buf generate
```

The Buffa Rust output lives in `crates/proto/codec/src/generated/buffa`. SDK
outputs live under `gen/` and `packages/codec/src/proto/generated`.
