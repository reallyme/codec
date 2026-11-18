# reallyme-codec-proto

Generated Rust bindings for codec requests, results, options and errors in `reallyme.codec.v1`.

This crate declares protobuf messages, not a service.

JSON is a generated ProtoJSON request convenience. Results remain one fully
discriminated binary `CodecOperationResponse` for existing operations, or the expanded
`CodecApiOperationResponse`.

```toml
[dependencies]
reallyme-codec-proto = { version = "0.4.4", features = ["generated"] }
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

## Generated runtime compatibility

The `generated` feature exposes message types produced by ReallyMe Buffa
0.1.3 under the `buffa` dependency alias. Native cleanup messages implement
`Drop`; bytes and strings use wipe-owning storage, and repeated fields use
guarded collections. Borrow fields or use builders and `mem::take` when
transferring ownership. Those types and the public `encode_protobuf` and `decode_protobuf`
functions implement Buffa traits, so a Buffa version change can alter this
crate's source API even when the protobuf schema is unchanged. Treat such a
runtime upgrade as a compatibility review, regenerate every language binding,
and run downstream compile tests before publishing. The wire schema remains
the cross-language contract; callers should not depend on Buffa-owned fields
or views for long-lived domain models.

## License

Dual-licensed under the MIT License or Apache License, Version 2.0, at your
option. See [LICENSE](LICENSE) for both license texts.
