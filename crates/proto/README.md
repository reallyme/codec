# reallyme-codec-proto

Generated Rust messages for the `reallyme.codec.v1` codec API. This crate
defines messages and serialization; it declares no service and executes no
codec operations.

JSON is a generated ProtoJSON request convenience. Results remain one fully
discriminated binary protobuf response.

```toml
[dependencies]
reallyme-codec-proto = { version = "0.4.6", features = ["generated"] }
```

The [schema](proto/reallyme/codec/v1/codec.proto) ships with the crate. For
execution, enable `operation-contract` on `reallyme-codec` and use
`reallyme_codec::operation_contract`.

Generated messages implement ReallyMe Buffa 0.1.3 traits under the `buffa`
dependency alias. Sensitive owned fields clear on drop. Borrow fields or use
builders and `mem::take` when transferring ownership from a message with `Drop`.
Do not log messages or serialized JSON containing sensitive fields.

## License

MIT or Apache-2.0, at your option. See [LICENSE](LICENSE).
