// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![no_main]

use codec_proto::generated::proto::reallyme::codec::v1::CodecTextResult;
use libfuzzer_sys::fuzz_target;

fuzz_target!(|input: &[u8]| {
    // Exercise the guarded tree and typed projection independently. Invalid
    // input must return a fixed reason while dropping every partial owner.
    let decoded = reallyme_codec::json::decode(input, None);
    drop(std::hint::black_box(decoded));
    let projected = reallyme_codec::json::from_slice::<CodecTextResult>(input, None);
    drop(std::hint::black_box(projected));
});
