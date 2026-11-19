# Codec contract

The [protobuf schema](crates/proto/proto/reallyme/codec/v1/codec.proto) defines
Codec's public operation requests, results, options and error reasons. Rust and
SDK packages share the same release version. Use matching versions when passing
serialized operations between languages.

Native Rust APIs are documented on [docs.rs](https://docs.rs/reallyme-codec).
See the [protobuf API](docs/protobuf.md) for serialized calls and
[data handling](SECURITY_MEMORY_MODEL.md) for caller responsibilities.

Repository maintenance follows the ReallyMe
[engineering rules](https://github.com/reallyme/.github/blob/main/ENGINEERING_PHILOSOPHY.md)
and [architecture](https://github.com/reallyme/.github/blob/main/ARCHITECTURE.md).
