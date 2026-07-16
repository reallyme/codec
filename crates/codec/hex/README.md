<!--
SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved

SPDX-License-Identifier: Apache-2.0
-->

# reallyme-codec-hex

Small lowercase hexadecimal helpers used by the ReallyMe codec packages.

The decoder is intentionally canonical: uppercase input is rejected, and odd
lengths or non-hex characters return typed errors.
