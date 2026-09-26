use super::*;

// Build one tx that both creates a fresh deposit+receipt pair and converts an old receipt to xUDT, but overstates the minted xUDT by one shannon: the mixed flow must still conserve value, so verification fails.
#[test]
fn mixed_flow_cannot_overmint_by_combining_new_deposit_with_phase2_receipt() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, dao, xudt) = ickb_logic_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * CKB;
    let receipt_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(occupied_capacity(&funding_lock, &ickb_logic, 12).pack())
            .lock(funding_lock.clone())
            .type_(Some(ickb_logic.clone()).pack())
            .build(),
        receipt_data(1, deposit_amount),
    );
    let receipt_header = insert_header_for_cell(&mut context, &receipt_out_point, 0, GENESIS_AR);

    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((2_000 * CKB).pack())
            .lock(funding_lock.clone())
            .build(),
        Bytes::new(),
    );

    let deposit_output = CellOutput::new_builder()
        .capacity(deposit_capacity(&ickb_logic, &dao, 8, deposit_amount).pack())
        .lock(ickb_logic.clone())
        .type_(Some(dao).pack())
        .build();
    let new_receipt_output = CellOutput::new_builder()
        .capacity(occupied_capacity(&funding_lock, &ickb_logic, 12).pack())
        .lock(funding_lock.clone())
        .type_(Some(ickb_logic.clone()).pack())
        .build();
    let udt_output = CellOutput::new_builder()
        .capacity(occupied_capacity(&funding_lock, &xudt, 16).pack())
        .lock(funding_lock)
        .type_(Some(xudt).pack())
        .build();

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(receipt_out_point).build())
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .outputs(vec![deposit_output, new_receipt_output, udt_output])
        .outputs_data(
            vec![
                dao_deposit_data(),
                receipt_data(1, deposit_amount),
                udt_data(u128::from(deposit_amount) + 1),
            ]
            .pack(),
        )
        .header_dep(receipt_header.clone())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_AMOUNT_MISMATCH);
}

// Build one tx that simultaneously creates a new deposit, starts a withdrawal from another deposit, reissues a receipt, and remints xUDT by one extra shannon: even with phase1 and phase2 combined, the cross-flow accounting invariant should reject the overmint.
#[test]
fn mixed_flow_cannot_overmint_when_deposit_phase1_phase2_and_withdrawal_share_one_tx() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
    let ickb_logic = ickb_logic_script(&mut context);
    let owned_owner = owned_owner_script(&mut context);
    let xudt = xudt_script(&mut context, &ickb_logic);
    let dao = dao_script(&mut context);

    let deposit_amount = 1_000 * CKB;
    let receipt_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(occupied_capacity(&user_lock, &ickb_logic, 12).pack())
            .lock(user_lock.clone())
            .type_(Some(ickb_logic.clone()).pack())
            .build(),
        receipt_data(1, deposit_amount),
    );
    let receipt_header = insert_header_for_cell(&mut context, &receipt_out_point, 0, GENESIS_AR);

    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);
    let deposit_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_total_capacity.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    link_cell_to_header(&mut context, &deposit_input, &deposit_header);

    let udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(occupied_capacity(&user_lock, &xudt, 16).pack())
            .lock(user_lock.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        udt_data(u128::from(deposit_amount)),
    );
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((2_000 * CKB).pack())
            .lock(user_lock.clone())
            .build(),
        Bytes::new(),
    );

    let new_deposit_output = CellOutput::new_builder()
        .capacity(deposit_total_capacity.pack())
        .lock(ickb_logic.clone())
        .type_(Some(dao.clone()).pack())
        .build();
    let owned_output = CellOutput::new_builder()
        .capacity(deposit_total_capacity.pack())
        .lock(owned_owner.clone())
        .type_(Some(dao).pack())
        .build();
    let owner_output = CellOutput::new_builder()
        .capacity(occupied_capacity(&user_lock, &owned_owner, 4).pack())
        .lock(user_lock.clone())
        .type_(Some(owned_owner).pack())
        .build();
    let new_receipt_output = CellOutput::new_builder()
        .capacity(occupied_capacity(&user_lock, &ickb_logic, 12).pack())
        .lock(user_lock.clone())
        .type_(Some(ickb_logic.clone()).pack())
        .build();
    let udt_output = CellOutput::new_builder()
        .capacity(occupied_capacity(&user_lock, &xudt, 16).pack())
        .lock(user_lock)
        .type_(Some(xudt).pack())
        .build();

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(receipt_out_point).build())
        .input(CellInput::new_builder().previous_output(deposit_input).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .outputs(vec![new_deposit_output, owned_output, owner_output, new_receipt_output, udt_output])
        .outputs_data(
            vec![
                dao_deposit_data(),
                withdrawal_request_data(1554),
                owner_distance_data(-1),
                receipt_data(1, deposit_amount),
                udt_data(u128::from(deposit_amount) + 1),
            ]
            .pack(),
        )
        .header_dep(receipt_header.clone())
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_AMOUNT_MISMATCH);
}

