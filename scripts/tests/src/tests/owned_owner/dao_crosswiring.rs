use super::*;

/// Phase 1 withdrawing deposits of `amount1` and `amount2` with user1's xUDT, where the owner cells
/// after both requests give user1 the first request and user2 the second.
fn crosswired(env: &Env, user1_lock: &Script, user2_lock: &Script, amount1: u64, amount2: u64) -> TxSpec {
    let mut spec = withdrawals(env, &[amount1, amount2]);
    spec.inputs.push(In::new(udt(env, user1_lock, u128::from(amount1 + amount2))));
    spec.inputs.push(funding(env));
    spec.outputs.push(owner(env, user1_lock, -1).into());
    spec.outputs.push(owner(env, user2_lock, -3).into());
    spec
}

// Even with weak locks, some crosswired layouts are still stopped by the DAO's same-index withdrawal rules before any later claim can rotate.
#[test]
fn crosswired_batch_is_blocked_by_dao_index_rules() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let user1_lock = named_always_success_lock(&mut context, b"user1");
    let user2_lock = named_always_success_lock(&mut context, b"user2");
    let mut spec = withdrawals(&env, &[1_000 * CKB, 1_100 * CKB]);
    spec.inputs.push(In::new(udt(&env, &user1_lock, u128::from(2_100 * CKB))));
    spec.outputs.insert(1, owner(&env, &user1_lock, 1).into());
    spec.outputs.push(owner(&env, &user2_lock, -3).into());
    spec.fail(&mut context, ERROR_DAO_INVALID_WITHDRAWING_CELL);
}

// This two-way batch keeps DAO index rules satisfied, so weak phase 1 owner locks are enough to rotate later claim ownership.
#[test]
fn weak_lock_valid_dao_batch_can_crosswire_claims() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let user1_lock = named_always_success_lock(&mut context, b"user1");
    let user2_lock = named_always_success_lock(&mut context, b"user2");
    crosswired(&env, &user1_lock, &user2_lock, 1_000 * CKB, 1_100 * CKB)
        .verify(&mut context)
        .expect("under weak owner locks, owned_owner accepts crosswired owner assignments when DAO index rules are still satisfied");
}

// The later phase 2 claim follows the crosswired owner cell, not the user's intended pairing, once phase 1 accepted the weak-lock batch.
#[test]
fn weak_lock_crosswired_batch_reassigns_phase2_claims() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let user1_lock = named_always_success_lock(&mut context, b"user1");
    let user2_lock = named_always_success_lock(&mut context, b"user2");
    let create = crosswired(&env, &user1_lock, &user2_lock, 1_000 * CKB, 1_200 * CKB);
    let live = accept(&mut context, &create, "under weak owner locks, the crosswired phase1 batch should verify");

    let withdraw_header2 = withdraw_header(2_000_621);
    TxSpec {
        inputs: vec![claim(live[1].clone(), &withdraw_header2, 0x2003e802340002f3, 1), live[2].clone()],
        outputs: vec![plain(&user1_lock, claim_capacity(&live[1].cell)).into()],
        header_deps: vec![withdraw_header2, deposit_header(1555)],
    }
    .verify(&mut context)
    .expect("under weak owner locks, user1's owner cell should successfully claim the crosswired second withdrawal request");

    let withdraw_header1 = withdraw_header(2_000_610);
    TxSpec {
        inputs: vec![claim(live[0].clone(), &withdraw_header1, 0x2003e800000002f4, 1), live[2].clone()],
        outputs: vec![plain(&user1_lock, claim_capacity(&live[0].cell)).into()],
        header_deps: vec![withdraw_header1, deposit_header(1554)],
    }
    .fail(&mut context, ERROR_OWNED_OWNER_MISMATCH);
}

// The same weak-lock claim rotation generalizes beyond two users: a three-way phase 1 crosswire can rotate later phase 2 ownership.
#[test]
fn weak_lock_three_way_crosswire_rotates_claims() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let user1_lock = named_always_success_lock(&mut context, b"user1");
    let user2_lock = named_always_success_lock(&mut context, b"user2");
    let user3_lock = named_always_success_lock(&mut context, b"user3");

    let mut create = withdrawals(&env, &[1_000 * CKB, 1_100 * CKB, 1_200 * CKB]);
    create.inputs.push(In::new(udt(&env, &user1_lock, u128::from(3_300 * CKB))));
    create.inputs.push(funding(&env));
    for (lock, distance) in [(&user1_lock, -2), (&user2_lock, -2), (&user3_lock, -5)] {
        create.outputs.push(Cell { capacity: 200 * CKB, ..owner(&env, lock, distance) }.into());
    }
    let live = accept(&mut context, &create, "under weak owner locks, owned_owner accepts a three-way crosswired withdrawal batch when DAO index rules still hold");

    let withdraw_header2 = withdraw_header(2_000_621);
    let claim_of = |owner: &In, lock: &Script| TxSpec {
        inputs: vec![claim(live[1].clone(), &withdraw_header2, 0x2003e802340002f3, 1), owner.clone()],
        outputs: vec![plain(lock, claim_capacity(&live[1].cell)).into()],
        header_deps: vec![withdraw_header2.clone(), deposit_header(1555)],
    };
    claim_of(&live[3], &user1_lock)
        .verify(&mut context)
        .expect("under weak owner locks, user1 should successfully claim user2's withdrawal request from a three-way crosswired batch");
    claim_of(&live[4], &user2_lock).fail(&mut context, ERROR_OWNED_OWNER_MISMATCH);
}
