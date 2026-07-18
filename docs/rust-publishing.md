<!--
SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved

SPDX-License-Identifier: Apache-2.0
-->

# Rust Publishing

Publishable crates in this repository are codec crates only. The root workspace
is virtual and is not published.

`reallyme-codec` is the recommended public Rust entry point. The publishable
leaf crates support dependency hygiene, implementation modularity, and
crates.io dependency resolution. They are released in lockstep
with `reallyme-codec`; they are not separately marketed products with
independent compatibility promises.

Use the release script to inspect or publish crates in dependency order:

```sh
node scripts/publish_crates_in_order.mjs inspect --allow-dirty
node scripts/publish_crates_in_order.mjs publish
```

Before publishing, run:

```sh
cargo fmt --check
cargo check --workspace --all-features
RUSTFLAGS=-Dwarnings cargo check --workspace --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
node scripts/check_release_readiness.mjs
```
