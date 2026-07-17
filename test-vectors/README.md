<!--
SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved

SPDX-License-Identifier: Apache-2.0
-->

# Codec Vectors

`codec-vectors.json` is the shared cross-language conformance suite for the
Rust, TypeScript, Swift, and Kotlin codec lanes.

The suite uses three provenance levels:

- `official`: vectors or rules published by an RFC or normative specification.
- `trusted-upstream`: vectors derived from upstream ecosystem specifications or
  registries, such as IPLD DAG-CBOR and multiformats multicodec.
- `reallyme-pinned`: ReallyMe-selected inputs whose expected outputs are frozen
  to catch cross-language drift. These are not represented as third-party
  vectors.

The manifest is intentionally JSON so every SDK lane can consume the same bytes
without generated-code coupling.

Schema version 2 adds shared rejection vectors. Every language lane must reject
the same non-canonical base encodings, unsupported multibase/multikey forms,
ambiguous or non-minimal DAG-CBOR, duplicate-member and malformed JCS text, and
exact integers outside the interoperable range. These are acceptance-policy
vectors: bindings may expose different typed error names, but none may accept
or normalize the input.
