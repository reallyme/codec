// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Fuzz the executable protobuf and generated ProtoJSON dispatch boundaries.

#![no_main]

use libfuzzer_sys::fuzz_target;
use reallyme_codec::proto_process::{process_proto, process_proto_json};

fuzz_target!(|data: &[u8]| {
    let binary_envelope = process_proto(data);
    let _binary_len = binary_envelope.len();

    // ProtoJSON accepts bytes directly and must reject invalid UTF-8 and JSON
    // through the same structured result-envelope channel without panicking.
    let json_envelope = process_proto_json(data);
    let _json_len = json_envelope.len();
});
