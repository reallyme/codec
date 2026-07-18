<!--
SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved

SPDX-License-Identifier: Apache-2.0
-->

# reallyme-codec-pem

`reallyme-codec-pem` parses and emits PEM text armor: BEGIN/END labels,
base64 bodies, line ending normalization, strict label matching, and size
limits.

It deliberately does not interpret DER, ASN.1, or key structure. Use this crate
when you need the text envelope only. Algorithm-aware key import belongs in the
crypto layer that consumes the decoded bytes.
