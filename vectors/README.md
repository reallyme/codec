# Codec Vectors

`codec-vectors.json` contains shared codec test inputs and expected outputs.

The suite uses three provenance levels:

Source labels are `official` for specification examples, `trusted-upstream` for upstream fixtures and `reallyme-pinned` for repository test cases.

Schema version 2 includes rejection cases. Tests must assert the indicated rejection outcome.

`deterministicCbor.fixtureClasses` labels each section by intent:

Fixture classes are `golden` for expected bytes, `rejection-fixture` for invalid input, `construction-recipe` for generated large inputs and `interop-fixture` for interoperability cases.

Interoperability cases require `fixtureKind`, `sourceRepo`, `sourceCommit`, `source`, `explanation`, `sourceFiles`, `hex`, `byteLength` and `sha256`. Source-file entries include SHA-256 hashes. Use synthetic data without PII or secrets.

Unsigned fixture integers are decimal strings to preserve values above `2^53 - 1`. `equivalentInputOrders` lists entry orders; `resourceRejections` specifies generated large inputs.

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

A scalar root has depth zero; a container root has depth one. Every value and map key counts as one node.
