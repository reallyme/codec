# Dependency Updates

Review codec dependency updates one at a time when they affect parsing,
serialization, protobuf generation, WASM bindings, JNI, or native ABI behavior.

Every update must pass the relevant Rust, TypeScript, Swift, and Kotlin codec
tests. Updates that change accepted input, canonical output, or error mapping
require focused negative tests.

The JVM and Android builds use locked dependency graphs and strict SHA-256
verification metadata. Do not accept a checksum merely because Gradle generated
it: corroborate new artifact hashes through a publisher-controlled source or an
independent trusted mirror and record that provenance during review.

Run `python3 scripts/verify_gradle_checksum_provenance.py` after changing either
Gradle verification file. The verifier compares every pinned SHA-256 with the
publisher's SHA-256 sidecar or a fresh download of the artifact from Maven
Central, Google Maven, or the Gradle Plugin Portal. It fails if any entry is
unavailable or differs. Its local regression tests run with
`python3 -m unittest scripts.verify_gradle_checksum_provenance_test`.

On 2026-10-01, all 1,009 unique entries across the JVM and Android verification
files matched publisher bytes: 463 through SHA-256 sidecars and 546 through
fresh artifact downloads. No mismatch or unavailable entry remained.

PGP verification must not be enabled by blindly committing Gradle's
auto-trusted or auto-ignored bootstrap keys. Enabling it requires independently
verifying every trusted full fingerprint, scoping each key to its publisher,
committing the reviewed public-key ring, and retaining SHA-256 verification as
the integrity control and fallback for unsigned artifacts.
