// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

pub(crate) use crate::sensitive_json_value::{SensitiveJsonValue, SensitiveNumber};
use crate::{read_json_string::read_json_string, JcsError};
use serde::de::{Deserializer, Visitor};
use std::{collections::BTreeMap, sync::OnceLock};

/// Parse one complete document without an unwiped string or number scratch.
pub(crate) fn parse_json_text(input: &str) -> Result<SensitiveJsonValue, JcsError> {
    static NUMBER_VISITOR_IS_SAFE: OnceLock<bool> = OnceLock::new();
    // Retain the published fail-closed feature-unification policy. Serde sees
    // only this public constant, never identifying text from the caller.
    if !NUMBER_VISITOR_IS_SAFE.get_or_init(native_number_representation) {
        return Err(JcsError::UnsupportedNumberRepresentation);
    }
    let mut parser = Parser {
        input,
        offset: 0,
        duplicate: false,
    };
    let value = parser.value(0)?;
    parser.whitespace()?;
    if parser.offset != input.len() {
        return Err(JcsError::InvalidJson);
    }
    if parser.duplicate {
        return Err(JcsError::DuplicateProperty);
    }
    Ok(value)
}

fn native_number_representation() -> bool {
    struct Probe;
    impl<'de> Visitor<'de> for Probe {
        type Value = bool;
        fn expecting(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            formatter.write_str("native binary64 number visitation")
        }
        fn visit_f64<E>(self, value: f64) -> Result<bool, E> {
            Ok(value == 1.5)
        }
    }
    let mut probe = serde_json::Deserializer::from_str("1.5");
    matches!(probe.deserialize_any(Probe), Ok(true))
}

struct Parser<'a> {
    input: &'a str,
    offset: usize,
    duplicate: bool,
}

