// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use std::collections::BTreeMap;
use zeroize::{Zeroize, ZeroizeOnDrop};

/// Parsed values own every identifying string and erase partial trees on error.
pub(crate) enum SensitiveJsonValue {
    Null,
    Bool(bool),
    Number(SensitiveNumber),
    String(String),
    #[expect(
        clippy::vec_box,
        reason = "growth must not copy sensitive numeric payloads"
    )]
    Array(Vec<Box<SensitiveJsonValue>>),
    // Tree rotations move nodes, so boxed values keep numbers at stable addresses.
    Object(BTreeMap<String, Box<SensitiveJsonValue>>),
}

pub(crate) enum SensitiveNumber {
    Signed(i64),
    Unsigned(u64),
    Float(f64),
}

impl Zeroize for SensitiveNumber {
    fn zeroize(&mut self) {
        match self {
            Self::Signed(value) => value.zeroize(),
            Self::Unsigned(value) => value.zeroize(),
            Self::Float(value) => value.zeroize(),
        }
    }
}

impl Drop for SensitiveNumber {
    fn drop(&mut self) {
        self.zeroize();
    }
}

impl ZeroizeOnDrop for SensitiveNumber {}

impl Drop for SensitiveJsonValue {
    fn drop(&mut self) {
        match self {
            Self::Null | Self::Number(_) => {}
            Self::Bool(value) => value.zeroize(),
            Self::String(text) => text.zeroize(),
            Self::Array(values) => values.clear(),
            Self::Object(values) => {
                for (mut key, value) in core::mem::take(values) {
                    key.zeroize();
                    drop(value);
                }
            }
        }
    }
}

impl ZeroizeOnDrop for SensitiveJsonValue {}
