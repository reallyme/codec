# reallyme-codec

`reallyme-codec` is the recommended Rust entry point for the ReallyMe Codec
workspace. It provides identity encoding utilities without pulling in
signature, AEAD, KEM, or password-hashing implementations.

Use this crate when a resolver, service, or tool needs key and content
encodings but does not need cryptographic operations. The supported surface is
algorithm-agnostic: PEM armor, base64/base64url, lowercase hex, multibase,
multicodec, multikey, canonical CBOR/DAG-CBOR helpers, and JSON
Canonicalization Scheme helpers.

## Install

```toml
[dependencies]
reallyme-codec = "0.4.0"
```

The default feature set enables the base-encoding, CBOR/JCS, multiformat, and PEM
families. Consumers that need a smaller dependency surface can select only the
families they use:

```toml
[dependencies]
reallyme-codec = { version = "0.4.0", default-features = false, features = ["base64url", "multikey"] }
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

Multikey support treats keys as opaque public bytes plus a multicodec prefix.
Algorithm-aware key parsing, signing, verification, and JWK envelopes live in
the crypto layer that consumes the decoded bytes.

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

## Package Contract

ReallyMe Codec is one cross-language codec contract for identity data. The
publishable Rust leaf crates exist for dependency hygiene, implementation
modularity, and crates.io dependency resolution. They are support crates
released in lockstep with `reallyme-codec`.

Rust consumers should usually depend on this umbrella crate. Direct use of a
leaf crate is appropriate only when the consumer deliberately needs a smaller
primitive surface and accepts the same lockstep ReallyMe Codec release line.

## License

Dual-licensed under the MIT License or Apache License, Version 2.0, at your
option. See the crate's `LICENSE` file for both license texts.

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
