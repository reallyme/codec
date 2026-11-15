// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use core::cmp::Ordering;

#[derive(Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum MapKeyOrderError {
    #[error("duplicate map key")]
    Duplicate,
    #[error("descending map key")]
    Descending,
}

/// Tracks strictly increasing canonical wire keys for one map.
#[derive(Default)]
pub(crate) struct MapKeyOrder<'a> {
    previous: Option<&'a [u8]>,
}

impl<'a> MapKeyOrder<'a> {
    pub(crate) fn admit(&mut self, key: &'a [u8]) -> Result<(), MapKeyOrderError> {
        if let Some(previous) = self.previous {
            // Strict progression excludes duplicates anywhere in an admitted
            // map and retains only the previous borrowed key.
            let order = previous
                .len()
                .cmp(&key.len())
                .then_with(|| previous.cmp(key));
            match order {
                Ordering::Less => {}
                Ordering::Equal => return Err(MapKeyOrderError::Duplicate),
                Ordering::Greater => return Err(MapKeyOrderError::Descending),
            }
        }
        self.previous = Some(key);
        Ok(())
    }
}

#[cfg(kani)]
#[path = "map_canonicality_proof_tests.rs"]
mod proofs;
