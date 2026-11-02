use super::*;

/// Phase 1 of the canonical withdrawal with its owner cell moved under `foreign_lock`, accepted,
/// then the phase 2 claim of that pair paying 123,468,106,670 shannons to `owner_lock`, funded by the wallet.
fn claim_through_foreign_owner(context: &mut Context, env: &Env, owner_lock: &Script, foreign_lock: &Script, expect: &str) -> TxSpec {
    let mut create = withdrawal(env, owner_lock);
    create.outputs[1] = (env).owner(-1).under(foreign_lock).into();
    let live = create.accept(context, expect);
    let withdraw_header = withdraw_header(2_000_610);
    TxSpec {
        inputs: vec![claim(live[0].clone(), &withdraw_header, 0x2003e800000002f4, 1), live[1].clone(), env.funding_input()],
        outputs: vec![plain(owner_lock, 123_468_106_670).into()],
        header_deps: vec![withdraw_header, deposit_header(1554)],
    }
}

// Creation-time output checks do not block outputs that later become unspendable.
// Phase 1 accepts an owner cell whose lock never executed on creation, but the later claim fails once that foreign lock has to run.
#[test]
fn phase1_accepts_unspendable_foreign_owner_lock() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let poisoned_lock = data1_script(&mut context, "limit_order", Bytes::from(vec![1]));
    claim_through_foreign_owner(&mut context, &env, &owner_lock, &poisoned_lock, "owned_owner creation accepts an owner cell whose lock never executed on creation")
        .fail(&mut context, ERROR_NOT_EMPTY_ARGS);
}

// Even an empty-args foreign lock can strand the pair later if the eventual owner lock semantics do not match the claim flow.
#[test]
fn phase1_accepts_limit_order_owner_lock_but_claim_strands() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    claim_through_foreign_owner(&mut context, &env, &owner_lock, &env.limit_order, "owned_owner creation accepts an empty-args limit_order owner lock at creation")
        .fail(&mut context, ERROR_ENCODING);
}

// Phase 1 may emit an owner cell locked by limit_order whose data reads both as a live order
// (pointing at a master) and as owner distance -1. Owned Owner never checks the owner lock at
// creation, so this is accepted, and whoever holds that order's master can later melt the owner
// cell and take the whole claim: the owner lock decides who claims, whatever that lock is.
#[test]
fn limit_order_backed_owner_hands_the_claim_to_the_master_holder() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let burn_lock = named_always_success_lock(&mut context, b"owner");
    let holder_lock = named_always_success_lock(&mut context, b"master-holder");
    let master = env.master().under(&holder_lock);
    let master_input = In::new(master.clone());
    let owner_order_data = order_data_match(u128::from(u32::MAX), &master_input.out_point, (1, 1));
    let mut phase1 = withdrawal(&env, &burn_lock);
    phase1.outputs[1] = Cell {
        capacity: deposit_capacity(&env.limit_order, &env.owned_owner, owner_order_data.len(), 1_500 * CKB),
        lock: env.limit_order.clone(),
        type_: Some(env.owned_owner.clone()),
        data: owner_order_data.to_vec(),
    }
    .into();
    phase1.inputs.push(In::new(plain(&env.funding, 2_000 * CKB)));
    let created = phase1.accept(&mut context, "phase 1 accepts a limit-order-backed owner cell");

    let withdrawn = withdraw_header(2_000_610);
    let claimed = claim_capacity(&created[0].cell) + created[1].cell.capacity + master.capacity;
    TxSpec {
        inputs: vec![
            claim(created[0].clone(), &withdrawn, 0x2003e800000002f4, 1),
            created[1].clone(),
            master_input,
        ],
        outputs: vec![plain(&holder_lock, claimed).into()],
        header_deps: vec![withdrawn, deposit_header(1554)],
    }
    .verify(&mut context)
    .expect("the master holder melts the owner cell and claims the withdrawal");
}
