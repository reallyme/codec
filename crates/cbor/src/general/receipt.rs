// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use zeroize::{Zeroize, ZeroizeOnDrop};

use super::compare::require_type;
use super::cursor::Cursor;
use super::error::{error, CborError, ErrorReason};
use super::float::Float;
use super::limits::{Budget, Limits};
use super::profile::{LengthPolicy, Profile};
use super::read_header::{read_header_prefix, ArgumentWidth, Header, MajorType};
use super::scan::{add, scan, tail};

/// Complete integer domain: unsigned values or the negative argument `n`
/// representing `-1-n`, including -18446744073709551616.
pub struct Integer {
    negative: bool,
    argument: u64,
}

impl Integer {
    /// Whether this is a major-type-one integer.
    #[must_use]
    pub const fn is_negative(&self) -> bool {
        self.negative
    }
    /// Extract the complete unsigned argument. The caller owns this scalar copy.
    #[must_use]
    pub const fn argument(&self) -> u64 {
        self.argument
    }
    /// Convert exactly to a signed type that covers the entire CBOR domain.
    #[must_use]
    pub fn to_i128(&self) -> i128 {
        if self.negative {
            -1 - i128::from(self.argument)
        } else {
            i128::from(self.argument)
        }
    }
}

impl Drop for Integer {
    fn drop(&mut self) {
        self.argument.zeroize();
        self.negative.zeroize();
    }
}
impl ZeroizeOnDrop for Integer {}

/// Receipt for one well-formed item, without a basic-validity claim.
///
/// Framing allows invalid UTF-8 and equivalent map keys. Use `validate_exact`
/// or `Frame::validate` when the basic generic data model is required. Tag
/// semantics are always separate. All payload spans borrow the original owner.
pub struct Frame<'a> {
    pub(super) encoded: &'a [u8],
    pub(super) body: &'a [u8],
    pub(super) header: Header<'a>,
    pub(super) limits: Limits,
    pub(super) lengths: LengthPolicy,
}

impl<'a> Frame<'a> {
    /// Exact complete received item, with its original headers and ordering.
    #[must_use]
    pub const fn received_bytes(&self) -> &'a [u8] {
        self.encoded
    }
    /// Inspect the original header without reconstructing it.
    #[must_use]
    pub const fn header(&self) -> &Header<'a> {
        &self.header
    }
    /// Validate this receipt using a fresh, caller-selected resource budget.
    pub fn validate(&self, profile: Profile, limits: Limits) -> Result<Item<'a>, CborError> {
        validate_exact(self.encoded, profile, limits)
    }
    /// Extract an integer without truncating either argument domain.
    pub fn integer(&self) -> Result<Integer, CborError> {
        let negative = match self.header.major_type() {
            MajorType::Unsigned => false,
            MajorType::Negative => true,
            _ => return Err(error(ErrorReason::WrongType)),
        };
        Ok(Integer {
            negative,
            argument: self
                .header
                .argument()
                .ok_or(error(ErrorReason::WrongType))?,
        })
    }
    /// Extract protected floating-point bits.
    pub fn float(&self) -> Result<Float, CborError> {
        self.header.float()?.ok_or(error(ErrorReason::WrongType))
    }
    /// Extract an untagged simple value (20/21 are booleans, 22 is null).
    pub fn simple_value(&self) -> Result<u8, CborError> {
        self.header
            .simple_value()?
            .ok_or(error(ErrorReason::WrongType))
    }
    /// Extract an untagged boolean; other simple values have a different type.
    pub fn as_bool(&self) -> Result<bool, CborError> {
        const FALSE: u8 = 20;
        const TRUE: u8 = 21;
        match self.simple_value()? {
            FALSE => Ok(false),
            TRUE => Ok(true),
            _ => Err(error(ErrorReason::WrongType)),
        }
    }
    /// Borrow definite byte-string contents; chunked strings have no single span.
    pub fn as_bytes(&self) -> Result<&'a [u8], CborError> {
        require_type(self.header.major_type(), MajorType::Bytes)?;
        self.contiguous()
    }
    /// Borrow definite text, checking UTF-8 even on a framing-only receipt.
    pub fn as_text(&self) -> Result<&'a str, CborError> {
        require_type(self.header.major_type(), MajorType::Text)?;
        core::str::from_utf8(self.contiguous()?).map_err(|_| error(ErrorReason::InvalidUtf8))
    }
    fn contiguous(&self) -> Result<&'a [u8], CborError> {
        if self.header.argument_width() == ArgumentWidth::Indefinite {
            return Err(error(ErrorReason::NonContiguousString));
        }
        Ok(self.body)
    }
    /// Iterate borrowed contents of string chunks. Definite strings yield once.
    /// Framing-only text chunks still require UTF-8 validation by the consumer.
    pub fn string_chunks(&self) -> Result<StringChunks<'a>, CborError> {
        if !matches!(self.header.major_type(), MajorType::Bytes | MajorType::Text) {
            return Err(error(ErrorReason::WrongType));
        }
        Ok(StringChunks {
            remaining: self.body,
            definite: self.header.argument_width() != ArgumentWidth::Indefinite,
            done: false,
        })
    }
    /// Traverse direct array children, alternating map keys/values, or a tag's
    /// single child. This framing cursor has its own cumulative limits; it does
    /// not inherit or strengthen a parent's validity claim.
    pub fn children(&self) -> Result<Cursor<'a>, CborError> {
        if !matches!(
            self.header.major_type(),
            MajorType::Array | MajorType::Map | MajorType::Tag
        ) {
            return Err(error(ErrorReason::WrongType));
        }
        Cursor::new(self.body, self.limits, self.lengths)
    }
    /// Borrow an arbitrary tag's exact outer wrapper and contiguous byte payload.
    /// No claim is made about embedded CBOR or registered tag semantics.
    pub fn tagged_bytes(&self) -> Result<TaggedBytes<'a>, CborError> {
        require_type(self.header.major_type(), MajorType::Tag)?;
        let child = frame_exact(self.body, self.limits, self.lengths)?;
        Ok(TaggedBytes {
            tag: self
                .header
                .argument()
                .ok_or(error(ErrorReason::WrongType))?,
            outer: self.encoded,
            payload: child.as_bytes()?,
        })
    }
}

