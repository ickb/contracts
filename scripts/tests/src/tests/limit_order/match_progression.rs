use super::*;

/// A CKB->UDT match order at master index 5, with `udt` matched and this UDT->CKB ratio.
fn ckb_to_udt(udt: u128, udt_to_ckb: (u64, u64)) -> Bytes {
    order_data_custom(udt, 1, [0u8; 32], 5u32.to_le_bytes(), (1, 1), udt_to_ckb, 4)
}

/// A match order at the fake master `0x00..00:5` holding `unused_in` CKB unused and `udt_in`,
/// continued to `unused_out` and `udt_out`.
fn match_step(env: &Env, (unused_in, udt_in): (u64, u128), (unused_out, udt_out): (u64, u128)) -> TxSpec {
    let master = OutPoint::new(Byte32::zero(), 5);
    TxSpec {
        inputs: vec![In::new(env.order_unused(unused_in, order_data_match(udt_in, &master, (1, 1))))],
        outputs: vec![env.order_unused(unused_out, order_data_match(udt_out, &master, (1, 1))).into()],
        ..Default::default()
    }
}

// Continue a UDT->CKB order with extra funding so the match path preserves value and clears the minimum fill threshold.
#[test]
fn udt_to_ckb_match_passes() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    udt_to_ckb_step(&env, 20 * CKB, (100, 80), 4)
        .verify(&mut context)
        .expect("udt-to-ckb partial matches should verify when value and minimum match hold");
}

// Continue the same UDT->CKB shape right at the minimum fill boundary; the match path should still accept the exact-threshold step.
#[test]
fn udt_to_ckb_match_accepts_exact_minimum_partial_fill() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    udt_to_ckb_step(&env, 16 * CKB, (100, 84), 4)
        .verify(&mut context)
        .expect("udt-to-ckb partial matches should verify when the fill lands exactly on the minimum boundary");
}

// Continue one valid match while also creating an unrelated typed output; the match path should ignore the foreign lock/type pair and still pass.
#[test]
fn valid_match_ignores_foreign_typed_output() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let foreign_lock = named_always_success_lock(&mut context, b"foreign-lock");
    let foreign_type = named_always_success_lock(&mut context, b"foreign-type");
    let mut spec = udt_to_ckb_step(&env, 20 * CKB, (100, 80), 4);
    spec.inputs[1].cell.capacity = 300 * CKB;
    spec.outputs.push(Cell { type_: Some(foreign_type), ..plain(&foreign_lock, 200 * CKB) }.into());
    spec.verify(&mut context)
        .expect("limit_order should ignore unrelated foreign typed outputs during a valid match");
}

// Continue a CKB->UDT order with both ratios populated; the match path accepts when value moves in the allowed direction.
#[test]
fn ckb_to_udt_match_passes_with_both_ratios_present() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    // The matcher collects the 20 CKB into its own funding cell, which a bare 20 CKB cell could not hold.
    TxSpec {
        inputs: vec![
            In::new(env.order_unused(1_500 * CKB, ckb_to_udt(0, (1, 1)))),
            In::new(env.plain(1_000 * CKB)),
        ],
        outputs: vec![
            env.order_unused(1_480 * CKB, ckb_to_udt(u128::from(20 * CKB), (1, 1))).into(),
            env.plain(1_020 * CKB).into(),
        ],
        ..Default::default()
    }
    .verify(&mut context)
    .expect("ckb-to-udt partial matches should verify when both ratios are present and value is preserved");
}

// Continue a CKB->UDT order by exactly the advertised minimum fill; the match path accepts the boundary case.
#[test]
fn ckb_to_udt_match_accepts_exact_minimum_partial_fill() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    TxSpec {
        inputs: vec![In::new(env.order_unused(1_500 * CKB, ckb_to_udt(0, (0, 0))))],
        outputs: vec![env.order_unused(1_500 * CKB - 16, ckb_to_udt(16, (0, 0))).into()],
        ..Default::default()
    }
    .verify(&mut context)
    .expect("ckb-to-udt partial matches should verify when the fill lands exactly on the minimum boundary");
}

// Continue a UDT->CKB order with too little UDT progress for its declared minimum; the match path rejects the underfilled step.
#[test]
fn udt_to_ckb_match_rejects_small_udt_delta_under_minimum() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    udt_to_ckb_step(&env, 20 * CKB, (100, 99), 8).fail(&mut context, ERROR_LIMIT_ORDER_INSUFFICIENT_MATCH);
}

// Continue a UDT->CKB order whose `ckb_min_match_log` demands a larger fill than the delta shown; the match path rejects the insufficient continuation.
#[test]
fn udt_to_ckb_match_rejects_large_minimum_partial_fill() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    udt_to_ckb_step(&env, 20 * CKB, (100, 99), 40).fail(&mut context, ERROR_LIMIT_ORDER_INSUFFICIENT_MATCH);
}

// Continue a match-shaped order while reversing progress in the wrong direction; the match path rejects the inconsistent state transition.
#[test]
fn match_rejects_invalid_direction_change() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    match_step(&env, (1_500 * CKB, 100 * CKB as u128), (1_400 * CKB, 50 * CKB as u128))
        .fail(&mut context, ERROR_LIMIT_ORDER_INVALID_MATCH);
}

// Re-emit a match-shaped order without changing capacity or matched amount; the match path rejects the no-op continuation.
#[test]
fn match_rejects_unchanged_shape() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    match_step(&env, (1_500 * CKB, 100 * CKB as u128), (1_500 * CKB, 100 * CKB as u128))
        .fail(&mut context, ERROR_LIMIT_ORDER_INVALID_MATCH);
}

// Continue a match-shaped order while reducing total order value; the match path catches the decreasing-value transition.
#[test]
fn match_rejects_decreasing_value() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    match_step(&env, (1_500 * CKB, 0), (1_400 * CKB, 50 * CKB as u128))
        .fail(&mut context, ERROR_LIMIT_ORDER_DECREASING_VALUE);
}

// Continue a CKB->UDT order by less than its minimum partial fill; the match path rejects the too-small delta.
#[test]
fn match_rejects_too_small_partial_fill() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    TxSpec {
        inputs: vec![In::new(env.order_unused(1_500 * CKB, ckb_to_udt(0, (0, 0))))],
        outputs: vec![env.order_unused(1_500 * CKB - 8, ckb_to_udt(8, (0, 0))).into()],
        ..Default::default()
    }
    .fail(&mut context, ERROR_LIMIT_ORDER_INSUFFICIENT_MATCH);
}

// Fully fill a CKB->UDT order down to its exact occupied capacity; terminal fills bypass the partial-fill minimum.
#[test]
fn ckb_to_udt_terminal_full_fill_passes_at_occupied_capacity() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    TxSpec {
        inputs: vec![In::new(env.order_unused(8, ckb_to_udt(0, (0, 0))))],
        outputs: vec![env.order_unused(0, ckb_to_udt(8, (0, 0))).into()],
        ..Default::default()
    }
    .verify(&mut context)
    .expect("a terminal full fill at exact occupied capacity should verify");
}
