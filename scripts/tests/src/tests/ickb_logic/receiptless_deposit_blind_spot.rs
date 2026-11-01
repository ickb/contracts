use super::*;

/// A plain input of exactly `deposit`'s capacity creating it with no receipt.
fn create_receiptless(env: &Env, deposit: &Cell) -> TxSpec {
    TxSpec { inputs: vec![In::new(env.plain(deposit.capacity))], outputs: vec![deposit.clone().into()], ..Default::default() }
}

/// `cell` as the live output `index` of the verified `tx`, created in `header`'s block.
fn output_of(tx: &TransactionView, index: u32, cell: Cell, header: &HeaderView) -> In {
    In { out_point: OutPoint::new(tx.hash(), index), ..at(cell, header) }
}

// Build a plain funding tx that creates a DAO-typed ickb_logic output with deposit data but no receipt, then spend it in phase1 withdrawal mode: creation passes because output locks do not enforce receipt pairing, and the later withdrawal also passes because classification trusts the live deposit shape alone.
#[test]
fn receiptless_dao_shaped_output_is_accepted_as_deposit() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    // An empty-args lock like the deposit's, so the request keeps the deposit's lock size.
    let user_lock = always_success_lock(&mut context);

    let deposit_amount = 1_500 * CKB;
    let deposit = env.deposit(&env.ickb, deposit_amount);
    create_receiptless(&env, &deposit)
        .verify(&mut context)
        .expect("receiptless DAO-shaped ickb_logic output can be created at output-lock creation time");

    let deposit_header = gen_header(1554, GENESIS_AR, 35, 1000, 1000);
    let withdraw = TxSpec {
        inputs: vec![at(deposit.clone(), &deposit_header), In::new(env.udt(u128::from(deposit_amount)))],
        outputs: vec![request(&deposit, &user_lock, 1554).into()],
        header_deps: vec![deposit_header],
    };
    withdraw
        .verify(&mut context)
        .expect("later phase1 withdrawal request should accept the receiptless DAO-shaped output as a structurally valid deposit input");
}

// The deposit bounds live in ickb_logic's output check, which runs only when the creating tx also
// carries a receipt; a receiptless deposit of any size is created without it. Below the minimum (500 CKB
// and 0 CKB unoccupied), such a deposit is still withdrawn at its exact iCKB value.
#[test]
fn receiptless_deposits_below_the_minimum_are_created_and_withdrawn_at_value() {
    for deposit_amount in [500 * CKB, 0] {
        let mut context = Context::default();
        let env = Env::new(&mut context);
        // An empty-args lock like the deposit's, so the request keeps the deposit's lock size.
        let user_lock = always_success_lock(&mut context);
        let deposit = env.deposit(&env.ickb, deposit_amount);

        create_receiptless(&env, &deposit)
            .verify(&mut context)
            .expect("a receiptless deposit below the minimum is created without running the bounds check");

        let deposit_header = gen_header(1554, GENESIS_AR, 35, 1000, 1000);
        let deposit_input = at(deposit.clone(), &deposit_header);

        // At the genesis AR a deposit below the soft cap is worth exactly its unoccupied capacity.
        let mut burns = vec![u128::from(deposit_amount)];
        if deposit_amount > 0 {
            burns.push(u128::from(deposit_amount) - 1);
        }
        for burned in burns {
            let mut withdraw = TxSpec {
                inputs: vec![deposit_input.clone()],
                outputs: vec![request(&deposit, &user_lock, 1554).into()],
                header_deps: vec![deposit_header.clone()],
            };
            if burned > 0 {
                withdraw.inputs.push(In::new(env.udt(burned)));
            }
            let result = withdraw.verify(&mut context);
            if burned == u128::from(deposit_amount) {
                result.expect("burning exactly the deposit's value withdraws it");
            } else {
                assert_script_error(result.unwrap_err(), ERROR_AMOUNT_MISMATCH);
            }
        }
    }
}

