# reallyme-codec-proto

Generated Rust bindings for codec requests, results, options and errors in `reallyme.codec.v1`.

This crate declares protobuf messages, not a service.

JSON is a generated ProtoJSON request convenience. Results remain one fully
discriminated binary `CodecOperationResponse`.

```toml
[dependencies]
reallyme-codec-proto = { version = "0.2.3", features = ["generated"] }
```

The protobuf source is published with this crate at
[`proto/reallyme/codec/v1/codec.proto`](proto/reallyme/codec/v1/codec.proto).
Runtime codec operations still use the typed APIs and errors from
`reallyme-codec`. This crate owns messages and wire codecs; executable dispatch
belongs to `reallyme-codec::operation_contract` (enable the `operation-contract` feature),
which accepts one
`CodecOperationRequest` and returns one fully discriminated
`CodecOperationResponse`.

## License

Dual-licensed under the MIT License or Apache License, Version 2.0, at your
option. See [LICENSE](LICENSE) for both license texts.

Copyright © 2026 ReallyMe Technologies Limited. All rights reserved