impl Parser<'_> {
    fn peek(&self) -> Option<u8> {
        self.input.as_bytes().get(self.offset).copied()
    }

    fn advance(&mut self) -> Result<(), JcsError> {
        self.offset = self.offset.checked_add(1).ok_or(JcsError::InvalidJson)?;
        Ok(())
    }

    fn whitespace(&mut self) -> Result<(), JcsError> {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
            self.advance()?;
        }
        Ok(())
    }

    fn consume(&mut self, expected: u8) -> Result<(), JcsError> {
        if self.peek() != Some(expected) {
            return Err(JcsError::InvalidJson);
        }
        self.advance()
    }

    fn literal(&mut self, literal: &[u8]) -> Result<(), JcsError> {
        let end = self
            .offset
            .checked_add(literal.len())
            .ok_or(JcsError::InvalidJson)?;
        if self.input.as_bytes().get(self.offset..end) != Some(literal) {
            return Err(JcsError::InvalidJson);
        }
        self.offset = end;
        Ok(())
    }

    fn value(&mut self, depth: usize) -> Result<SensitiveJsonValue, JcsError> {
        self.whitespace()?;
        match self.peek() {
            Some(b'n') => {
                self.literal(b"null")?;
                Ok(SensitiveJsonValue::Null)
            }
            Some(b't') => {
                self.literal(b"true")?;
                Ok(SensitiveJsonValue::Bool(true))
            }
            Some(b'f') => {
                self.literal(b"false")?;
                Ok(SensitiveJsonValue::Bool(false))
            }
            Some(b'"') => {
                let mut text = read_json_string(self.input, &mut self.offset)?;
                Ok(SensitiveJsonValue::String(core::mem::take(&mut *text)))
            }
            Some(b'[') => self.array(depth),
            Some(b'{') => self.object(depth),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(JcsError::InvalidJson),
        }
    }

    fn descend(depth: usize) -> Result<usize, JcsError> {
        let next = depth.checked_add(1).ok_or(JcsError::DepthExceeded)?;
        if next > crate::MAX_NESTING_DEPTH {
            return Err(JcsError::DepthExceeded);
        }
        Ok(next)
    }

    fn array(&mut self, depth: usize) -> Result<SensitiveJsonValue, JcsError> {
        let depth = Self::descend(depth)?;
        self.consume(b'[')?;
        self.whitespace()?;
        let mut values = Vec::new();
        if self.peek() == Some(b']') {
            self.advance()?;
            return Ok(SensitiveJsonValue::Array(values));
        }
        loop {
            let value = self.value(depth)?;
            values
                .try_reserve(1)
                .map_err(|_| JcsError::SerializationError)?;
            // Only pointers move during growth; each boxed value wipes itself.
            values.push(Box::new(value));
            self.whitespace()?;
            match self.peek() {
                Some(b']') => {
                    self.advance()?;
                    return Ok(SensitiveJsonValue::Array(values));
                }
                Some(b',') => self.advance()?,
                _ => return Err(JcsError::InvalidJson),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<SensitiveJsonValue, JcsError> {
        let depth = Self::descend(depth)?;
        self.consume(b'{')?;
        self.whitespace()?;
        // Install the wiping owner before a member can fail. In particular,
        // BTreeMap<String, _> alone would release earlier keys without wiping.
        let mut owner = SensitiveJsonValue::Object(BTreeMap::new());
        if self.peek() == Some(b'}') {
            self.advance()?;
            return Ok(owner);
        }
        if let SensitiveJsonValue::Object(values) = &mut owner {
            loop {
                self.whitespace()?;
                let mut key = read_json_string(self.input, &mut self.offset)?;
                self.whitespace()?;
                self.consume(b':')?;
                let value = self.value(depth)?;
                if values.contains_key(key.as_str()) {
                    self.duplicate = true;
                } else {
                    values.insert(core::mem::take(&mut *key), Box::new(value));
                }
                self.whitespace()?;
                match self.peek() {
                    Some(b'}') => {
                        self.advance()?;
                        break;
                    }
                    Some(b',') => self.advance()?,
                    _ => return Err(JcsError::InvalidJson),
                }
            }
        }
        Ok(owner)
    }

    fn digits(&mut self) -> Result<bool, JcsError> {
        let start = self.offset;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.advance()?;
        }
        Ok(self.offset != start)
    }

    fn number(&mut self) -> Result<SensitiveJsonValue, JcsError> {
        let start = self.offset;
        let negative = self.peek() == Some(b'-');
        if negative {
            self.advance()?;
        }
        match self.peek() {
            Some(b'0') => self.advance()?,
            Some(b'1'..=b'9') => {
                self.digits()?;
            }
            _ => return Err(JcsError::InvalidJson),
        }
        let mut integer = true;
        if self.peek() == Some(b'.') {
            integer = false;
            self.advance()?;
            if !self.digits()? {
                return Err(JcsError::InvalidJson);
            }
        }
        if matches!(self.peek(), Some(b'e' | b'E')) {
            integer = false;
            self.advance()?;
            if matches!(self.peek(), Some(b'+' | b'-')) {
                self.advance()?;
            }
            if !self.digits()? {
                return Err(JcsError::InvalidJson);
            }
        }
        let token = self
            .input
            .get(start..self.offset)
            .ok_or(JcsError::InvalidJson)?;
        if integer {
            if negative {
                if let Ok(value) = token.parse::<i64>() {
                    return Ok(SensitiveJsonValue::Number(SensitiveNumber::Signed(value)));
                }
            } else if let Ok(value) = token.parse::<u64>() {
                return Ok(SensitiveJsonValue::Number(SensitiveNumber::Unsigned(value)));
            }
        }
        // core's binary64 parser borrows the token and needs no heap scratch.
        // Validate JSON grammar first: FromStr also accepts non-JSON numbers.
        let value = token.parse::<f64>().map_err(|_| JcsError::InvalidJson)?;
        if !value.is_finite() {
            return Err(JcsError::InvalidJson);
        }
        Ok(SensitiveJsonValue::Number(SensitiveNumber::Float(value)))
    }
}
