# reallyme-codec

Rust codecs for base encodings, multiformats, CBOR, CID, JCS and PEM.

Use the public APIs below for encoding and parsing. Cryptographic signing and verification are outside this crate.

## Install

```toml
[dependencies]
reallyme-codec = "0.4.4"
```

The default feature set enables the base-encoding, CBOR/JCS, multiformat, and PEM
families. Consumers that need a smaller dependency surface can select only the
families they use:

```toml
[dependencies]
reallyme-codec = { version = "0.4.4", default-features = false, features = ["base64url", "multikey"] }
```

## Quick Start

```rust
use reallyme_codec::base64url::{base64url_to_bytes, bytes_to_base64url};

fn round_trip() -> Result<(), reallyme_codec::base64url::Base64UrlError> {
    let encoded = bytes_to_base64url(b"hello");
    let decoded = base64url_to_bytes(&encoded)?;
    assert_eq!(decoded, b"hello");
    Ok(())
}
```

Multikey decoding does not perform algorithm-specific cryptographic key validation.

`cbor::is_valid_cid_string` and `cbor::try_parse_cid` check bounded CID syntax.
CIDv0 requires a complete 32-byte SHA-256 multihash. CIDv1 preserves opaque hash
codes and 0–64 digest bytes when the declared length matches, including unknown
codes, truncated hashes, and identity hashes. Parsing makes no hash-support or
content-integrity claim. `cbor::verify_dag_cbor_cid` validates canonical DAG-CBOR
and matches only its CIDv1 DAG-CBOR/full-SHA-256 identity in lowercase base32.

DTOs that carry byte fields as unpadded base64url strings can enable the
`serde` feature and use `reallyme_codec::base64url::serde_bytes` or
`reallyme_codec::base64url::serde_option_bytes`.

## Features

- `base64`
- `base64url`
- `cbor`
- `der` — opt-in strict, bounded DER readers for Rust consumers
- `tlv` — opt-in borrowed definite BER/ISO 7816 and canonical DER header framing
- `hex`
- `jcs`
- `multibase`
- `multicodec`
- `multikey`
- `pem`
- `serde` — base64url serde field adapters
- `operation-contract` — generated binary protobuf and ProtoJSON dispatch

`der`, `tlv`, `serde`, and `operation-contract` are opt-in; they are not part of the default
feature set. The operation contract enables its required codec families.

The `der` module provides borrowed, schema-directed readers and a strict
SEQUENCE-of-two-unsigned-INTEGER helper. Canonical tags, definite minimal
lengths, INTEGER encodings, BIT STRING padding, OID subidentifiers, SET OF
ordering, and complete consumption are enforced. Input size, nesting, element
count, and integer lengths are bounded. The Rust API documentation specifies
the supported ASN.1 types and resource ceilings. Curve-specific scalar widths,
subgroup-order checks, and signature high-S policy remain in the crypto layer.
The DER reader does not expose a Swift, Kotlin, or TypeScript facade.

### Borrowed TLV framing

The `tlv` feature provides `tlv::read_identifier`, `tlv::read_length`,
`tlv::read_header`, `tlv::frame_prefix`, and `tlv::frame_exact`. Options and
profiles are generated from the protobuf schema; choose a profile explicitly.
The default source limit is 1 MiB, with explicit limits up to 16 MiB.
Prefix and header reads can succeed before the value arrives. Frames expose
exact borrowed received/value bytes and checked
source spans; prefix framing also returns the unconsumed suffix. No payload is
copied or allocated from a declared length. Receipt metadata is bounded.

The ISO 7816 profile retains wire identifiers and redundant definite lengths.
The DER framing profile checks canonical outer headers only; constructed
contents remain opaque. The existing `der` schema reader remains strict and
provides recursive value and canonicality checks. See
[the protobuf contract](../../docs/protobuf.md#definite-tlv-framing) for limits,
receipts, and typed failures.

### Guarded JSON decoding

With the `json` feature, `reallyme_codec::json::decode` executes the
protobuf-defined `CodecJsonDecodeRequest` contract and returns a guarded
`CodecJsonDecodeResult`. `json::from_slice` projects that same admitted document
into an owned Serde target, including generated protocol messages. The target
must retain wiping ownership for its sensitive fields.

Decoded strings, escaped member names, number text, and partial output have
cleanup owners. Duplicate decoded names, invalid Unicode, non-finite numbers,
trailing data, and exhausted byte/depth/entry/node/string/work budgets are
rejected with canonical `CodecErrorReason` values. Input buffers remain owned
by the caller. JSON number text retains its original spelling in the generated
result; native projections use integer and finite floating-point visitors.

### Bounded Base64 decoding

`base64::base64_to_bytes_with_limits` takes schema-generated `Base64DecodeLimits`
and returns `Zeroizing<Vec<u8>>`. Encoded-input and decoded-output byte budgets
are independent, and checked before reserving output. Standard padded Base64
and canonical pad bits are required. The original decoder retains its
2,097,152-byte encoded-input default. See the [protobuf contract](../../docs/protobuf.md#bounded-base64-decoding)
for limits, typed failures, and the shared native/SDK operation.

## License

Dual-licensed under the MIT License or Apache License, Version 2.0, at your
option. See the crate's `LICENSE` file for both license texts.