// Build one tx that combines a legitimate split receipt with a separately funded receiptless aggregate deposit and rolls the aggregate into withdrawal: only the soft-cap spread should mint as xUDT, so the exact delta passes and any extra principal remains excluded.
#[test]
fn split_receipt_against_receiptless_aggregate_mints_only_spread() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    // An empty-args lock like the deposit's, so the request keeps the deposit's lock size.
    let user_lock = always_success_lock(&mut context);

    let split_amount = 100_000 * CKB;
    let aggregate_amount = 2 * split_amount;
    let soft_cap = u128::from(100_000 * CKB);
    let aggregate_ickb = u128::from(aggregate_amount);
    let aggregate_deposit_value = aggregate_ickb - (aggregate_ickb - soft_cap) / 10;
    let split_receipt_value = 2u128 * u128::from(split_amount);
    let delta = split_receipt_value - aggregate_deposit_value;
    let aggregate = env.deposit(&env.ickb, aggregate_amount);

    assert_eq!(delta, u128::from(10_000 * CKB));

    create_receiptless(&env, &aggregate)
        .verify(&mut context)
        .expect("aggregate receiptless deposit creation is allowed at output-lock creation time");

    let shared_header = gen_header(1554, GENESIS_AR, 35, 1000, 1000);

    // The receipt contributes only the valuation delta between per-deposit and aggregate soft-cap treatment.
    // The separately funded aggregate principal stays in the DAO withdrawal output below.
    let mut spec = TxSpec {
        inputs: vec![at(aggregate.clone(), &shared_header), at(env.receipt(2, split_amount), &shared_header), funding(&env)],
        outputs: vec![request(&aggregate, &user_lock, 1554).into(), env.udt(delta).into()],
        header_deps: vec![shared_header],
    };
    spec.verify(&mut context)
        .expect("split receipt should mint only the soft-cap spread while the separately funded aggregate principal stays in the withdrawal output");

    spec.outputs[1] = env.udt(delta + 1).into();
    spec.fail(&mut context, ERROR_AMOUNT_MISMATCH);
}

// Verify one continuous split-deposit receipt, receiptless creation, delta-only spread, and DAO phase2 claim trajectory: each later input keeps the preceding verified output's tx hash and index.
#[test]
fn verified_receiptless_creation_spread_and_claim_trajectory() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    // An empty-args lock like the deposit's, so the request keeps the deposit's lock size.
    let user_lock = always_success_lock(&mut context);

    let split_amount = 100_000 * CKB;
    let aggregate_amount = 2 * split_amount;
    let split = env.deposit(&env.ickb, split_amount);
    let aggregate = env.deposit(&env.ickb, aggregate_amount);
    let delta = 10_000 * CKB as u128;
    let deposit_header = gen_header(1554, GENESIS_AR, 35, 1000, 1000);
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);

    // Under the empty-args lock the receipt fits in the 100 CKB the funding input leaves for it.
    let receipt = sized(Cell { lock: user_lock.clone(), ..env.receipt(2, split_amount) });
    let split_tx = TxSpec {
        inputs: vec![In::new(env.plain(2 * split.capacity + capacity_for_data(16)))],
        outputs: vec![split.clone().into(), split.into(), receipt.clone().into()],
        ..Default::default()
    }
    .build(&mut context);
    context
        .verify(&split_tx, MAX_CYCLES)
        .expect("two split deposits should create their quantity-two receipt");

    let create_tx = create_receiptless(&env, &aggregate).build(&mut context);
    context
        .verify(&create_tx, MAX_CYCLES)
        .expect("aggregate receiptless deposit creation is allowed at output-lock creation time");

    // This is the same delta-only mint path as above, then a DAO phase2 claim.
    // The claim demonstrates that the self-funded aggregate principal stays spendable after minting only the spread.
    let request = request(&aggregate, &user_lock, 1554);
    let mint_tx = TxSpec {
        inputs: vec![
            output_of(&create_tx, 0, aggregate, &deposit_header),
            output_of(&split_tx, 2, receipt, &deposit_header),
            funding(&env),
        ],
        outputs: vec![request.clone().into(), env.udt(delta).into()],
        header_deps: vec![deposit_header.clone()],
    }
    .build(&mut context);
    context
        .verify(&mint_tx, MAX_CYCLES)
        .expect("split receipt should realize the soft-cap spread while rolling only the self-funded aggregate deposit into withdrawal");

    let claim_capacity = dao_maximum_withdraw_capacity(&request.output(), request.data.len(), GENESIS_AR, SYNTHETIC_WITHDRAW_AR);
    let claim = TxSpec {
        inputs: vec![In {
            since: 0x2003e800000002f4,
            witness: header_dep_index_witness(1),
            ..output_of(&mint_tx, 0, request, &withdraw_header)
        }],
        outputs: vec![Cell { lock: user_lock, ..env.plain(claim_capacity) }.into()],
        header_deps: vec![withdraw_header, deposit_header],
    };
    claim
        .verify(&mut context)
        .expect("the self-funded principal from the receiptless aggregate-deposit soft-cap path should remain spendable in DAO phase2");
}

