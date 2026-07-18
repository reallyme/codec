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

The manifest also carries a structured `deterministicCbor` section containing
pinned positive, canonical-rejection, and interoperability literals for the
`0.2.0` profile. Its synthetic `idkit-ios` fixtures freeze byte-for-byte
compatibility with the existing consumer, including passport claims
optionality, the separate fingerprint map, and mixed integer claim-tag
ordering, without introducing passport-specific production behavior.

`deterministicCbor.fixtureClasses` labels each section by intent:

- `golden`: fixed literal canonical bytes that every lane must reproduce;
- `rejection-fixture`: fixed bytes that every lane must reject;
- `construction-recipe`: a language-neutral recipe for oversized or hostile
  inputs that are intentionally not committed as large literals;
- `interop-fixture`: a pinned consumer-shape fixture that must include
  provenance.

Every deterministic-CBOR interoperability fixture must include `fixtureKind`,
`sourceRepo`, `sourceCommit`, `source`, `explanation`, `sourceFiles`, `hex`,
`byteLength`, and `sha256`. Synthetic fixtures use
`sourceCommit: "content-hash-pinned"` plus SHA-256 digests for every source
file used to derive the bytes. Captured production artifacts must instead name
the source commit or release identifier and must not include PII or secrets.
The release gate runs `node scripts/validate_codec_vectors.mjs` to enforce
these fields, repository-relative source paths, lowercase digest syntax, and
the fixture byte-length/content digest relationship.

`equivalentInputOrders` uses an explicit fixture-only value notation so every
language can build the same ordered-entry semantic maps and prove that input
permutation does not affect canonical bytes. `resourceRejections` contains
exact, language-neutral construction recipes rather than committing megabytes
of repeated literal data. These manifest structures are test data only; they
are not a public JSON codec contract or an alternative to generated ProtoJSON.
Unsigned fixture integers are decimal strings so JSON runtimes cannot round
values above `2^53 - 1`.

Every positive deterministic-CBOR vector carries both an independent typed
`value` and its literal canonical `hex`. Conformance tests encode the declared
value directly and compare it with the committed bytes; decoder/encoder
round-trip agreement alone is insufficient because paired semantic defects can
otherwise agree with each other. The interoperability fixture additionally
pins SHA-256 digests of the exact external source files from which it was
captured, because that source snapshot is not identified by this repository's
commit history.

The positive-value notation is explicitly discriminated: `unsigned` and
`negative` contain decimal strings, `bytes` contains standard padded base64,
and recursive containers use `array` or `map`. A map contains an entry list;
each entry has a typed `key` and `value`. Bare JSON arrays never represent maps.

Resource construction recipe semantics are fixed as follows:

- `encoded-byte-count` creates exactly `count` bytes, each equal to
  `fillByteHex`;
- `byte-string-length` creates one semantic byte string containing exactly
  `count` zero bytes;
- `balanced-array-tree` creates `levels` array-container levels, with every
  array containing `branching` children and null leaves after the final level;
- `array-of-null` creates one array with exactly `count` null children;
- `nested-singleton-arrays` wraps null in exactly `depth` single-element
  arrays.

For deterministic-CBOR limits, a scalar root has nesting depth zero and a
root container has depth one. Every semantic value and every map key counts as
one node; map-entry wrappers and array slots add no separate nodes. The
`balanced-array-tree` recipe therefore contains one root array, `branching`
child arrays, and `branching * branching` null leaves when `levels` is two.
