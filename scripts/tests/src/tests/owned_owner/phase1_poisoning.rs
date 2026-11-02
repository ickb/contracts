use super::*;

// Scenario: a valid withdrawal batch also creates an unrelated non-empty-args lock output.
// Expectation: Owned Owner scans the batch, treats the non-empty args as poison, and rejects it.
#[test]
fn unrelated_non_empty_args_output_lock_poisons_withdrawal() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let poisoned_lock = data1_script(&mut context, "owned_owner", Bytes::from(vec![1]));
    let mut spec = withdrawal(&env, &owner_lock);
    spec.inputs.push(env.funding_input());
    spec.outputs.push(plain(&poisoned_lock, 200 * CKB).into());
    spec.fail(&mut context, ERROR_NOT_EMPTY_ARGS);
}

// Scenario: the batch includes an unrelated typed output that does not use Owned Owner.
// Expectation: the foreign typed output is ignored and the valid withdrawal pair still verifies.
#[test]
fn foreign_typed_output_is_ignored() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let foreign_lock = named_always_success_lock(&mut context, b"foreign-lock");
    let foreign_type = named_always_success_lock(&mut context, b"foreign-type");
    let mut spec = withdrawal(&env, &owner_lock);
    spec.inputs.push(env.funding_input());
    spec.outputs.push(Cell { type_: Some(foreign_type), ..plain(&foreign_lock, 200 * CKB) }.into());
    spec.verify(&mut context)
        .expect("owned_owner should ignore unrelated foreign typed outputs during a valid withdrawal batch");
}

// Scenario: the batch includes a DAO-shaped sibling output that looks like Owned Owner but has non-empty args.
// Expectation: the plausible shape is still poison because the args are non-empty.
#[test]
fn owned_shaped_non_empty_args_output_poisons_withdrawal() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let poisoned_lock = data1_script(&mut context, "owned_owner", Bytes::from(vec![1]));
    let mut spec = withdrawal(&env, &owner_lock);
    let owned_shaped = Cell { lock: poisoned_lock, ..spec.outputs[0].cell.clone() };
    // A plain deposit at index 2, so the owned-shaped sibling is a well-formed request for it.
    let header = deposit_header(1555);
    let deposit = env.deposit(&env.user, 1_000 * CKB);
    let request = Cell { capacity: deposit.capacity, data: withdrawal_request_data(1555).to_vec(), ..owned_shaped };
    spec.inputs.push(In::new(deposit).in_block(&header));
    spec.header_deps.push(header);
    spec.outputs.push(request.into());
    spec.inputs.push(env.funding_input());
    spec.fail(&mut context, ERROR_NOT_EMPTY_ARGS);
}

// Scenario: a valid withdrawal consumes an owner-side sibling input that uses non-empty args.
// Expectation: Owned Owner rejects the whole batch because poisoned siblings are checked on inputs too.
#[test]
fn non_empty_args_owner_sibling_poisons_withdrawal() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let poisoned_owner_type = data1_script(&mut context, "owned_owner", Bytes::from(vec![1]));
    let mut spec = withdrawal(&env, &owner_lock);
    let poisoned_owner = Cell { capacity: 200 * CKB, type_: Some(poisoned_owner_type), ..env.owner(-1).under(&owner_lock) };
    spec.inputs.push(In::new(poisoned_owner));
    spec.fail(&mut context, ERROR_NOT_EMPTY_ARGS);
}
