use super::*;

// Build a plain funding tx that creates a DAO-typed ickb_logic output with deposit data but no receipt, then spend it in phase1 withdrawal mode: creation passes because output locks do not enforce receipt pairing, and the later withdrawal also passes because classification trusts the live deposit shape alone.
#[test]
fn receiptless_dao_shaped_output_is_accepted_as_deposit() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
    let (ickb_logic, dao, xudt) = ickb_logic_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_500 * CKB;
    let deposit_total_capacity = deposit_capacity(&ickb_logic, &dao, 8, deposit_amount);
    let funding_input = context.create_cell(
        cell(deposit_total_capacity, &user_lock, None),
        Bytes::new(),
    );

    let create_tx = TransactionBuilder::default()
        .input(input(funding_input))
        .output(cell(deposit_total_capacity, &ickb_logic, Some(&dao)))
        .output_data(dao_deposit_data().pack())
        .build();

    let create_tx = context.complete_tx(create_tx);
    context
        .verify(&create_tx, MAX_CYCLES)
        .expect("receiptless DAO-shaped ickb_logic output can be created at output-lock creation time");

    let receiptless_deposit_input = create_deposit(&mut context, deposit_total_capacity, &ickb_logic, &dao);
    let deposit_header = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);
    link_cell_to_header(&mut context, &receiptless_deposit_input, &deposit_header);

    let udt_input = create_udt(&mut context, &user_lock, &xudt, u128::from(deposit_amount));

    let withdraw_tx = TransactionBuilder::default()
        .input(input(receiptless_deposit_input))
        .input(input(udt_input))
        .output(cell(deposit_total_capacity, &user_lock, Some(&dao)))
        .output_data(withdrawal_request_data(1554).pack())
        .header_dep(deposit_header.hash())
        .build();

    let withdraw_tx = context.complete_tx(withdraw_tx);
    context
        .verify(&withdraw_tx, MAX_CYCLES)
        .expect("later phase1 withdrawal request should accept the receiptless DAO-shaped output as a structurally valid deposit input");
}

