# Data handling

Codec inputs, decoded values and serialized requests can contain personal data
or key material. Keep them out of logs, telemetry and exception metadata.

## Ownership and cleanup

Callers own their input buffers and returned bytes, strings and value trees.
Clear mutable sensitive buffers when they are no longer needed. Keep borrowed
source storage alive for every reader or view that refers to it.

Rust APIs returning `Zeroizing` owners clear the retained allocation on drop.
Copies, detached values and allocations retired by later `Vec` growth need
their own cleanup. Avoid growing or copying sensitive output unnecessarily.
Process termination and aborts can bypass cleanup.

Serialization creates additional copies. Native transport helpers clear the
transient buffers they own; generic operation callers must clear their request
and response storage. A decoded byte view may alias its response buffer: finish
using it before clearing that buffer.

Swift `Data`, JVM `ByteString`, JavaScript strings and garbage-collected objects
can retain copies that cannot be reliably erased. Clear mutable byte arrays,
keep immutable copies short-lived, and avoid logging generated messages.
Generated JVM transport messages are unsuitable keys for large hash collections
because their constant hashes can cause poor performance.

## JSON transport

Prefer binary protobuf for sensitive requests. ProtoJSON parsing may retain
unescaped text in parser scratch that is freed without wiping. Guarded JSON
document decoding does not remove this limitation from the outer transport.
Copies created by application deserializers remain the application's responsibility.

## Protobuf resource accounting

Wire-byte and node limits do not impose a total heap quota. A request near the
16 MiB wire cap can use several times that memory before returning a resource
limit error. Allow for input, decoded values, serialization copies and concurrent
requests when sizing a service or a 32-bit Wasm host. Apply concurrency and
process-memory limits appropriate to the deployment.

## Bounded Base64 and protobuf copies

Encoded-input and decoded-output budgets are independent. The direct Rust
`base64_to_bytes_with_limits` API returns `Zeroizing<Vec<u8>>`. Protobuf calls
add request and response copies, and request admission can copy encoded text
before the operation's limits are checked. Transport limits still apply.

## Native callers

C ABI callers must provide live allocations of the declared lengths, aligned
output pointers, and the ownership and aliasing guarantees documented in the
header. Pointer validation cannot establish that an arbitrary address is
allocated. Load native libraries only from trusted locations; use a pinned digest
when selecting an external library.
