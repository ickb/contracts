use super::*;

// Build a plain funding tx that creates a DAO-typed ickb_logic output with deposit data but no receipt, then spend it in phase1 withdrawal mode: creation passes because output locks do not enforce receipt pairing, and the later withdrawal also passes because classification trusts the live deposit shape alone.
#[test]
fn receiptless_dao_shaped_output_is_accepted_as_deposit() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
    let (ickb_logic, dao, xudt) = ickb_logic_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_500 * SHANNONS;
    let deposit_total_capacity = deposit_capacity(&ickb_logic, &dao, 8, deposit_amount);
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_total_capacity.pack())
            .lock(user_lock.clone())
            .build(),
        Bytes::new(),
    );

    let create_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .output(
            CellOutput::new_builder()
                .capacity(deposit_total_capacity.pack())
                .lock(ickb_logic.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
        )
        .output_data(dao_deposit_data().pack())
        .build();

    let create_tx = context.complete_tx(create_tx);
    context
        .verify_tx(&create_tx, MAX_CYCLES)
        .expect("receiptless DAO-shaped ickb_logic output can be created at output-lock creation time");

    let receiptless_deposit_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_total_capacity.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    let deposit_header = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);
    link_cell_to_header(&mut context, &receiptless_deposit_input, &deposit_header);

    let udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(user_lock.clone())
            .type_(Some(xudt).pack())
            .build(),
        udt_data(u128::from(deposit_amount)),
    );

    let withdraw_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(receiptless_deposit_input).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .output(
            CellOutput::new_builder()
                .capacity(deposit_total_capacity.pack())
                .lock(user_lock)
                .type_(Some(dao).pack())
                .build(),
        )
        .output_data(withdrawal_request_data(1554).pack())
        .header_dep(deposit_header.hash())
        .build();

    let withdraw_tx = context.complete_tx(withdraw_tx);
    context
        .verify_tx(&withdraw_tx, MAX_CYCLES)
        .expect("later phase1 withdrawal request should accept the receiptless DAO-shaped output as a structurally valid deposit input");
}

// Build one tx that combines a legitimate split receipt with a separately funded receiptless aggregate deposit and rolls the aggregate into withdrawal: only the soft-cap spread should mint as xUDT, so the exact delta passes and any extra principal remains excluded.
#[test]
fn split_receipt_against_receiptless_aggregate_mints_only_spread() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
    let (ickb_logic, dao, xudt) = ickb_logic_dao_and_xudt_scripts(&mut context);

    let split_amount = 100_000 * SHANNONS;
    let aggregate_amount = 2 * split_amount;
    let soft_cap = u128::from(100_000 * SHANNONS);
    let aggregate_ickb = u128::from(aggregate_amount);
    let aggregate_deposit_value = aggregate_ickb - (aggregate_ickb - soft_cap) / 10;
    let split_receipt_value = 2u128 * u128::from(split_amount);
    let delta = split_receipt_value - aggregate_deposit_value;
    let aggregate_total_capacity = deposit_capacity(&ickb_logic, &dao, 8, aggregate_amount);

    assert_eq!(delta, u128::from(10_000 * SHANNONS));

    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(aggregate_total_capacity.pack())
            .lock(user_lock.clone())
            .build(),
        Bytes::new(),
    );
    let create_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .output(
            CellOutput::new_builder()
                .capacity(aggregate_total_capacity.pack())
                .lock(ickb_logic.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
        )
        .output_data(dao_deposit_data().pack())
        .build();
    let create_tx = context.complete_tx(create_tx);
    context
        .verify_tx(&create_tx, MAX_CYCLES)
        .expect("aggregate receiptless deposit creation is allowed at output-lock creation time");

    let receiptless_aggregate_deposit = context.create_cell(
        CellOutput::new_builder()
            .capacity(aggregate_total_capacity.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    let shared_header = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);
    link_cell_to_header(&mut context, &receiptless_aggregate_deposit, &shared_header);

    let receipt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(user_lock.clone())
            .type_(Some(ickb_logic).pack())
            .build(),
        receipt_data(2, split_amount),
    );
    link_cell_to_header(&mut context, &receipt_input, &shared_header);

    // The receipt contributes only the valuation delta between per-deposit and aggregate soft-cap treatment.
    // The separately funded aggregate principal stays in the DAO withdrawal output below.
    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(receiptless_aggregate_deposit).build())
        .input(CellInput::new_builder().previous_output(receipt_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(aggregate_total_capacity.pack())
                .lock(user_lock.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(user_lock)
                .type_(Some(xudt).pack())
                .build(),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), udt_data(delta)].pack())
        .header_dep(shared_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("split receipt should mint only the soft-cap spread while the separately funded aggregate principal stays in the withdrawal output");

    let tampered_tx = tx
        .as_advanced_builder()
        .set_outputs_data(vec![withdrawal_request_data(1554).pack(), udt_data(delta + 1).pack()])
        .build();
    let err = context.verify_tx(&tampered_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_AMOUNT_MISMATCH);
}

