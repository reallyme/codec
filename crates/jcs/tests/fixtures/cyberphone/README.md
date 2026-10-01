# JCS reference fixtures

These input and output pairs come from `testdata/{input,output}` in
[cyberphone/json-canonicalization](https://github.com/cyberphone/json-canonicalization)
at commit `19d51d7fe467d4706a3ff08adf8a748f29fc21e0`. The original notice is
in [LICENSE](LICENSE).

`upstream_conformance_tests.rs` pins SHA-256 over each relative path, a NUL
separator, and its unmodified bytes, with all input files followed by all
output files in filename order. The expected digest is
`4f464dc75b68c8310a2e18b64b388342bcce2dd90b5ffa55a169c5b628a6b2be`.

The `values.json` sample includes `1E30`. ReallyMe Codec's documented strict
I-JSON integer policy rejects that sample with a typed error. The other five
pairs must match the upstream canonical bytes exactly.
