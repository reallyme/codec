<!--
SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved

SPDX-License-Identifier: Apache-2.0
-->

# reallyme-codec-proto

`reallyme-codec-proto` contains the Rust Buffa bindings for
`reallyme.codec.v1`. The package is intentionally small: it publishes the
codec error envelope and fixed-shape codec result messages used by services and
SDK boundaries without pulling in the rest of the codec runtime.

This crate defines messages only; it intentionally declares no protobuf service.

JSON is a generated ProtoJSON request convenience. Results remain a binary protobuf result envelope.

```toml
[dependencies]
reallyme-codec-proto = { version = "0.1.22", features = ["generated"] }
```

The protobuf source is published with this crate at
[`proto/reallyme/codec/v1/codec.proto`](proto/reallyme/codec/v1/codec.proto).
Runtime codec operations still use the typed APIs and errors from
`reallyme-codec`. This crate owns messages and wire codecs; executable dispatch
belongs to `reallyme-codec::proto_process`, which accepts one
`CodecOperationRequest` and returns one `CodecProtoResultEnvelope`.
