// SPDX-FileCopyrightText: Copyright © 2026 ReallyMe Technologies Limited. All rights reserved
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{try_vec_with_capacity, DeterministicCborError};

#[test]
fn impossible_capacity_returns_typed_allocation_failure() {
    assert!(matches!(
        try_vec_with_capacity::<u8>(usize::MAX),
        Err(DeterministicCborError::AllocationFailure)
    ));
}