// The deposit bounds live in ickb_logic's output check, which runs only when the creating tx also
// carries a receipt; a receiptless deposit of any size is created without it. Below the minimum (500 CKB
// and 0 CKB unoccupied), such a deposit is still withdrawn at its exact iCKB value.
#[test]
fn receiptless_deposits_below_the_minimum_are_created_and_withdrawn_at_value() {
    for deposit_amount in [500 * CKB, 0] {
        let mut context = Context::default();
        let user_lock = always_success_lock(&mut context);
        let (ickb_logic, dao, xudt) = ickb_logic_dao_and_xudt_scripts(&mut context);
        let deposit_total_capacity = deposit_capacity(&ickb_logic, &dao, 8, deposit_amount);

        let funding_input = context.create_cell(
            cell(deposit_total_capacity, &user_lock, None),
            Bytes::new(),
        );
        let create_tx = TransactionBuilder::default()
            .input(input(funding_input))
            .output(cell(deposit_total_capacity, &ickb_logic, Some(&dao)))
            .output_data(dao_deposit_data().pack())
            .build();
        let create_tx = context.complete_tx(create_tx);
        context
            .verify(&create_tx, MAX_CYCLES)
            .expect("a receiptless deposit below the minimum is created without running the bounds check");

        let deposit_input = create_deposit(&mut context, deposit_total_capacity, &ickb_logic, &dao);
        let deposit_header = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);
        link_cell_to_header(&mut context, &deposit_input, &deposit_header);

        // At the genesis AR a deposit below the soft cap is worth exactly its unoccupied capacity.
        let mut burns = vec![u128::from(deposit_amount)];
        if deposit_amount > 0 {
            burns.push(u128::from(deposit_amount) - 1);
        }
        for burned in burns {
            let mut builder = TransactionBuilder::default()
                .input(input(deposit_input.clone()));
            if burned > 0 {
                let udt_input = create_udt(&mut context, &user_lock, &xudt, burned);
                builder = builder.input(input(udt_input));
            }
            let withdraw_tx = builder
                .output(cell(deposit_total_capacity, &user_lock, Some(&dao)))
                .output_data(withdrawal_request_data(1554).pack())
                .header_dep(deposit_header.hash())
                .build();
            let withdraw_tx = context.complete_tx(withdraw_tx);
            let result = context.verify(&withdraw_tx, MAX_CYCLES);
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
    let user_lock = always_success_lock(&mut context);
    let (ickb_logic, dao, xudt) = ickb_logic_dao_and_xudt_scripts(&mut context);

    let split_amount = 100_000 * CKB;
    let aggregate_amount = 2 * split_amount;
    let soft_cap = u128::from(100_000 * CKB);
    let aggregate_ickb = u128::from(aggregate_amount);
    let aggregate_deposit_value = aggregate_ickb - (aggregate_ickb - soft_cap) / 10;
    let split_receipt_value = 2u128 * u128::from(split_amount);
    let delta = split_receipt_value - aggregate_deposit_value;
    let aggregate_total_capacity = deposit_capacity(&ickb_logic, &dao, 8, aggregate_amount);

    assert_eq!(delta, u128::from(10_000 * CKB));

    let funding_input = context.create_cell(
        cell(aggregate_total_capacity, &user_lock, None),
        Bytes::new(),
    );
    let create_tx = TransactionBuilder::default()
        .input(input(funding_input))
        .output(cell(aggregate_total_capacity, &ickb_logic, Some(&dao)))
        .output_data(dao_deposit_data().pack())
        .build();
    let create_tx = context.complete_tx(create_tx);
    context
        .verify(&create_tx, MAX_CYCLES)
        .expect("aggregate receiptless deposit creation is allowed at output-lock creation time");

    let receiptless_aggregate_deposit = create_deposit(&mut context, aggregate_total_capacity, &ickb_logic, &dao);
    let shared_header = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);
    link_cell_to_header(&mut context, &receiptless_aggregate_deposit, &shared_header);

    let receipt_input = create_receipt(&mut context, &user_lock, &ickb_logic, 2, split_amount);
    link_cell_to_header(&mut context, &receipt_input, &shared_header);

    // The receipt contributes only the valuation delta between per-deposit and aggregate soft-cap treatment.
    // The separately funded aggregate principal stays in the DAO withdrawal output below.
    let tx = TransactionBuilder::default()
        .input(input(receiptless_aggregate_deposit))
        .input(input(receipt_input))
        .input(input(funding_cell(&mut context)))
        .outputs(vec![
            cell(aggregate_total_capacity, &user_lock, Some(&dao)),
            cell(occupied_capacity(&user_lock, &xudt, 16), &user_lock, Some(&xudt)),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), udt_data(delta)].pack())
        .header_dep(shared_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("split receipt should mint only the soft-cap spread while the separately funded aggregate principal stays in the withdrawal output");

    let tampered_tx = tx
        .as_advanced_builder()
        .set_outputs_data(vec![withdrawal_request_data(1554).pack(), udt_data(delta + 1).pack()])
        .build();
    fail(&context, &tampered_tx, ERROR_AMOUNT_MISMATCH);
}

