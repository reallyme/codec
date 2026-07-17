<!--
SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved

SPDX-License-Identifier: Apache-2.0
-->

# Fuzzing Harnesses

Coverage-guided libFuzzer targets for the untrusted-input parsers: the places
that turn attacker-controlled bytes into structured values. Every target asserts
the same baseline safety property: arbitrary input must not panic, overflow,
read out of bounds, or run unbounded. Parsing fails closed with a typed error.

This crate is intentionally outside the main Cargo workspace. It declares its
own empty `[workspace]` so it does not inherit lint settings that libFuzzer's
`#![no_main]` runtime would violate. The root workspace lists `fuzz` under
`[workspace] exclude`.

## Targets

| Target | Parser under test | Entry point |
| --- | --- | --- |
| `multibase` | multibase + base58btc decode | `codec_multibase::{multibase_to_bytes, base58btc_decode}` |
| `multicodec` | multicodec varint prefix | `codec_multicodec::{lookup_codec_prefix, strip_codec_prefix}` |
| `multikey` | multikey (multibase+multicodec+binding) | `codec_multikey::parse_multikey` |
| `base64url` | unpadded base64url decode | `codec_base64url::base64url_to_bytes` |
| `dag_cbor` | DAG-CBOR decode + CID parse/verify | `codec_cbor::{decode_dag_cbor, try_parse_cid, verify_dag_cbor_cid}` |
| `proto_process` | executable protobuf + generated ProtoJSON dispatch | `reallyme_codec::proto_process::{process_proto, process_proto_json}` |
| `jcs_text` | strict JSON parsing + RFC 8785 canonicalization | `codec_jcs::canonicalize_json_text` |

## Running

Requires a nightly toolchain (libFuzzer) and `cargo-fuzz`:

```sh
rustup toolchain install nightly-2026-07-01
cargo install --git https://github.com/rust-fuzz/cargo-fuzz.git --rev 984c861c8dfea28055254c5f1d2659ab2cd63f76 --locked cargo-fuzz

# From the repository root:
cargo +nightly-2026-07-01 fuzz run multibase
cargo +nightly-2026-07-01 fuzz run jcs_text -- -max_total_time=60  # time-boxed
cargo +nightly-2026-07-01 fuzz list                              # all targets
```

Reproduce a crash artifact:

```sh
cargo +nightly-2026-07-01 fuzz run <target> fuzz/artifacts/<target>/<crash-file>
```

## CI

Targets are built (not run to convergence) in CI as a smoke check so they cannot
bit-rot. A scheduled job runs each target time-boxed; any crash artifact is
uploaded and fails the job.