/// Basic-valid item under the recorded profile. Registered tag semantics are
/// opaque; this receipt never implies application-level or tag-24 validation.
pub struct Item<'a> {
    frame: Frame<'a>,
    profile: Profile,
}

impl<'a> Item<'a> {
    /// Borrow its exact framing receipt and typed accessors.
    #[must_use]
    pub const fn frame(&self) -> &Frame<'a> {
        &self.frame
    }
    /// Inspect the guarantees selected at validation.
    #[must_use]
    pub const fn profile(&self) -> Profile {
        self.profile
    }
    /// Iterate direct children as validated items under the same profile.
    ///
    /// A parent validated under a profile has every descendant validated
    /// under it, so children keep the validated receipt type. Map children
    /// alternate key and value; a tag yields its single enclosed item.
    pub fn children(&self) -> Result<ItemChildren<'a>, CborError> {
        Ok(ItemChildren {
            frames: self.frame.children()?,
            profile: self.profile,
        })
    }
}

/// Validated direct children of an [`Item`], in received order.
pub struct ItemChildren<'a> {
    frames: Cursor<'a>,
    profile: Profile,
}

impl<'a> Iterator for ItemChildren<'a> {
    type Item = Result<Item<'a>, CborError>;
    fn next(&mut self) -> Option<Self::Item> {
        let profile = self.profile;
        self.frames
            .next()
            .map(|frame| frame.map(|frame| Item { frame, profile }))
    }
}

impl core::iter::FusedIterator for ItemChildren<'_> {}

/// Exact tagged wrapper and opaque byte contents, both tied to the input owner.
pub struct TaggedBytes<'a> {
    tag: u64,
    outer: &'a [u8],
    payload: &'a [u8],
}

