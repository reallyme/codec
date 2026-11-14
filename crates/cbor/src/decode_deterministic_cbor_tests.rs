// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{DecodeLimits, MAX_DETERMINISTIC_CBOR_NODES};
use crate::DeterministicCborError;

#[test]
fn decoder_node_guards_accept_exact_limit_and_reject_one_more() {
    // Keep the two accounting guards independently observable. Normal
    // decoding reserves descendants before visiting them, so an earlier
    // reservation failure can mask a later add-node regression.
    let mut reservation = DecodeLimits {
        nodes: 1,
        pending_nodes: MAX_DETERMINISTIC_CBOR_NODES - 2,
        ..DecodeLimits::default()
    };
    assert!(reservation.reserve_children(1).is_ok());
    assert_eq!(
        reservation.reserve_children(1),
        Err(DeterministicCborError::NodeLimitExceeded)
    );

    let mut visits = DecodeLimits {
        nodes: MAX_DETERMINISTIC_CBOR_NODES - 1,
        pending_nodes: 2,
        ..DecodeLimits::default()
    };
    assert!(visits.add_node().is_ok());
    assert_eq!(
        visits.add_node(),
        Err(DeterministicCborError::NodeLimitExceeded)
    );
}
