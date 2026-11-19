// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::fmt;

use buffa::MessageField;
use codec_proto::generated::proto::reallyme::codec::v1 as proto;
use proto::codec_json_value::Kind;
use serde::de::{DeserializeOwned, MapAccess, SeqAccess, Visitor};
use serde::Deserializer;
use serde_json::value::RawValue;
use zeroize::Zeroizing;

use super::{
    error::{invalid_json, overflow},
    limits::Budget,
    string, DecodeResult, ErrorReason as R, JsonError, Limits, Value,
};

/// Decode exactly one bounded JSON document into its generated wiping owner.
/// Duplicate decoded keys are rejected independently at every object level.
pub fn decode(input: &[u8], limits: Option<&Limits>) -> Result<DecodeResult, JsonError> {
    let mut budget = Budget::new(limits)?;
    if input.len() > budget.input {
        return Err(JsonError::new(
            R::CODEC_ERROR_REASON_CANONICAL_JSON_INPUT_LIMIT,
        ));
    }
    budget.scan(input.len())?;
    // RawValue's borrowed reader scans strings without decoding them. Its
    // internal scratch contains framing delimiters, never string payloads.
    let raw: &RawValue = serde_json::from_slice(input).map_err(|_| invalid_json())?;
    let value = parse(raw, &mut budget, 0)?;
    Ok(DecodeResult {
        value: MessageField::some(value),
    })
}

/// Project the same guarded decoding operation into an owned Serde target.
/// The target remains responsible for wiping its own sensitive field storage.
pub fn from_slice<T: DeserializeOwned>(
    input: &[u8],
    limits: Option<&Limits>,
) -> Result<T, JsonError> {
    let value = decode(input, limits)?;
    super::project::from_value(value.value.as_option().ok_or_else(invalid_json)?)
}

fn parse(raw: &RawValue, budget: &mut Budget, depth: usize) -> Result<Value, JsonError> {
    let text = raw.get();
    budget.scan(text.len())?;
    budget.node()?;
    let kind = match text.as_bytes().first() {
        Some(b'"') => Kind::StringValue(string::decode(text, budget)?),
        Some(b'n') if text == "null" => Kind::NullValue(Box::default()),
        Some(b't') if text == "true" => Kind::BoolValue(true),
        Some(b'f') if text == "false" => Kind::BoolValue(false),
        Some(b'[' | b'{') => {
            let next = depth.checked_add(1).ok_or_else(overflow)?;
            if next > budget.depth {
                return Err(JsonError::new(
                    R::CODEC_ERROR_REASON_CANONICAL_JSON_DEPTH_LIMIT,
                ));
            }
            let mut parser = serde_json::Deserializer::from_str(text);
            // Depth is enforced before each child projection. RawValue scans
            // nested syntax iteratively, so it does not recurse on the stack.
            let result = if text.starts_with('[') {
                parser.deserialize_seq(ArrayVisitor {
                    budget,
                    depth: next,
                })
            } else {
                parser.deserialize_map(ObjectVisitor {
                    budget,
                    depth: next,
                })
            };
            let result = result.map_err(|_| budget.failure.take().unwrap_or_else(invalid_json))?;
            parser.end().map_err(|_| invalid_json())?;
            result
        }
        Some(b'-' | b'0'..=b'9') => {
            // Parsing directly into a primitive keeps number admission
            // independent of Serde's arbitrary_precision feature unification.
            let value = Zeroizing::new(text.parse::<f64>().map_err(|_| invalid_json())?);
            if !value.is_finite() {
                return Err(JsonError::new(
                    R::CODEC_ERROR_REASON_CANONICAL_JSON_NUMBER_OUT_OF_RANGE,
                ));
            }
            Kind::NumberText(string::number_text(text, budget)?)
        }
        _ => return Err(invalid_json()),
    };
    Ok(Value { kind: Some(kind) })
}

struct ArrayVisitor<'a> {
    budget: &'a mut Budget,
    depth: usize,
}

impl<'de> Visitor<'de> for ArrayVisitor<'_> {
    type Value = Kind;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a bounded JSON array")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut input: A) -> Result<Kind, A::Error> {
        let mut array = proto::CodecJsonArray::default();
        while let Some(raw) = input.next_element::<&RawValue>()? {
            if array.values.len() >= self.budget.entries {
                return Err(self.budget.reject(JsonError::new(
                    R::CODEC_ERROR_REASON_CANONICAL_JSON_ENTRY_LIMIT,
                )));
            }
            let value =
                parse(raw, self.budget, self.depth).map_err(|error| self.budget.reject(error))?;
            array.values.push(value);
        }
        Ok(Kind::ArrayValue(Box::new(array)))
    }
}

struct ObjectVisitor<'a> {
    budget: &'a mut Budget,
    depth: usize,
}

impl<'de> Visitor<'de> for ObjectVisitor<'_> {
    type Value = Kind;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a bounded unambiguous JSON object")
    }

    fn visit_map<A: MapAccess<'de>>(self, mut input: A) -> Result<Kind, A::Error> {
        let mut object = proto::CodecJsonObject::default();
        // RawValue works for object keys too. Asking Serde for String here
        // would reintroduce escaped-key scratch outside the wiping owner.
        while let Some(raw) = input.next_key::<&RawValue>()? {
            if object.members.len() >= self.budget.entries {
                return Err(self.budget.reject(JsonError::new(
                    R::CODEC_ERROR_REASON_CANONICAL_JSON_ENTRY_LIMIT,
                )));
            }
            self.budget
                .scan(raw.get().len())
                .map_err(|error| self.budget.reject(error))?;
            let name = string::decode(raw.get(), self.budget)
                .map_err(|error| self.budget.reject(error))?;
            for member in object.members.iter() {
                // Bound duplicate detection too: adversarial common-prefix
                // names must not turn a small document into unmetered work.
                let compared = member
                    .name
                    .len()
                    .min(name.len())
                    .checked_add(1)
                    .ok_or_else(overflow)
                    .map_err(|error| self.budget.reject(error))?;
                self.budget
                    .scan(compared)
                    .map_err(|error| self.budget.reject(error))?;
                if member.name == name {
                    return Err(self.budget.reject(JsonError::new(
                        R::CODEC_ERROR_REASON_CANONICAL_JSON_DUPLICATE_PROPERTY,
                    )));
                }
            }
            let raw = input.next_value::<&RawValue>()?;
            let value =
                parse(raw, self.budget, self.depth).map_err(|error| self.budget.reject(error))?;
            object.members.push(proto::CodecJsonMember {
                name,
                value: MessageField::some(value),
            });
        }
        Ok(Kind::ObjectValue(Box::new(object)))
    }
}
