<!--
SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved

SPDX-License-Identifier: Apache-2.0
-->

# reallyme-codec-proto

`reallyme-codec-proto` contains the Rust Buffa bindings for
`reallyme.codec.v1`. The package is intentionally small: it publishes the
codec error envelope and fixed-shape codec result messages used by services and
SDK boundaries without pulling in the rest of the codec runtime.

```toml
[dependencies]
reallyme-codec-proto = { version = "0.1.21", features = ["generated"] }
```

The protobuf source is published with this crate at
[`proto/reallyme/codec/v1/codec.proto`](proto/reallyme/codec/v1/codec.proto).
Runtime codec operations still use the typed errors from `reallyme-codec`; this
crate is for wire/config contracts, binary FFI result envelopes, and generated
protobuf integration.