// Build one live transition that consumes a receipt, a DAO deposit, and a real limit order, then emits a new limit order plus a withdrawal pair: all three script families should compose cleanly when each sibling shape and amount is valid, so verification passes.
#[test]
fn all_three_scripts_can_compose_in_one_live_state_transition() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let helper_type = helper_type_script(&mut context);
    let (real_order_out_point, real_master_out_point) =
        build_real_limit_order_and_master(&mut context, owner_lock.clone(), helper_type.clone());

    let (ickb_logic, limit_order) = ickb_logic_and_limit_order_scripts(&mut context);
    let owned_owner = owned_owner_script(&mut context);
    let dao = dao_script(&mut context);

    let deposit_amount = 1_000 * CKB;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);

    let receipt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(occupied_capacity(&owner_lock, &ickb_logic, 12).pack())
            .lock(owner_lock.clone())
            .type_(Some(ickb_logic.clone()).pack())
            .build(),
        receipt_data(1, deposit_amount),
    );
    link_cell_to_header(&mut context, &receipt_input, &deposit_header);

    let deposit_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_total_capacity.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    link_cell_to_header(&mut context, &deposit_input, &deposit_header);

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(receipt_input).build())
        .input(CellInput::new_builder().previous_output(deposit_input).build())
        .input(CellInput::new_builder().previous_output(real_order_out_point).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity((1_400 * CKB).pack())
                .lock(limit_order)
                .type_(Some(helper_type).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(deposit_total_capacity.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(occupied_capacity(&owner_lock, &owned_owner, 4).pack())
                .lock(owner_lock)
                .type_(Some(owned_owner).pack())
                .build(),
        ])
        .outputs_data(
            vec![
                order_data_match(100 * CKB as u128, &real_master_out_point, (1, 1)),
                withdrawal_request_data(1554),
                owner_distance_data(-1),
            ]
            .pack(),
        )
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("a live receipt, live deposit, and live limit order should compose in one valid transaction");
}