// Build a receiptless aggregate deposit and try to mint the spread without any matching split receipt input: the blind spot alone does not create mint authority, so verification fails on amount mismatch.
#[test]
fn receiptless_aggregate_alone_cannot_mint_spread() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    // An empty-args lock like the deposit's, so the request keeps the deposit's lock size.
    let user_lock = always_success_lock(&mut context);

    let aggregate = env.deposit(&env.ickb, 2 * 100_000 * CKB);
    let delta = u128::from(10_000 * CKB);
    let deposit_header = gen_header(1554, GENESIS_AR, 35, 1000, 1000);

    let spec = TxSpec {
        inputs: vec![at(aggregate.clone(), &deposit_header)],
        outputs: vec![request(&aggregate, &user_lock, 1554).into(), env.udt(delta).into()],
        header_deps: vec![deposit_header],
    };
    spec.fail(&mut context, ERROR_AMOUNT_MISMATCH);
}

// Build the same receiptless-aggregate-plus-split-receipt pattern at 20x size: the larger aggregate should still pass while realizing a proportionally larger soft-cap spread, showing the blind spot scales with aggregate size.
#[test]
fn oversized_receiptless_aggregate_realizes_larger_spread() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    // An empty-args lock like the deposit's, so the request keeps the deposit's lock size.
    let user_lock = always_success_lock(&mut context);

    let split_amount = 100_000 * CKB;
    let quantity = 20u32;
    let aggregate_amount = u64::from(quantity) * split_amount;
    let aggregate = env.deposit(&env.ickb, aggregate_amount);
    let delta = u128::from(quantity) * u128::from(split_amount) - soft_capped_ickb(aggregate_amount, GENESIS_AR);
    assert_eq!(aggregate_amount, 2_000_000 * CKB);
    assert_eq!(delta, u128::from(190_000 * CKB));

    create_receiptless(&env, &aggregate)
        .verify(&mut context)
        .expect("oversized aggregate receiptless deposit can still be created at output-lock creation time");

    let shared_header = gen_header(1554, GENESIS_AR, 35, 1000, 1000);
    let spec = TxSpec {
        inputs: vec![at(aggregate.clone(), &shared_header), at(env.receipt(quantity, split_amount), &shared_header), funding(&env)],
        outputs: vec![request(&aggregate, &user_lock, 1554).into(), env.udt(delta).into()],
        header_deps: vec![shared_header],
    };
    spec.verify(&mut context)
        .expect("the oversized self-funded receiptless aggregate deposit should realize a larger soft-cap spread far past the intended per-deposit maximum");
}
