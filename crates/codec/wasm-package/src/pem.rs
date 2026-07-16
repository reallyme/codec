// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe LLC. All rights reserved
//
// SPDX-License-Identifier: Apache-2.0

use codec_pem::{
    decode_pem, encode_pem, PemDecodePolicy, PemEncodeOptions, PemLabel, PemLineEnding,
};
use js_sys::{Object, Uint8Array};
use serde::Deserialize;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use zeroize::Zeroizing;

use crate::map_error::invalid_input;
use crate::write_js_object::{set_bytes, set_string};

const DEFAULT_MAX_PEM_INPUT_LEN: usize = 1024 * 1024;
const DEFAULT_MAX_DER_LEN: usize = 1024 * 1024;
const DEFAULT_PEM_LINE_WIDTH: usize = 64;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DecodeOptions {
    #[serde(default)]
    pub(crate) allowed_labels: Option<Vec<String>>,
    #[serde(default = "default_max_pem_input_len")]
    pub(crate) max_input_len: usize,
    #[serde(default = "default_max_der_len")]
    pub(crate) max_der_len: usize,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct EncodeOptions {
    #[serde(default = "default_max_der_len")]
    max_der_len: usize,
    #[serde(default = "default_pem_line_width")]
    line_width: usize,
    #[serde(default)]
    line_ending: Option<String>,
}

fn default_max_pem_input_len() -> usize {
    DEFAULT_MAX_PEM_INPUT_LEN
}

fn default_max_der_len() -> usize {
    DEFAULT_MAX_DER_LEN
}

fn default_pem_line_width() -> usize {
    DEFAULT_PEM_LINE_WIDTH
}

pub(crate) fn parse_label(label: &str) -> Result<PemLabel, JsValue> {
    match label {
        "PRIVATE KEY" => Ok(PemLabel::PrivateKey),
        "EC PRIVATE KEY" => Ok(PemLabel::EcPrivateKey),
        "PUBLIC KEY" => Ok(PemLabel::PublicKey),
        _ => Err(invalid_input()),
    }
}

pub(crate) fn label_text(label: PemLabel) -> &'static str {
    match label {
        PemLabel::PrivateKey => "PRIVATE KEY",
        PemLabel::EcPrivateKey => "EC PRIVATE KEY",
        PemLabel::PublicKey => "PUBLIC KEY",
    }
}

fn parse_line_ending(line_ending: Option<&str>) -> Result<PemLineEnding, JsValue> {
    match line_ending {
        None | Some("lf") | Some("LF") => Ok(PemLineEnding::Lf),
        Some("crlf") | Some("CRLF") => Ok(PemLineEnding::Crlf),
        Some(_) => Err(invalid_input()),
    }
}

pub(crate) fn parse_decode_options(options_json: &str) -> Result<DecodeOptions, JsValue> {
    if options_json.is_empty() {
        return Ok(DecodeOptions {
            allowed_labels: None,
            max_input_len: DEFAULT_MAX_PEM_INPUT_LEN,
            max_der_len: DEFAULT_MAX_DER_LEN,
        });
    }
    serde_json::from_str(options_json).map_err(|_| invalid_input())
}

fn parse_encode_options(options_json: &str) -> Result<EncodeOptions, JsValue> {
    if options_json.is_empty() {
        return Ok(EncodeOptions {
            max_der_len: DEFAULT_MAX_DER_LEN,
            line_width: DEFAULT_PEM_LINE_WIDTH,
            line_ending: None,
        });
    }
    serde_json::from_str(options_json).map_err(|_| invalid_input())
}

#[wasm_bindgen(js_name = pemDecode)]
/// Decode PEM text armor with caller-supplied size and label policy.
pub fn pem_decode(input: &str, options_json: &str) -> Result<JsValue, JsValue> {
    let options = parse_decode_options(options_json)?;
    let labels = match options.allowed_labels {
        Some(labels) => {
            let mut parsed = Vec::with_capacity(labels.len());
            for label in labels {
                parsed.push(parse_label(&label)?);
            }
            parsed
        }
        None => vec![
            PemLabel::PrivateKey,
            PemLabel::EcPrivateKey,
            PemLabel::PublicKey,
        ],
    };
    let policy = PemDecodePolicy {
        allowed_labels: &labels,
        max_input_len: options.max_input_len,
        max_der_len: options.max_der_len,
    };
    let decoded = decode_pem(input, policy).map_err(|_| invalid_input())?;
    let object = Object::new();
    set_string(&object, "label", label_text(decoded.label))?;
    set_bytes(&object, "der", decoded.der.as_slice())?;
    Ok(object.into())
}

#[wasm_bindgen(js_name = pemEncode)]
/// Encode DER bytes as PEM text armor.
pub fn pem_encode(label: &str, der: &Uint8Array, options_json: &str) -> Result<String, JsValue> {
    let options = parse_encode_options(options_json)?;
    let line_ending = parse_line_ending(options.line_ending.as_deref())?;
    let options = PemEncodeOptions {
        max_der_len: options.max_der_len,
        line_width: options.line_width,
        line_ending,
    };
    let der = Zeroizing::new(der.to_vec());
    let encoded =
        encode_pem(parse_label(label)?, der.as_slice(), options).map_err(|_| invalid_input())?;
    Ok(encoded.as_str().to_owned())
}
