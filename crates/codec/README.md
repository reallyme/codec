# reallyme-codec

Rust byte-format and multiformat codecs.

Use the public APIs below for encoding and parsing. Cryptographic signing and verification are outside this crate.

## Install

```toml
[dependencies]
reallyme-codec = "0.1.20"
```

The default feature set enables every codec family. Consumers that need a
smaller dependency surface can select only the families they use:

```toml
[dependencies]
reallyme-codec = { version = "0.1.20", default-features = false, features = ["base64url", "multikey"] }
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
- `hex`
- `jcs`
- `multibase`
- `multicodec`
- `multikey`
- `pem`
- `serde`

Copyright © 2026 ReallyMe Technologies Limited. All rights reserved
