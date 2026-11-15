// SPDX-FileCopyrightText: 2026 ReallyMe Technologies Limited
//
// SPDX-License-Identifier: MIT OR Apache-2.0

use super::{Budget, ErrorReason, Limits, MAX_BYTES, MAX_DEPTH, MAX_NODES, MAX_WORK};
use crate::general::{CborError, MapKeyEquality, Profile};

#[kani::proof]
fn consuming_nodes_and_work_charges_only_its_own_counter() {
    let node_limit: usize = kani::any();
    let work_limit: usize = kani::any();
    let nodes: usize = kani::any();
    let work: usize = kani::any();
    let cost: usize = kani::any();
    kani::assume(node_limit <= MAX_NODES && nodes <= node_limit);
    kani::assume(work_limit <= MAX_WORK && work <= work_limit);
    let mut budget = Budget::new(
        Limits::default()
            .with_nodes(node_limit)
            .unwrap()
            .with_work(work_limit)
            .unwrap(),
    );
    budget.nodes = nodes;
    budget.work = work;
    let node_result = budget.node().map_err(CborError::reason);
    assert_eq!(
        node_result,
        charge_reason(wide(nodes) + 1, node_limit, ErrorReason::NodeLimit)
    );
    assert_eq!(wide(budget.nodes), wide(nodes) + 1);
    assert_eq!(budget.work, work);
    let total = wide(work) + wide(cost);
    let work_result = budget.charge(cost).map_err(CborError::reason);
    assert_eq!(
        work_result,
        charge_reason(total, work_limit, ErrorReason::WorkLimit)
    );
    assert_eq!(budget.work, usize::try_from(total).unwrap_or(work));
    assert_eq!(wide(budget.nodes), wide(nodes) + 1);
    kani::cover!(
        node_result == Err(ErrorReason::NodeLimit) && work_result.is_ok(),
        "only nodes exhausted"
    );
    kani::cover!(
        node_result.is_ok() && work_result == Err(ErrorReason::WorkLimit),
        "only work exhausted"
    );
    kani::cover!(
        work_result == Err(ErrorReason::ArithmeticOverflow),
        "work cannot wrap"
    );
}

#[kani::proof]
fn contents_charge_only_the_selected_payload_budget() {
    let limit: usize = kani::any();
    let initial: usize = kani::any();
    let amount: usize = kani::any();
    let other: usize = kani::any();
    let text: bool = kani::any();
    kani::assume(limit <= MAX_BYTES && initial <= limit);
    let (mut budget, failure) = if text {
        let mut budget = Budget::new(Limits::default().with_text_bytes(limit).unwrap());
        budget.text = initial;
        budget.bytes = other;
        (budget, ErrorReason::TextLimit)
    } else {
        let mut budget = Budget::new(Limits::default().with_byte_string_bytes(limit).unwrap());
        budget.bytes = initial;
        budget.text = other;
        (budget, ErrorReason::ByteStringLimit)
    };
    let total = wide(initial) + wide(amount);
    let expected = charge_reason(total, limit, failure);
    let actual = budget.contents(amount, text).map_err(CborError::reason);
    assert_eq!(actual, expected);
    let (charged, untouched) = if text {
        (budget.text, budget.bytes)
    } else {
        (budget.bytes, budget.text)
    };
    assert_eq!(untouched, other);
    assert_eq!(charged, usize::try_from(total).unwrap_or(initial));
    kani::cover!(
        text && total == wide(limit) && actual.is_ok(),
        "text reaches its limit"
    );
    kani::cover!(
        !text && actual == Err(ErrorReason::ByteStringLimit),
        "byte strings exceed their limit"
    );
    kani::cover!(
        actual == Err(ErrorReason::ArithmeticOverflow),
        "the sum does not wrap"
    );
}

#[kani::proof]
fn declared_nodes_check_both_remaining_budgets_in_order() {
    let node_limit: usize = kani::any();
    let work_limit: usize = kani::any();
    let existing_nodes: usize = kani::any();
    let existing_work: usize = kani::any();
    let children: usize = kani::any();
    kani::assume(node_limit <= MAX_NODES && existing_nodes <= node_limit);
    kani::assume(work_limit <= MAX_WORK && existing_work <= work_limit);
    let limits = Limits::default()
        .with_nodes(node_limit)
        .unwrap()
        .with_work(work_limit)
        .unwrap();
    let mut budget = Budget::new(limits);
    budget.nodes = existing_nodes;
    budget.work = existing_work;
    let nodes = charge_reason(
        wide(existing_nodes) + wide(children),
        node_limit,
        ErrorReason::NodeLimit,
    );
    let work = charge_reason(
        wide(existing_work) + wide(children),
        work_limit,
        ErrorReason::WorkLimit,
    );
    let actual = budget.declared_nodes(children).map_err(CborError::reason);
    assert_eq!(actual, nodes.and(work));
    assert_eq!(budget.nodes, existing_nodes);
    assert_eq!(budget.work, existing_work);
    kani::cover!(children > 0 && actual.is_ok(), "reservation fits");
    kani::cover!(
        nodes.is_err() && work.is_err(),
        "node errors precede work errors"
    );
    kani::cover!(
        nodes.is_ok() && actual == Err(ErrorReason::WorkLimit),
        "reservation exhausts only work"
    );
}

#[kani::proof]
fn entering_a_container_checks_depth_at_the_profile_boundary() {
    let depth: usize = kani::any();
    let limit: usize = kani::any();
    let encoded_scalars: bool = kani::any();
    kani::assume(limit <= MAX_DEPTH);
    let limits = Limits::default().with_depth(limit).unwrap();
    let budget = if encoded_scalars {
        let profile = Profile::preferred()
            .with_map_key_equality(MapKeyEquality::EncodedScalars)
            .unwrap();
        Budget::for_profile(limits, profile)
    } else {
        Budget::new(limits)
    };
    let next = wide(depth) + 1;
    let actual = budget.enter(depth).map_err(CborError::reason);
    let expected = if next > wide(usize::MAX) {
        Err(ErrorReason::ArithmeticOverflow)
    } else if !encoded_scalars && next > wide(limit) {
        Err(ErrorReason::DepthLimit)
    } else {
        Ok(usize::try_from(next).unwrap())
    };
    assert_eq!(actual, expected);
    if let Ok(child_depth) = actual {
        let visit = budget
            .check_item_depth(child_depth)
            .map_err(CborError::reason);
        assert_eq!(
            visit,
            if next <= wide(limit) {
                Ok(())
            } else {
                Err(ErrorReason::DepthLimit)
            }
        );
    }
    kani::cover!(
        !encoded_scalars && depth == limit,
        "basic model rejects entering past depth"
    );
    kani::cover!(
        encoded_scalars && depth == limit,
        "COSE rejects visiting a child past depth"
    );
    kani::cover!(depth == usize::MAX, "depth increment cannot wrap");
}

fn charge_reason(total: u128, limit: usize, failure: ErrorReason) -> Result<(), ErrorReason> {
    if total > wide(usize::MAX) {
        Err(ErrorReason::ArithmeticOverflow)
    } else if total > wide(limit) {
        Err(failure)
    } else {
        Ok(())
    }
}

fn wide(value: usize) -> u128 {
    u128::from(u64::try_from(value).unwrap())
}
