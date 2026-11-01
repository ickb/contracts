use super::*;

// Run an otherwise valid match while adding an unrelated `limit_order` output lock with non-empty args; that extra output lock executes and poisons the whole tx.
#[test]
fn unrelated_non_empty_args_output_lock_poisons_match() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let poisoned_lock = data1_script(&mut context, "limit_order", Bytes::from(vec![1]));
    let mut spec = udt_to_ckb_step(&env, 20 * CKB, (100, 80), 4);
    spec.outputs.push(plain(&poisoned_lock, 200 * CKB).into());
    spec.fail(&mut context, ERROR_NOT_EMPTY_ARGS);
}

// Run the same valid match while also creating a plausible order-shaped output under non-empty-args `limit_order`; the extra output lock still executes and aborts the tx.
#[test]
fn order_shaped_non_empty_args_output_poisons_match() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let poisoned_lock = data1_script(&mut context, "limit_order", Bytes::from(vec![1]));
    let mut spec = udt_to_ckb_step(&env, 20 * CKB, (100, 80), 4);
    spec.inputs[1].cell.capacity = 400 * CKB;
    let poisoned_order = Cell { lock: poisoned_lock, ..env.order(0, order_data_mint(0, 1, (1, 1))) };
    spec.outputs.push(Cell { capacity: poisoned_order.occupied() + 300 * CKB, ..poisoned_order }.into());
    spec.fail(&mut context, ERROR_NOT_EMPTY_ARGS);
}

// Run a valid match but include an unrelated master sibling whose type is non-empty-args `limit_order`; that extra input-side script executes and poisons verification.
#[test]
fn non_empty_args_master_sibling_poisons_match() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let poisoned_master_type = data1_script(&mut context, "limit_order", Bytes::from(vec![1]));
    let mut spec = udt_to_ckb_step(&env, 20 * CKB, (100, 80), 4);
    spec.inputs.push(In::new(Cell { type_: Some(poisoned_master_type), ..plain(&owner_lock, 200 * CKB) }));
    spec.fail(&mut context, ERROR_NOT_EMPTY_ARGS);
}
