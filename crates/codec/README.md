# reallyme-codec

Rust codecs for base64, base64url, lowercase hex, multiformats, CBOR, CID, JCS,
PEM, DER and definite TLV framing.

```toml
[dependencies]
reallyme-codec = "0.4.4"
```

```rust
use reallyme_codec::base64url::{base64url_to_bytes, bytes_to_base64url};

fn round_trip() -> Result<Vec<u8>, reallyme_codec::base64url::Base64UrlError> {
    base64url_to_bytes(&bytes_to_base64url(b"hello"))
}
```

Base encodings, CBOR/JCS, multiformats and PEM are enabled by default.
Select fewer families with `default-features = false`. Optional features are
`der`, `tlv`, `json`, `serde` and `operation-contract`; `serde` supplies
base64url field adapters, while `operation-contract` enables serialized calls.

`json::decode` returns a bounded JSON document; `json::from_slice` projects it
into an owned Serde target. Duplicate names, invalid Unicode, trailing data and
exceeded limits return typed errors. Callers own input storage and copies made
by target deserializers.

`base64::base64_to_bytes_with_limits` accepts schema-defined independent input
and output budgets and returns `Zeroizing<Vec<u8>>`. The existing decoder's
encoded-input default remains 2,097,152 bytes. Standard padding and zero pad
bits are required.

DER decoding checks values against a caller-supplied schema. TLV framing only
checks outer headers and extents; it does not validate nested ASN.1 values.
CID parsing checks syntax, not content integrity. Use `cbor::verify_dag_cbor_cid`
when verification is required.

See [API documentation](https://docs.rs/reallyme-codec) for types, profiles and
limits, and [data handling](https://github.com/reallyme/codec/blob/main/SECURITY_MEMORY_MODEL.md)
for cleanup obligations.

## License

MIT or Apache-2.0, at your option. See `LICENSE`.