impl<'a> TaggedBytes<'a> {
    /// Registered or unregistered tag argument.
    #[must_use]
    pub const fn tag(&self) -> u64 {
        self.tag
    }
    /// Complete original tagged item including the byte-string header.
    #[must_use]
    pub const fn received_bytes(&self) -> &'a [u8] {
        self.outer
    }
    /// Exact opaque byte-string contents, without either wrapper header.
    #[must_use]
    pub const fn payload_bytes(&self) -> &'a [u8] {
        self.payload
    }
    /// For standard tag 24 only, validate exactly one embedded CBOR item.
    /// Empty payloads and payloads containing trailing items are rejected.
    pub fn validate_embedded(
        &self,
        profile: Profile,
        limits: Limits,
    ) -> Result<Item<'a>, CborError> {
        const ENCODED_CBOR_TAG: u64 = 24;
        if self.tag != ENCODED_CBOR_TAG {
            return Err(error(ErrorReason::WrongTag));
        }
        if self.payload.is_empty() {
            return Err(error(ErrorReason::EmptyEmbeddedItem));
        }
        validate_exact(self.payload, profile, limits)
    }
}

impl Drop for TaggedBytes<'_> {
    fn drop(&mut self) {
        self.tag.zeroize();
    }
}
impl ZeroizeOnDrop for TaggedBytes<'_> {}

/// Borrowed chunks with no concatenation or structural allocation.
pub struct StringChunks<'a> {
    remaining: &'a [u8],
    definite: bool,
    done: bool,
}

impl<'a> Iterator for StringChunks<'a> {
    type Item = Result<&'a [u8], CborError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }
        if self.definite {
            self.done = true;
            return Some(Ok(self.remaining));
        }
        if self.remaining.is_empty() {
            self.done = true;
            return None;
        }
        let result = (|| {
            let header = read_header_prefix(self.remaining)?;
            let length =
                usize::try_from(header.argument().ok_or(error(ErrorReason::InvalidChunk))?)
                    .map_err(|_| error(ErrorReason::ArithmeticOverflow))?;
            let start = header.received_bytes().len();
            let end = add(start, length)?;
            let bytes = self
                .remaining
                .get(start..end)
                .ok_or(error(ErrorReason::Truncated))?;
            self.remaining = tail(self.remaining, end)?;
            Ok(bytes)
        })();
        if result.is_err() {
            self.done = true;
        }
        Some(result)
    }
}

impl core::iter::FusedIterator for StringChunks<'_> {}

pub(super) fn check_input(input: &[u8], limits: Limits) -> Result<(), CborError> {
    if input.len() > limits.input {
        Err(error(ErrorReason::InputLimit))
    } else {
        Ok(())
    }
}

/// Frame one item and return the exact unconsumed suffix separately.
pub fn frame_prefix(
    input: &[u8],
    limits: Limits,
    lengths: LengthPolicy,
) -> Result<(Frame<'_>, &[u8]), CborError> {
    check_input(input, limits)?;
    let frame = scan(input, &mut Budget::new(limits), 0, None, true, lengths)?;
    let rest = tail(input, frame.encoded.len())?;
    Ok((frame, rest))
}

/// Frame exactly one item, rejecting every trailing byte.
pub fn frame_exact(
    input: &[u8],
    limits: Limits,
    lengths: LengthPolicy,
) -> Result<Frame<'_>, CborError> {
    let (frame, rest) = frame_prefix(input, limits, lengths)?;
    if !rest.is_empty() {
        return Err(error(ErrorReason::TrailingData));
    }
    Ok(frame)
}

/// Validate one item under the selected basic-model and serialization profile.
pub fn validate_prefix(
    input: &[u8],
    profile: Profile,
    limits: Limits,
) -> Result<(Item<'_>, &[u8]), CborError> {
    check_input(input, limits)?;
    let frame = scan(
        input,
        &mut Budget::new(limits),
        0,
        Some(profile),
        true,
        profile.lengths,
    )?;
    let rest = tail(input, frame.encoded.len())?;
    Ok((Item { frame, profile }, rest))
}

/// Validate exactly one item under the selected profile, rejecting trailing bytes.
pub fn validate_exact(
    input: &[u8],
    profile: Profile,
    limits: Limits,
) -> Result<Item<'_>, CborError> {
    let (item, rest) = validate_prefix(input, profile, limits)?;
    if !rest.is_empty() {
        return Err(error(ErrorReason::TrailingData));
    }
    Ok(item)
}
