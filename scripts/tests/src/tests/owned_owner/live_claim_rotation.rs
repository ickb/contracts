use super::*;

// Live claim rotation and pair-swapping limits.
// A live claim cannot be re-emitted as a fresh Owned Owner pair during phase 2: DAO withdrawal semantics block the rotation before a new live pair exists.
#[test]
fn live_claim_cannot_roll_into_fresh_pair() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let old_owner_lock = named_always_success_lock(&mut context, b"old-owner");
    let new_owner_lock = named_always_success_lock(&mut context, b"new-owner");
    let mut spec = melt(&env, 9, env.owner(-1).under(&old_owner_lock));
    spec.outputs = vec![owned(&env, 123_456_780_000, 1554).into(), env.owner(-1).under(&new_owner_lock).into()];
    let tx = spec.build(&mut context);
    // The node also requires a deposit, not a claim, at a new request's index: the same rule under test.
    assert_script_error(context.verify_where_node_also_rejects(&tx, MAX_CYCLES, "The DAO output data at index 0").unwrap_err(), ERROR_DAO_NEWLY_CREATED_CELL);
}

// Consuming multiple live pairs still cannot rotate them into a new set of live pairs once the full DAO claim rules are enforced.
#[test]
fn live_claims_cannot_rotate_into_new_pairs() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let user1_lock = named_always_success_lock(&mut context, b"user1");
    let user2_lock = named_always_success_lock(&mut context, b"user2");

    let mut create = withdrawals(&env, &[1_000 * CKB, 1_100 * CKB]);
    create.inputs.push(In::new(env.udt(u128::from(2_100 * CKB)).under(&user1_lock)));
    create.inputs.push(env.funding_input());
    create.outputs.push(env.owner(-2).under(&user1_lock).into());
    create.outputs.push(env.owner(-2).under(&user2_lock).into());
    let live = create.accept(&mut context, "initial live owned_owner pairs should verify");

    let (withdraw_header1, withdraw_header2) = (withdraw_header(2_000_610), withdraw_header(2_000_621));
    let spec = TxSpec {
        inputs: vec![
            claim(live[0].clone(), &withdraw_header1, 0x2003e800000002f4, 2),
            claim(live[1].clone(), &withdraw_header2, 0x2003e802340002f3, 3),
            live[2].clone(),
            live[3].clone(),
        ],
        outputs: vec![
            create.outputs[0].clone(),
            create.outputs[1].clone(),
            env.owner(-1).under(&user1_lock).into(),
            env.owner(-3).under(&user2_lock).into(),
        ],
        header_deps: vec![withdraw_header1, withdraw_header2, deposit_header(1554), deposit_header(1555)],
    };
    let tx = spec.build(&mut context);
    // The node also requires a deposit, not a claim, at a new request's index: the same rule under test.
    assert_script_error(context.verify_where_node_also_rejects(&tx, MAX_CYCLES, "The DAO output data at index 0").unwrap_err(), ERROR_DAO_NEWLY_CREATED_CELL);
}
