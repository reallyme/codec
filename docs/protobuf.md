# Protobuf

Use the protobuf schemas and generated types shipped with this package version.
They define its wire messages, enum values and error codes. Do not mix message
definitions from different package versions.

Prefer binary protobuf for sensitive requests. JSON parsing and managed-runtime
values can retain copies that cannot be reliably erased. Keep payloads out of
logs and clear caller-owned request and response buffers after use.
