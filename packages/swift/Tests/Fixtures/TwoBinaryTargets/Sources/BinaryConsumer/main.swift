// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

import CSecp256k1
import ReallyMeCodecFFI

// Reference a symbol from each archive so the consumer exercises both links.
secp256k1_selftest()
print("Codec C ABI version \(rm_codec_abi_version())")
