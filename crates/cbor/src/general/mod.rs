// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

//! Borrowed general CBOR framing, basic validity, and protected encoding.
//!
//! Framing preserves received bytes and has a distinct receipt type from profile
//! validation. Profiles check UTF-8, basic-model key uniqueness, selected float
//! policy, and optional preferred encoding or key ordering. Tag interpretation
//! is explicit; embedded tag-24 bytes are never reconstructed for authentication.
//!
//! Input storage remains with its owner. Use a zeroizing owner for sensitive
//! input and do not log borrowed bytes. Payload-bearing types have no Debug,
//! cloning, or serialization implementation. Encoder outputs zeroize on drop.
//!
//! ```
//! use codec_cbor::general::{validate_exact, Limits, Profile};
//! use zeroize::Zeroizing;
//!
//! // The owner outlives every borrowed receipt and erases the received bytes.
//! let received = Zeroizing::new(vec![0xd8, 0x18, 0x43, 0xa1, 0x01, 0x02]);
//! let item = validate_exact(&received, Profile::preferred(), Limits::default())?;
//! let tagged = item.frame().tagged_bytes()?;
//! assert_eq!(tagged.received_bytes(), received.as_slice());
//! assert_eq!(tagged.payload_bytes(), &[0xa1, 0x01, 0x02]);
//! let embedded = tagged.validate_embedded(Profile::preferred(), Limits::default())?;
//! # Ok::<(), codec_cbor::general::CborError>(())
//! ```
//!
//! Receipts cannot escape the lifetime of the received owner:
//!
//! ```compile_fail
//! use codec_cbor::general::{frame_exact, LengthPolicy, Limits};
//! let receipt;
//! {
//!     let received = vec![0x41, 0x01];
//!     receipt = frame_exact(&received, Limits::default(), LengthPolicy::DefiniteOnly).unwrap();
//! }
//! let _ = receipt.received_bytes();
//! ```
//!
//! A framing receipt cannot be substituted for a validated item:
//!
//! ```compile_fail
//! use codec_cbor::general::{Frame, Item};
//! fn validated(frame: Frame<'_>) -> Item<'_> { frame }
//! ```
//!
//! Payload-bearing receipts and protected output deliberately omit diagnostics
//! and duplication traits:
//!
//! ```compile_fail
//! use codec_cbor::general::{frame_exact, LengthPolicy, Limits};
//! let receipt = frame_exact(&[0x00], Limits::default(), LengthPolicy::DefiniteOnly).unwrap();
//! let _ = format!("{receipt:?}");
//! ```
//!
//! ```compile_fail
//! use codec_cbor::general::{encode, Limits, Profile, ValueRef};
//! let output = encode(&ValueRef::Unsigned(0), Profile::preferred(), Limits::default()).unwrap();
//! let _ = output.clone();
//! ```

mod compare;
mod cursor;
mod encode;
mod error;
mod float;
mod limits;
mod profile;
mod read_header;
mod receipt;
mod scan;
mod unique;

pub use cursor::Cursor;
pub use encode::{encode, EncodedCbor, MapEntry, ValueRef};
pub use error::{CborError, ErrorReason};
pub use float::{Float, FloatWidth};
pub use limits::{Limits, MAX_BYTES, MAX_DEPTH, MAX_ENTRIES, MAX_NODES, MAX_WORK};
pub use profile::{FloatPolicy, LengthPolicy, MapOrdering, Profile, Serialization};
pub use read_header::{read_header_prefix, ArgumentWidth, Header, MajorType};
pub use receipt::{
    frame_exact, frame_prefix, validate_exact, validate_prefix, Frame, Integer, Item, ItemChildren,
    StringChunks, TaggedBytes,
};