// Verify one continuous split-deposit receipt, receiptless creation, delta-only spread, and DAO phase2 claim trajectory: each later input keeps the preceding verified output's tx hash and index.
#[test]
fn verified_receiptless_creation_spread_and_claim_trajectory() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
    let (ickb_logic, dao, xudt) = ickb_logic_dao_and_xudt_scripts(&mut context);

    let split_amount = 100_000 * CKB;
    let aggregate_amount = 2 * split_amount;
    let split_total_capacity = deposit_capacity(&ickb_logic, &dao, 8, split_amount);
    let aggregate_total_capacity = deposit_capacity(&ickb_logic, &dao, 8, aggregate_amount);
    let delta = 10_000 * CKB as u128;
    let deposit_header = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);

    let split_funding_input = context.create_cell(
        cell(2 * split_total_capacity + capacity_for_data(16), &user_lock, None),
        Bytes::new(),
    );
    let split_tx = TransactionBuilder::default()
        .input(input(split_funding_input))
        .outputs(vec![
            cell(split_total_capacity, &ickb_logic, Some(&dao)),
            cell(split_total_capacity, &ickb_logic, Some(&dao)),
            cell(occupied_capacity(&user_lock, &ickb_logic, 12), &user_lock, Some(&ickb_logic)),
        ])
        .outputs_data(
            vec![
                dao_deposit_data(),
                dao_deposit_data(),
                receipt_data(2, split_amount),
            ]
            .pack(),
        )
        .build();
    let split_tx = context.complete_tx(split_tx);
    context
        .verify(&split_tx, MAX_CYCLES)
        .expect("two split deposits should create their quantity-two receipt");
    let split_deposit_1 = seed_verified_output(&mut context, &split_tx, 0, dao_deposit_data());
    let split_deposit_2 = seed_verified_output(&mut context, &split_tx, 1, dao_deposit_data());
    let receipt_input = seed_verified_output(
        &mut context,
        &split_tx,
        2,
        receipt_data(2, split_amount),
    );
    link_cell_to_header(&mut context, &split_deposit_1, &deposit_header);
    link_cell_to_header(&mut context, &split_deposit_2, &deposit_header);
    link_cell_to_header(&mut context, &receipt_input, &deposit_header);

    let funding_input = context.create_cell(
        cell(aggregate_total_capacity, &user_lock, None),
        Bytes::new(),
    );
    let create_tx = TransactionBuilder::default()
        .input(input(funding_input))
        .output(cell(aggregate_total_capacity, &ickb_logic, Some(&dao)))
        .output_data(dao_deposit_data().pack())
        .build();
    let create_tx = context.complete_tx(create_tx);
    context
        .verify(&create_tx, MAX_CYCLES)
        .expect("aggregate receiptless deposit creation is allowed at output-lock creation time");

    let receiptless_aggregate_deposit = seed_verified_output(
        &mut context,
        &create_tx,
        0,
        dao_deposit_data(),
    );
    link_cell_to_header(&mut context, &receiptless_aggregate_deposit, &deposit_header);

    // This is the same delta-only mint path as above, then a DAO phase2 claim.
    // The claim demonstrates that the self-funded aggregate principal stays spendable after minting only the spread.
    let mint_tx = TransactionBuilder::default()
        .input(input(receiptless_aggregate_deposit))
        .input(input(receipt_input))
        .input(input(funding_cell(&mut context)))
        .outputs(vec![
            cell(aggregate_total_capacity, &user_lock, Some(&dao)),
            cell(occupied_capacity(&user_lock, &xudt, 16), &user_lock, Some(&xudt)),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), udt_data(delta)].pack())
        .header_dep(deposit_header.hash())
        .build();
    let mint_tx = context.complete_tx(mint_tx);
    context
        .verify(&mint_tx, MAX_CYCLES)
        .expect("split receipt should realize the soft-cap spread while rolling only the self-funded aggregate deposit into withdrawal");

    let withdrawal_output = mint_tx.outputs().get(0).expect("withdrawing output");
    let withdrawal_out_point = seed_verified_output(
        &mut context,
        &mint_tx,
        0,
        withdrawal_request_data(1554),
    );
    link_cell_to_header(&mut context, &withdrawal_out_point, &withdraw_header);

    let witness = header_dep_index_witness(1);
    let claim_capacity = dao_maximum_withdraw_capacity(
        &withdrawal_output,
        withdrawal_request_data(1554).len(),
        GENESIS_AR as u64,
        SYNTHETIC_WITHDRAW_AR,
    );
    let claim_tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(withdrawal_out_point)
                .since(0x2003e800000002f4u64.pack())
                .build(),
        )
        .output(cell(claim_capacity, &user_lock, None))
        .output_data(Bytes::new().pack())
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_header.hash())
        .witness(witness.pack())
        .build();
    let claim_tx = context.complete_tx(claim_tx);
    context
        .verify(&claim_tx, MAX_CYCLES)
        .expect("the self-funded principal from the receiptless aggregate-deposit soft-cap path should remain spendable in DAO phase2");
}

