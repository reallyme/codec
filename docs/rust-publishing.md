# Rust Publishing

Publishable crates in this repository are codec crates only. The root workspace
is virtual and is not published.

`reallyme-codec` is the recommended public Rust entry point. The publishable
leaf crates support dependency hygiene, implementation modularity, and
crates.io dependency resolution. They are released in lockstep
with `reallyme-codec`; they are not separately marketed products with
independent compatibility promises.

Use **Crates Package Preflight** with the release version and exact current
`main` commit, then **Crates.io Release**. The release workflow resolves the
current `main` commit and requires successful checks and the matching preflight
before inspecting packages and publishing them in dependency order. If `main`
changes, the new commit needs its own successful preflight.

For local inspection without publishing:

```sh
node scripts/publish_crates_in_order.mjs inspect --allow-dirty
```

Before publishing, run the [release validation checks](../SECURITY_MEMORY_MODEL.md#validation-gate),
including dependency audits and SDK tests. Local inspection lists package
contents and runs Cargo's publish dry-run; dependent crates cannot complete
registry resolution until earlier crates of the same version are published.

Published crate metadata declares `MIT OR Apache-2.0`, and bundled `LICENSE`
files contain both license texts.