// Scenario 4F: withdrawing a deposit and re-depositing its value in one transaction must leave the
// withdrawal request at the deposit's index. Putting the fresh deposit there instead would restart
// the deposit's age in place; NervosDAO phase 1 rejects that layout.
#[test]
fn fresh_deposit_cannot_replace_a_withdrawn_deposit_at_its_index() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
    let (ickb_logic, dao, xudt) = ickb_logic_dao_and_xudt_scripts(&mut context);

    let amount = 1_500 * CKB;
    let deposit_total_capacity = deposit_capacity(&ickb_logic, &dao, 8, amount);
    let deposit_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_total_capacity.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    let deposit_header = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);
    link_cell_to_header(&mut context, &deposit_input, &deposit_header);
    let udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(occupied_capacity(&user_lock, &xudt, 16).pack())
            .lock(user_lock.clone())
            .type_(Some(xudt).pack())
            .build(),
        udt_data(u128::from(amount)),
    );
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((2 * deposit_total_capacity).pack())
            .lock(user_lock.clone())
            .build(),
        Bytes::new(),
    );

    let request = (
        CellOutput::new_builder()
            .capacity(deposit_total_capacity.pack())
            .lock(user_lock.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        withdrawal_request_data(1554),
    );
    let fresh_deposit = (
        CellOutput::new_builder()
            .capacity(deposit_total_capacity.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    let receipt = (
        CellOutput::new_builder()
            .capacity(occupied_capacity(&user_lock, &ickb_logic, 12).pack())
            .lock(user_lock.clone())
            .type_(Some(ickb_logic.clone()).pack())
            .build(),
        receipt_data(1, amount),
    );

    let tx = |outputs: Vec<(CellOutput, Bytes)>| {
        let (cells, data): (Vec<_>, Vec<_>) = outputs.into_iter().unzip();
        let tx: TransactionView = TransactionBuilder::default()
            .input(CellInput::new_builder().previous_output(deposit_input.clone()).build())
            .input(CellInput::new_builder().previous_output(udt_input.clone()).build())
            .input(CellInput::new_builder().previous_output(funding_input.clone()).build())
            .outputs(cells)
            .outputs_data(data.pack())
            .header_dep(deposit_header.hash())
            .build();
        tx
    };

    let withdraw_and_redeposit = context.complete_tx(tx(vec![request.clone(), fresh_deposit.clone(), receipt.clone()]));
    context
        .verify(&withdraw_and_redeposit, MAX_CYCLES)
        .expect("withdrawing with the request at the deposit's index and re-depositing elsewhere verifies");

    // Same cells with the two DAO outputs swapped: the fresh deposit takes the withdrawn deposit's index.
    let reset_in_place = context.complete_tx(tx(vec![fresh_deposit, request, receipt]));
    let err = context.verify(&reset_in_place, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_DAO_INVALID_WITHDRAWING_CELL);
}

// Scenario 6G: a deposit and its receipt created in the same block read the same accumulated rate,
// so a later transaction spending both (receipt to phase 2, deposit to withdrawal) mints nothing.
#[test]
fn same_block_receipt_and_deposit_mint_nothing_when_spent_together() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
    let (ickb_logic, dao, xudt) = ickb_logic_dao_and_xudt_scripts(&mut context);

    let amount = 1_500 * CKB;
    let deposit_total_capacity = deposit_capacity(&ickb_logic, &dao, 8, amount);
    let creation_header = gen_header(1554, SYNTHETIC_WITHDRAW_AR, 35, 1000, 1000);
    let deposit_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_total_capacity.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    let receipt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(occupied_capacity(&user_lock, &ickb_logic, 12).pack())
            .lock(user_lock.clone())
            .type_(Some(ickb_logic.clone()).pack())
            .build(),
        receipt_data(1, amount),
    );
    link_cell_to_header(&mut context, &deposit_input, &creation_header);
    link_cell_to_header(&mut context, &receipt_input, &creation_header);

    for minted in [0u128, 1] {
        let mut builder = TransactionBuilder::default()
            .input(CellInput::new_builder().previous_output(deposit_input.clone()).build())
            .input(CellInput::new_builder().previous_output(receipt_input.clone()).build())
            .output(
                CellOutput::new_builder()
                    .capacity(deposit_total_capacity.pack())
                    .lock(user_lock.clone())
                    .type_(Some(dao.clone()).pack())
                    .build(),
            )
            .output_data(withdrawal_request_data(1554).pack())
            .header_dep(creation_header.hash());
        if minted > 0 {
            builder = builder
                .output(
                    CellOutput::new_builder()
                        .capacity(occupied_capacity(&user_lock, &xudt, 16).pack())
                        .lock(user_lock.clone())
                        .type_(Some(xudt.clone()).pack())
                        .build(),
                )
                .output_data(udt_data(minted).pack());
        }
        let tx = context.complete_tx(builder.build());
        let result = context.verify(&tx, MAX_CYCLES);
        if minted == 0 {
            result.expect("the receipt's value exactly pays for the same-rate deposit");
        } else {
            assert_script_error(result.unwrap_err(), ERROR_AMOUNT_MISMATCH);
        }
    }
}
