// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

//! Multikey public-key encoding: a multicodec algorithm prefix plus multibase, with strict length and binding validation on decode.

mod binding;
mod encode;
mod error;
mod parse;

pub use binding::{binding_type_matches_codec, validate_key_binding, KeyBindingInput};
pub use encode::encode_multikey;
pub use error::{
    BindingAlgorithmKind, BindingTypeKind, CodecNameReason, MultikeyCodecKind, MultikeyError,
};
pub use parse::{parse_multikey, ParsedMultikey};
