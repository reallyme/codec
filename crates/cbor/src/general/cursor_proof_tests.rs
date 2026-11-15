// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{error, Cursor, ErrorReason, Frame, LengthPolicy, Limits};
use crate::general::read_header_prefix;

#[kani::proof]
#[kani::unwind(18)]
fn cursor_advances_exactly_or_keeps_its_span_and_fuses() {
    let argument: u8 = kani::any();
    let short: bool = kani::any();
    kani::assume(argument < 24);
    // The first item is complete and the second is a truncated argument.
    // Supply the corresponding scanner outcome to the production cursor step.
    let input = [argument, 0x18];
    let limits = Limits::default();
    let lengths = LengthPolicy::DefiniteOnly;
    let mut cursor = Cursor::new(&input, limits, lengths).unwrap();
    let scanned = if short {
        cursor.offset = 1;
        cursor.consumed = &input[..1];
        cursor.remaining = &input[1..];
        Err(error(ErrorReason::Truncated))
    } else {
        let header = read_header_prefix(&input[..1]).unwrap();
        Ok(Frame {
            encoded: &input[..1],
            body: &input[1..1],
            header,
            limits,
            lengths,
        })
    };
    let step = cursor.finish_step(scanned);
    if short {
        assert!(matches!(step, Err(error) if error.reason() == ErrorReason::Truncated));
        assert!(cursor.failed);
    } else {
        let Ok(frame) = step else {
            kani::assert(false, "the first complete integer advances the cursor");
            return;
        };
        assert!(!cursor.failed);
        assert!(core::ptr::eq(frame.received_bytes(), &input[..1]));
    }
    assert_eq!(cursor.offset, 1);
    assert!(core::ptr::eq(cursor.consumed_bytes(), &input[..1]));
    assert!(core::ptr::eq(cursor.remaining_bytes(), &input[1..]));
    kani::cover!(
        short,
        "truncated second item leaves the committed span intact"
    );
    kani::cover!(!short, "one successful step before the suffix");
}
