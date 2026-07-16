# Protobuf Contracts

The importable protobuf contract for ReallyMe codec errors and codec-owned
configuration values is owned by the publishable proto crate at
`crates/proto/codec/proto/reallyme/codec/v1/codec.proto`.

Generate code with:

```sh
cargo install protoc-gen-buffa --version 0.8.1 --locked
buf generate
```

Copyright © 2026 ReallyMe Technologies Limited. All rights reserved
