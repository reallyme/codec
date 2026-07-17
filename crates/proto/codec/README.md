# reallyme-codec-proto

Generated Rust bindings for codec errors and results in `reallyme.codec.v1`.

This crate declares protobuf messages, not a service.

JSON is a generated ProtoJSON request convenience. Results remain a binary protobuf result envelope.

```toml
[dependencies]
reallyme-codec-proto = { version = "0.1.22", features = ["generated"] }
```

The protobuf source is published with this crate at
[`proto/reallyme/codec/v1/codec.proto`](proto/reallyme/codec/v1/codec.proto).
Runtime codec operations still use the typed APIs and errors from
`reallyme-codec`. This crate owns messages and wire codecs; executable dispatch
belongs to `reallyme-codec::proto_process`, which accepts one
`CodecOperationRequest` and returns one `CodecProtoResultEnvelope`.

Copyright © 2026 ReallyMe Technologies Limited. All rights reserved
