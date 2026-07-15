# reallyme-codec-proto

Generated Rust bindings for codec errors and results in `reallyme.codec.v1`.

```toml
[dependencies]
reallyme-codec-proto = { version = "0.1.20", features = ["generated"] }
```

The protobuf source lives at
[`proto/reallyme/codec/v1/codec.proto`](../../../proto/reallyme/codec/v1/codec.proto).
Runtime codec operations still use the typed errors from `reallyme-codec`; this
crate is for wire/config contracts, binary FFI result envelopes, and generated
protobuf integration.

Copyright © 2026 ReallyMe Technologies Limited. All rights reserved
