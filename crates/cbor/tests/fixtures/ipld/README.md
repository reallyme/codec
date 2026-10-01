# IPLD DAG-CBOR fixtures

These five binary fixtures are copied without modification from
`_fixtures_src` in [ipld/codec-fixtures](https://github.com/ipld/codec-fixtures)
at commit `b676336faded408b2a5fc48baaab7fd6b04a4472`. The source names are
`array-2.dag-cbor`, `false.dag-cbor`, `int-255.dag-cbor`,
`map-1_pair.dag-cbor`, and `string-a.dag-cbor`; only the copied map filename
uses a hyphen in place of the underscore. The fixture repository is dual
licensed under [Apache 2.0](LICENSE-APACHE) and [MIT](LICENSE-MIT).

`upstream_fixture_tests.rs` pins SHA-256 over each copied filename, a NUL
separator, and its bytes in filename order. The expected digest is
`2e92a7371662a7c2060079f2485c68101887bb04f2cd94830f7e3001353f69e9`.
The tests also compare decoded and re-encoded bytes and the published fixture
CIDs. A duplicate-key negative vector comes from
`negative-fixtures/dag-cbor/decode/duplicate-keys.json` at the same commit.
