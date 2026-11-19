# Fuzzing

Install the pinned nightly and cargo-fuzz, then run a target from the repository
root:

```sh
rustup toolchain install nightly-2026-11-10
cargo install --git https://github.com/rust-fuzz/cargo-fuzz.git --rev 984c861c8dfea28055254c5f1d2659ab2cd63f76 --locked cargo-fuzz
cargo +nightly-2026-11-10 fuzz list
cargo +nightly-2026-11-10 fuzz run multibase -- -max_total_time=60
```

Targets include `base64`, `base64url`, `hex`, `multibase`, `multicodec`,
`multikey`, `pem_decode`, `dag_cbor`, `deterministic_cbor`, `general_cbor`,
`der`, `der_schema`, `tlv`, `jcs_text`, `guarded_json` and `operation_contract`.

Seed corpora are under `fuzz/seeds/`. The DER schema corpus contains generated
requests and needs `-max_len=2097152` to include its largest seed:

```sh
cargo +nightly-2026-11-10 fuzz run der_schema fuzz/seeds/der_schema -- -runs=0 -max_len=2097152
```

Replay a saved failure with:

```sh
cargo +nightly-2026-11-10 fuzz run <target> fuzz/artifacts/<target>/<crash-file>
```
