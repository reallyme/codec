# Data handling

Inputs, decoded values and serialized messages can contain personal data or key
material. Keep them out of logs and clear sensitive mutable buffers after use.
Callers own their input and returned storage, including copies they create.

Managed strings and garbage-collected values cannot be reliably erased.
Serialization can add copies. Prefer binary protobuf for sensitive data; JSON
parsing can retain unescaped text in scratch buffers that are freed without wiping.

Encoded-byte limits are not a total heap quota. Budget for decoded values,
transport copies and concurrent requests, especially on 32-bit Wasm hosts.

C callers must supply live allocations of the declared size and obey the
header's alignment, ownership and aliasing requirements. Pointer checks cannot
prove that an arbitrary address is allocated. Load native libraries only from
trusted storage.

Copyright © 2026 ReallyMe Technologies Limited. All rights reserved
