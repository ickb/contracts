use super::*;

// If phase 1 owner locks are weak, a mixed foreign-plus-iCKB batch can crosswire who later controls each valid DAO claim.
#[test]
fn weak_lock_mixed_foreign_and_ickb_batch_can_crosswire_claims() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let foreign_owner_lock = named_always_success_lock(&mut context, b"foreign-owner");
    let protocol_owner_lock = named_always_success_lock(&mut context, b"protocol-owner");

    let foreign_deposit_number = 1554u64;
    let protocol_deposit_number = 1555u64;
    let foreign_deposit_header = deposit_header(foreign_deposit_number);
    let protocol_deposit_header = deposit_header(protocol_deposit_number);

    let foreign_deposit_capacity = 123_456_780_000u64;
    // The foreign deposit predates block 10,000,000, so the node's lock-size rule does not stop its
    // ordinary user lock from being wrapped.
    let foreign_deposit = Cell { capacity: foreign_deposit_capacity, ..env.deposit(&foreign_owner_lock, 0) };
    let protocol_deposit_amount = 1_000 * CKB;
    let protocol_deposit = env.deposit(&env.ickb, protocol_deposit_amount);

    let create = TxSpec {
        inputs: vec![
            In::new(foreign_deposit).in_block(&foreign_deposit_header),
            In::new(protocol_deposit.clone()).in_block(&protocol_deposit_header),
            In::new(env.udt(u128::from(protocol_deposit_amount)).under(&protocol_owner_lock)),
            env.funding_input(),
        ],
        outputs: vec![
            owned(&env, foreign_deposit_capacity, foreign_deposit_number).into(),
            owned(&env, protocol_deposit.capacity, protocol_deposit_number).into(),
            env.owner(-1).under(&foreign_owner_lock).into(),
            env.owner(-3).under(&protocol_owner_lock).into(),
        ],
        header_deps: vec![foreign_deposit_header.clone(), protocol_deposit_header.clone()],
    };
    let live = create.accept(&mut context, "owned_owner should accept a weak-lock mixed foreign-plus-iCKB withdrawal batch with crosswired later claim assignments");

    let withdraw_header = withdraw_header(2_000_610);
    let claim_by = |owner: &In, lock: &Script| TxSpec {
        inputs: vec![claim(live[1].clone(), &withdraw_header, 0x2003e802340002f3, 2), owner.clone()],
        outputs: vec![plain(lock, claim_capacity(&live[1].cell)).into()],
        header_deps: vec![withdraw_header.clone(), foreign_deposit_header.clone(), protocol_deposit_header.clone()],
    };
    claim_by(&live[2], &foreign_owner_lock)
        .verify(&mut context)
        .expect("under weak owner locks, the foreign owner cell should be able to claim the real iCKB withdrawal once the mixed batch crosswires the later claim assignment");
    claim_by(&live[3], &protocol_owner_lock).fail(&mut context, ERROR_OWNED_OWNER_MISMATCH);
}
