# Protobuf API

The [schema](../crates/proto/proto/reallyme/codec/v1/codec.proto) defines Codec's
requests, results, options and errors. It declares no network service.

Rust callers can use `reallyme-codec` directly or enable `operation-contract`
for serialized requests. The generated Rust messages are available from
`reallyme-codec-proto` with its `generated` feature. TypeScript exposes generated
types through `@reallyme/codec/proto`; Swift and Kotlin provide SDK methods.

Encode one `CodecApiOperationRequest` and decode the returned
`CodecApiOperationResponse`. Its outcome contains either the operation's result
or a typed error with a reason and caller/provider origin. ProtoJSON requests
use the same operations and return binary protobuf responses.

Existing `CodecOperation*` envelopes remain supported. Consult the schema for
field presence, supported enum values and each operation's defaults.

## Definite TLV framing

TLV operations read identifiers, lengths and headers, or frame a complete value.
Choose the ISO 7816 definite BER or ASN.1 DER framing profile explicitly.
Neither profile validates nested contents; use schema-directed DER decoding
when value semantics must be checked.

Receipts contain spans into the supplied source. Keep that source alive while
using them. Exact framing rejects trailing bytes; prefix framing returns the
number of bytes consumed.

## Bounded Base64 decoding

Base64 decoding uses the standard alphabet with padding and rejects nonzero pad
bits. Input and output budgets are independent of the protobuf transport limit.
The schema specifies the options available for each operation.

## Data ownership

Messages and serialized buffers can contain personal data or key material.
Do not log generated messages or their JSON representations. Callers own the
lifetime and cleanup of input buffers and returned transport bytes. Managed
runtimes and serialization can create additional copies; see the
[memory ownership model](../SECURITY_MEMORY_MODEL.md) for the supported guarantees.