// Build the same delta-only spread path and then complete the DAO phase2 claim on the rolled withdrawal cell: both steps should pass, showing the self-funded aggregate principal stays recoverable even though only the spread minted in phase1.
#[test]
fn spread_path_keeps_self_funded_principal_claimable() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
    let (ickb_logic, dao, xudt) = ickb_logic_dao_and_xudt_scripts(&mut context);

    let split_amount = 100_000 * SHANNONS;
    let aggregate_amount = 2 * split_amount;
    let aggregate_total_capacity = deposit_capacity(&ickb_logic, &dao, 8, aggregate_amount);
    let delta = 10_000 * SHANNONS as u128;
    let deposit_header = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);

    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(aggregate_total_capacity.pack())
            .lock(user_lock.clone())
            .build(),
        Bytes::new(),
    );
    let create_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .output(
            CellOutput::new_builder()
                .capacity(aggregate_total_capacity.pack())
                .lock(ickb_logic.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
        )
        .output_data(dao_deposit_data().pack())
        .build();
    let create_tx = context.complete_tx(create_tx);
    context
        .verify_tx(&create_tx, MAX_CYCLES)
        .expect("aggregate receiptless deposit creation is allowed at output-lock creation time");

    let receiptless_aggregate_deposit = context.create_cell(
        CellOutput::new_builder()
            .capacity(aggregate_total_capacity.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    link_cell_to_header(&mut context, &receiptless_aggregate_deposit, &deposit_header);

    let receipt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(user_lock.clone())
            .type_(Some(ickb_logic.clone()).pack())
            .build(),
        receipt_data(2, split_amount),
    );
    link_cell_to_header(&mut context, &receipt_input, &deposit_header);

    // This is the same delta-only mint path as above, then a DAO phase2 claim.
    // The claim demonstrates that the self-funded aggregate principal stays spendable after minting only the spread.
    let mint_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(receiptless_aggregate_deposit).build())
        .input(CellInput::new_builder().previous_output(receipt_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(aggregate_total_capacity.pack())
                .lock(user_lock.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(user_lock.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), udt_data(delta)].pack())
        .header_dep(deposit_header.hash())
        .build();
    let mint_tx = context.complete_tx(mint_tx);
    context
        .verify_tx(&mint_tx, MAX_CYCLES)
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
        .output(
            CellOutput::new_builder()
                .capacity(claim_capacity.pack())
                .lock(user_lock)
                .build(),
        )
        .output_data(Bytes::new().pack())
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_header.hash())
        .witness(witness.pack())
        .build();
    let claim_tx = context.complete_tx(claim_tx);
    context
        .verify_tx(&claim_tx, MAX_CYCLES)
        .expect("the self-funded principal from the receiptless aggregate-deposit soft-cap path should remain spendable in DAO phase2");
}

// Build a receiptless aggregate deposit and try to mint the spread without any matching split receipt input: the blind spot alone does not create mint authority, so verification fails on amount mismatch.
#[test]
fn receiptless_aggregate_alone_cannot_mint_spread() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
    let (ickb_logic, dao, xudt) = ickb_logic_dao_and_xudt_scripts(&mut context);

    let aggregate_amount = 2 * 100_000 * SHANNONS;
    let aggregate_total_capacity = deposit_capacity(&ickb_logic, &dao, 8, aggregate_amount);
    let delta = u128::from(10_000 * SHANNONS);
    let deposit_header = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);

    let receiptless_aggregate_deposit = context.create_cell(
        CellOutput::new_builder()
            .capacity(aggregate_total_capacity.pack())
            .lock(ickb_logic)
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    link_cell_to_header(&mut context, &receiptless_aggregate_deposit, &deposit_header);

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(receiptless_aggregate_deposit).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(aggregate_total_capacity.pack())
                .lock(user_lock.clone())
                .type_(Some(dao).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(user_lock)
                .type_(Some(xudt).pack())
                .build(),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), udt_data(delta)].pack())
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_AMOUNT_MISMATCH);
}

// Build the same receiptless-aggregate-plus-split-receipt pattern at 20x size: the larger aggregate should still pass while realizing a proportionally larger soft-cap spread, showing the blind spot scales with aggregate size.
#[test]
fn oversized_receiptless_aggregate_realizes_larger_spread() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
    let (ickb_logic, dao, xudt) = ickb_logic_dao_and_xudt_scripts(&mut context);

    let split_amount = 100_000 * SHANNONS;
    let quantity = 20u32;
    let aggregate_amount = u64::from(quantity) * split_amount;
    let aggregate_total_capacity = deposit_capacity(&ickb_logic, &dao, 8, aggregate_amount);
    let delta = u128::from(quantity) * u128::from(split_amount) - soft_capped_ickb(aggregate_amount, GENESIS_AR);
    assert_eq!(aggregate_amount, 2_000_000 * SHANNONS);
    assert_eq!(delta, u128::from(190_000 * SHANNONS));

    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(aggregate_total_capacity.pack())
            .lock(user_lock.clone())
            .build(),
        Bytes::new(),
    );
    let create_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .output(
            CellOutput::new_builder()
                .capacity(aggregate_total_capacity.pack())
                .lock(ickb_logic.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
        )
        .output_data(dao_deposit_data().pack())
        .build();
    let create_tx = context.complete_tx(create_tx);
    context
        .verify_tx(&create_tx, MAX_CYCLES)
        .expect("oversized aggregate receiptless deposit can still be created at output-lock creation time");

    let receiptless_aggregate_deposit = context.create_cell(
        CellOutput::new_builder()
            .capacity(aggregate_total_capacity.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    let shared_header = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);
    link_cell_to_header(&mut context, &receiptless_aggregate_deposit, &shared_header);

    let receipt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(user_lock.clone())
            .type_(Some(ickb_logic.clone()).pack())
            .build(),
        receipt_data(quantity, split_amount),
    );
    link_cell_to_header(&mut context, &receipt_input, &shared_header);

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(receiptless_aggregate_deposit).build())
        .input(CellInput::new_builder().previous_output(receipt_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(aggregate_total_capacity.pack())
                .lock(user_lock.clone())
                .type_(Some(dao).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(user_lock)
                .type_(Some(xudt).pack())
                .build(),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), udt_data(delta)].pack())
        .header_dep(shared_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("the oversized self-funded receiptless aggregate deposit should realize a larger soft-cap spread far past the intended per-deposit maximum");
}
