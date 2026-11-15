# reallyme-codec-proto

Generated Rust bindings for codec requests, results, options and errors in `reallyme.codec.v1`.

This crate declares protobuf messages, not a service.

JSON is a generated ProtoJSON request convenience. Results remain one fully
discriminated binary `CodecOperationResponse` for existing operations, or the expanded
`CodecApiOperationResponse`.

```toml
[dependencies]
reallyme-codec-proto = { version = "0.4.3", features = ["generated"] }
```

The protobuf source is published with this crate at
[`proto/reallyme/codec/v1/codec.proto`](proto/reallyme/codec/v1/codec.proto).
Runtime codec operations still use the typed APIs and errors from
`reallyme-codec`. This crate owns messages and wire codecs; executable dispatch
belongs to `reallyme-codec::operation_contract` (enable the `operation-contract` feature),
which accepts `CodecApiOperationRequest` and returns a fully discriminated
`CodecApiOperationResponse`. The original `CodecOperationRequest`,
`CodecOperationResult`, `CodecOperationResponse`, and `CodecError` retain their
wire shapes. Their field numbers and payload types are shared with the
expanded envelope, so existing requests and responses keep the same wire bytes.
Both envelopes use the same runtime entry points and semantic implementations.

## License

Dual-licensed under the MIT License or Apache License, Version 2.0, at your
option. See [LICENSE](LICENSE) for both license texts.

## Generated runtime compatibility

Generated types implement Buffa traits. Keep the matching runtime version. When a message implements `Drop`, borrow its fields or use builders and `mem::take` to transfer ownership.
