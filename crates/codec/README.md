# reallyme-codec

Rust codecs for base encodings, multiformats, CBOR, CID, JCS and PEM.

Use the public APIs below for encoding and parsing. Cryptographic signing and verification are outside this crate.

## Install

```toml
[dependencies]
reallyme-codec = "0.3.4"
```

The default feature set enables the base-encoding, CBOR/JCS, multiformat, and PEM
families. Consumers that need a smaller dependency surface can select only the
families they use:

```toml
[dependencies]
reallyme-codec = { version = "0.3.4", default-features = false, features = ["base64url", "multikey"] }
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

DTOs that carry byte fields as unpadded base64url strings can enable the
`serde` feature and use `reallyme_codec::base64url::serde_bytes` or
`reallyme_codec::base64url::serde_option_bytes`.

## Features

- `base64`
- `base64url`
- `cbor`
- `der` — opt-in strict, bounded DER readers for Rust consumers
- `hex`
- `jcs`
- `multibase`
- `multicodec`
- `multikey`
- `pem`
- `serde` — base64url serde field adapters
- `operation-contract` — generated binary protobuf and ProtoJSON dispatch

`der`, `serde`, and `operation-contract` are opt-in; they are not part of the default
feature set. The operation contract enables its required codec families.

The `der` module provides borrowed, schema-directed readers and a strict
SEQUENCE-of-two-unsigned-INTEGER helper. Canonical tags, definite minimal
lengths, INTEGER encodings, BIT STRING padding, OID subidentifiers, SET OF
ordering, and complete consumption are enforced. Input size, nesting, element
count, and integer lengths are bounded. The Rust API documentation specifies
the supported ASN.1 types and resource ceilings. Curve-specific scalar widths,
subgroup-order checks, and signature high-S policy remain in the crypto layer.
The DER reader does not expose a Swift, Kotlin, or TypeScript facade.

## License

Dual-licensed under the MIT License or Apache License, Version 2.0, at your
option. See the crate's `LICENSE` file for both license texts.
