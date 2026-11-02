use super::*;

// An occupied-capacity-valid fulfilled CKB->UDT cell has no capacity left to decrease, so a continuation fails the outer match-shape check.
#[test]
fn fulfilled_ckb_to_udt_shape_fails_as_invalid_match() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let master = OutPoint::new(Byte32::zero(), 5);
    TxSpec {
        inputs: vec![In::new(env.order_unused(0, order_data_match(0, &master, (1, 1))))],
        outputs: vec![env.order_unused(0, order_data_match(1, &master, (1, 1))).into()],
        ..Default::default()
    }
    .fail(&mut context, ERROR_LIMIT_ORDER_INVALID_MATCH);
}

// Forge a fulfilled UDT->CKB-shaped cell and try to continue it; this shape never reaches the inner fulfilled guard and instead fails the outer match validation.
#[test]
fn fulfilled_udt_to_ckb_shape_cannot_reach_guard_and_fails_as_invalid_match() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    // The contract's fulfilled-order guard for UDT -> CKB sits behind `i.udt > o.udt`,
    // so a zero-UDT input can only be observed failing the outer match-shape check.
    udt_to_ckb_step(&env, 20 * CKB, (0, 0), 4).fail(&mut context, ERROR_LIMIT_ORDER_INVALID_MATCH);
}
