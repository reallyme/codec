# reallyme-codec-proto

Generated Rust bindings for codec errors and result messages in `reallyme.codec.v1`.

This crate declares protobuf messages, not a service.

JSON is a generated ProtoJSON request convenience. Results remain one fully
discriminated binary `CodecOperationResponse`.

```toml
[dependencies]
reallyme-codec-proto = { version = "0.2.1", features = ["generated"] }
```

The protobuf source is published with this crate at
[`proto/reallyme/codec/v1/codec.proto`](proto/reallyme/codec/v1/codec.proto).
Runtime codec operations still use the typed APIs and errors from
`reallyme-codec`. This crate owns messages and wire codecs; executable dispatch
belongs to `reallyme-codec::operation_contract`, which accepts one
`CodecOperationRequest` and returns one fully discriminated
`CodecOperationResponse`.

Copyright © 2026 ReallyMe Technologies Limited. All rights reserved
