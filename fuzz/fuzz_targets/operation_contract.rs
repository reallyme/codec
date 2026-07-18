// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Fuzz the executable protobuf and generated ProtoJSON dispatch boundaries.

#![no_main]

use libfuzzer_sys::fuzz_target;
use reallyme_codec::operation_contract::{process_operation_response, process_operation_response_json};

fuzz_target!(|data: &[u8]| {
    let binary_response = process_operation_response(data);
    let _binary_len = binary_response.len();

    // ProtoJSON accepts bytes directly and must reject invalid UTF-8 and JSON
    // through the same generated response channel without panicking.
    let json_response = process_operation_response_json(data);
    let _json_len = json_response.len();
});