// Build a receiptless aggregate deposit and try to mint the spread without any matching split receipt input: the blind spot alone does not create mint authority, so verification fails on amount mismatch.
#[test]
fn receiptless_aggregate_alone_cannot_mint_spread() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
    let (ickb_logic, dao, xudt) = ickb_logic_dao_and_xudt_scripts(&mut context);

    let aggregate_amount = 2 * 100_000 * CKB;
    let aggregate_total_capacity = deposit_capacity(&ickb_logic, &dao, 8, aggregate_amount);
    let delta = u128::from(10_000 * CKB);
    let deposit_header = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);

    let receiptless_aggregate_deposit = create_deposit(&mut context, aggregate_total_capacity, &ickb_logic, &dao);
    link_cell_to_header(&mut context, &receiptless_aggregate_deposit, &deposit_header);

    let tx = TransactionBuilder::default()
        .input(input(receiptless_aggregate_deposit))
        .outputs(vec![
            cell(aggregate_total_capacity, &user_lock, Some(&dao)),
            cell(occupied_capacity(&user_lock, &xudt, 16), &user_lock, Some(&xudt)),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), udt_data(delta)].pack())
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_AMOUNT_MISMATCH);
}

// Build the same receiptless-aggregate-plus-split-receipt pattern at 20x size: the larger aggregate should still pass while realizing a proportionally larger soft-cap spread, showing the blind spot scales with aggregate size.
#[test]
fn oversized_receiptless_aggregate_realizes_larger_spread() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
    let (ickb_logic, dao, xudt) = ickb_logic_dao_and_xudt_scripts(&mut context);

    let split_amount = 100_000 * CKB;
    let quantity = 20u32;
    let aggregate_amount = u64::from(quantity) * split_amount;
    let aggregate_total_capacity = deposit_capacity(&ickb_logic, &dao, 8, aggregate_amount);
    let delta = u128::from(quantity) * u128::from(split_amount) - soft_capped_ickb(aggregate_amount, GENESIS_AR);
    assert_eq!(aggregate_amount, 2_000_000 * CKB);
    assert_eq!(delta, u128::from(190_000 * CKB));

    let funding_input = context.create_cell(
        cell(aggregate_total_capacity, &user_lock, None),
        Bytes::new(),
    );
    let create_tx = TransactionBuilder::default()
        .input(input(funding_input))
        .output(cell(aggregate_total_capacity, &ickb_logic, Some(&dao)))
        .output_data(dao_deposit_data().pack())
        .build();
    let create_tx = context.complete_tx(create_tx);
    context
        .verify(&create_tx, MAX_CYCLES)
        .expect("oversized aggregate receiptless deposit can still be created at output-lock creation time");

    let receiptless_aggregate_deposit = create_deposit(&mut context, aggregate_total_capacity, &ickb_logic, &dao);
    let shared_header = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);
    link_cell_to_header(&mut context, &receiptless_aggregate_deposit, &shared_header);

    let receipt_input = create_receipt(&mut context, &user_lock, &ickb_logic, quantity, split_amount);
    link_cell_to_header(&mut context, &receipt_input, &shared_header);

    let tx = TransactionBuilder::default()
        .input(input(receiptless_aggregate_deposit))
        .input(input(receipt_input))
        .input(input(funding_cell(&mut context)))
        .outputs(vec![
            cell(aggregate_total_capacity, &user_lock, Some(&dao)),
            cell(occupied_capacity(&user_lock, &xudt, 16), &user_lock, Some(&xudt)),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), udt_data(delta)].pack())
        .header_dep(shared_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("the oversized self-funded receiptless aggregate deposit should realize a larger soft-cap spread far past the intended per-deposit maximum");
}
