# Test vectors

`codec-vectors.json` supplies inputs, expected bytes and failures for the Rust,
TypeScript, Swift and Kotlin tests. Validate edits with:

```sh
node scripts/validate_codec_vectors.mjs
```

Sources use `official` for specification examples, `trusted-upstream` for
upstream fixtures and `reallyme-pinned` for this repository's test cases.
Deterministic-CBOR sections use `golden`, `rejection-fixture`,
`construction-recipe` and `interop-fixture` to distinguish expected outcomes.
Keep source references and fixture hashes when updating imported data.

Typed values use decimal strings for `unsigned` and `negative`, padded base64
for `bytes`, and explicit `array` or `map` containers. Maps contain key/value
entry lists. This notation is test data, not a public codec API.

Resource recipes have these meanings:

- `encoded-byte-count`: `count` copies of `fillByteHex`.
- `byte-string-length`: one byte string containing `count` zero bytes.
- `balanced-array-tree`: `levels` array levels, each with `branching` children,
  followed by null leaves.
- `array-of-null`: one array of `count` null values.
- `nested-singleton-arrays`: null wrapped in `depth` one-element arrays.

A scalar root has depth zero; a container root has depth one. Every value and
map key counts as one node.
