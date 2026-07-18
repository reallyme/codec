<!--
SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved

SPDX-License-Identifier: Apache-2.0
-->

# Security Policy

`reallyme-codec` is security-sensitive infrastructure. We treat parser bugs,
non-canonical encoding acceptance, boundary panics, memory-safety issues, and
cross-language divergence as security-relevant.

## Reporting

Please report vulnerabilities privately before public disclosure. Use
[GitHub private vulnerability reporting](https://github.com/reallyme/codec/security/advisories/new)
for this repository when available; otherwise use the security contact listed
for ReallyMe LLC.

Include the affected package, version, input shape, and the smallest
reproduction you can share without exposing sensitive data. Do not attach
private keys, production identity documents, or other secrets.

## Scope

In scope:

- parser panics, unbounded resource use, or memory-safety issues;
- acceptance of non-canonical encodings where canonical form is required;
- Swift, Kotlin, or TypeScript behavior diverging from Rust;
- FFI/JNI/WASM boundary validation failures;
- error paths that expose raw attacker-controlled input, PII, or backend
  exception text.

Out of scope:

- cryptographic primitive implementation bugs, provider policy, and key
  management issues that belong to the separate crypto repository;
- unsupported platform combinations that fail closed with typed errors.

## Supported Versions

The current `0.2.x` line receives security fixes while the public API remains in
early release. Pin exact versions in production deployments.
