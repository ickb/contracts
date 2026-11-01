use super::*;

/// Phase 1 of the canonical withdrawal with its owner cell moved under `foreign_lock`, accepted,
/// then the phase 2 claim of that pair paying 123,468,106,670 shannons to `owner_lock`.
fn claim_through_foreign_owner(context: &mut Context, env: &Env, owner_lock: &Script, foreign_lock: &Script, expect: &str) -> TxSpec {
    let mut create = withdrawal(env, owner_lock);
    create.outputs[1] = owner(env, foreign_lock, -1).into();
    let live = accept(context, &create, expect);
    let withdraw_header = withdraw_header(2_000_610);
    TxSpec {
        inputs: vec![claim(live[0].clone(), &withdraw_header, 0x2003e800000002f4, 1), live[1].clone()],
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

// Scenario: phase 1 tries to emit an owner cell whose lock and type are both limit-order driven.
// Expectation: Owned Owner blocks this stronger form immediately instead of letting it reach phase 2.
#[test]
fn limit_order_backed_owner_is_blocked_in_phase1() {
    let mut context = Context::default();
    let (_attacker_privkey, attacker_lock, _secp_data_dep) = secp_lock(&mut context);
    let env = Env::new(&mut context);
    let burn_lock = named_always_success_lock(&mut context, b"owner");
    let master_out_point = context.create_cell(
        cell(occupied_capacity(&attacker_lock, &env.limit_order, 0), &attacker_lock, Some(&env.limit_order)),
        Bytes::new(),
    );

    let owner_order_data = order_data_match(u128::from(u32::MAX), &master_out_point, (1, 1));
    let mut spec = withdrawal(&env, &burn_lock);
    spec.outputs[1] = Cell {
        capacity: deposit_capacity(&env.limit_order, &env.owned_owner, owner_order_data.len(), 1_500 * CKB),
        lock: env.limit_order.clone(),
        type_: Some(env.owned_owner.clone()),
        data: owner_order_data.to_vec(),
    }
    .into();
    spec.fail(&mut context, ERROR_DAO_INCORRECT_CAPACITY);
}
