use super::*;

// Build phase2 mint attempts for one above-cap receipt: minting the raw deposit amount must fail, while minting the soft-capped amount passes, proving the discount starts immediately above the 100k boundary.
#[test]
fn phase2_mint_applies_soft_cap_discount_above_boundary() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let amount = 100_001 * SHANNONS;
    let receipt_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(12).pack())
            .lock(funding_lock.clone())
            .type_(Some(ickb_logic.clone()).pack())
            .build(),
        receipt_data(1, amount),
    );
    let receipt_header = insert_header_for_cell(&mut context, &receipt_out_point, 0, GENESIS_AR);

    let naive_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(receipt_out_point.clone()).build())
        .output(
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(funding_lock.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
        )
        .output_data(udt_data(u128::from(amount)).pack())
        .header_dep(receipt_header.clone())
        .build();
    let naive_tx = context.complete_tx(naive_tx);
    let err = context.verify_tx(&naive_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_AMOUNT_MISMATCH);

    let exact_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(receipt_out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(funding_lock)
                .type_(Some(xudt).pack())
                .build(),
        )
        .output_data(udt_data(soft_capped_ickb(amount, GENESIS_AR)).pack())
        .header_dep(receipt_header.clone())
        .build();
    let exact_tx = context.complete_tx(exact_tx);
    context
        .verify_tx(&exact_tx, MAX_CYCLES)
        .expect("phase2 mint should apply the documented soft-cap discount above 100k iCKB");
}

// Build a phase2 mint for a receipt exactly at 100k CKB: this is the soft-cap boundary itself, so no discount should apply and the full amount should pass.
#[test]
fn phase2_mint_does_not_discount_at_soft_cap_boundary() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let amount = 100_000 * SHANNONS;
    assert_eq!(soft_capped_ickb(amount, GENESIS_AR), u128::from(amount));

    let receipt_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(12).pack())
            .lock(funding_lock.clone())
            .type_(Some(ickb_logic).pack())
            .build(),
        receipt_data(1, amount),
    );
    let receipt_header = insert_header_for_cell(&mut context, &receipt_out_point, 0, GENESIS_AR);

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(receipt_out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(funding_lock)
                .type_(Some(xudt).pack())
                .build(),
        )
        .output_data(udt_data(u128::from(amount)).pack())
        .header_dep(receipt_header)
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("phase2 mint should keep the full amount exactly at the 100k soft-cap boundary");
}

// Build a quantity-2 phase2 mint where each receipt amount is just above the cap: the tx should reject a single aggregate haircut and accept the per-deposit haircut, proving the soft cap is applied per receipt bucket.
#[test]
fn multi_quantity_phase2_mint_applies_soft_cap_per_deposit() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let amount = 100_001 * SHANNONS;
    let quantity = 2u32;
    let receipt_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(12).pack())
            .lock(funding_lock.clone())
            .type_(Some(ickb_logic.clone()).pack())
            .build(),
        receipt_data(quantity, amount),
    );
    let receipt_header = insert_header_for_cell(&mut context, &receipt_out_point, 0, GENESIS_AR);
    let exact = u128::from(quantity) * soft_capped_ickb(amount, GENESIS_AR);
    let total_raw = u128::from(quantity) * u128::from(amount);
    let soft_cap = u128::from(100_000 * SHANNONS);
    let naive_aggregate = total_raw - (total_raw - soft_cap) / 10;
    assert!(exact > naive_aggregate);

    let naive_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(receipt_out_point.clone()).build())
        .output(
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(funding_lock.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
        )
        .output_data(udt_data(naive_aggregate).pack())
        .header_dep(receipt_header.clone())
        .build();
    let naive_tx = context.complete_tx(naive_tx);
    let err = context.verify_tx(&naive_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_AMOUNT_MISMATCH);

    let exact_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(receipt_out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(funding_lock)
                .type_(Some(xudt).pack())
                .build(),
        )
        .output_data(udt_data(exact).pack())
        .header_dep(receipt_header.clone())
        .build();
    let exact_tx = context.complete_tx(exact_tx);
    context
        .verify_tx(&exact_tx, MAX_CYCLES)
        .expect("multi-quantity receipts should apply the soft cap per deposit, not once on the aggregate");
}

// Build one mixed tx that both recreates two above-cap deposits and converts a quantity-2 receipt: even with phase1 and phase2 combined, the mint must still use the per-receipt soft cap, so aggregate-only discounting fails and the exact per-receipt amount passes.
#[test]
fn mixed_phase1_phase2_still_apply_soft_cap_per_receipt() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, dao, xudt) = ickb_logic_dao_and_xudt_scripts(&mut context);

    let amount = 100_001 * SHANNONS;
    let quantity = 2u32;
    let deposit_output_capacity = deposit_capacity(&ickb_logic, &dao, 8, amount);
    let receipt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(12).pack())
            .lock(funding_lock.clone())
            .type_(Some(ickb_logic.clone()).pack())
            .build(),
        receipt_data(quantity, amount),
    );
    let receipt_header = insert_header_for_cell(&mut context, &receipt_input, 0, GENESIS_AR);
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((300_000 * SHANNONS).pack())
            .lock(funding_lock.clone())
            .build(),
        Bytes::new(),
    );

    let exact = u128::from(quantity) * soft_capped_ickb(amount, GENESIS_AR);
    let total_raw = u128::from(quantity) * u128::from(amount);
    let soft_cap = u128::from(100_000 * SHANNONS);
    let naive_aggregate = total_raw - (total_raw - soft_cap) / 10;
    assert!(exact > naive_aggregate);

    let build_tx = |udt_amount: u128| {
        TransactionBuilder::default()
            .input(CellInput::new_builder().previous_output(funding_input.clone()).build())
            .input(CellInput::new_builder().previous_output(receipt_input.clone()).build())
            .outputs(vec![
                CellOutput::new_builder()
                    .capacity(deposit_output_capacity.pack())
                    .lock(ickb_logic.clone())
                    .type_(Some(dao.clone()).pack())
                    .build(),
                CellOutput::new_builder()
                    .capacity(deposit_output_capacity.pack())
                    .lock(ickb_logic.clone())
                    .type_(Some(dao.clone()).pack())
                    .build(),
                CellOutput::new_builder()
                    .capacity(capacity_for_data(12).pack())
                    .lock(funding_lock.clone())
                    .type_(Some(ickb_logic.clone()).pack())
                    .build(),
                CellOutput::new_builder()
                    .capacity(capacity_for_data(16).pack())
                    .lock(funding_lock.clone())
                    .type_(Some(xudt.clone()).pack())
                    .build(),
            ])
            .outputs_data(
                vec![
                    dao_deposit_data(),
                    dao_deposit_data(),
                    receipt_data(quantity, amount),
                    udt_data(udt_amount),
                ]
                .pack(),
            )
            .header_dep(receipt_header.clone())
            .build()
    };

    let naive_tx = context.complete_tx(build_tx(naive_aggregate));
    let err = context.verify_tx(&naive_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_AMOUNT_MISMATCH);

    let exact_tx = context.complete_tx(build_tx(exact));
    context
        .verify_tx(&exact_tx, MAX_CYCLES)
        .expect("fresh deposit creation in the same transaction should not let above-cap receipt conversion escape the per-receipt soft-cap discount");
}
