use super::*;

// Build one deposit plus one receipt whose quantity field is zero: the tx shape includes a receipt cell, but the receipt claims no deposits, so creation fails on the non-empty-receipt invariant.
#[test]
fn zero_quantity_receipt_is_rejected() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, dao) = ickb_logic_and_dao_scripts(&mut context);
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((2_000 * SHANNONS).pack())
            .lock(funding_lock.clone())
            .build(),
        Bytes::new(),
    );

    let amount = 1_000 * SHANNONS;
    let deposit_output = CellOutput::new_builder()
        .capacity(deposit_capacity(&ickb_logic, &dao, 8, amount).pack())
        .lock(ickb_logic.clone())
        .type_(Some(dao).pack())
        .build();
    let receipt_output = CellOutput::new_builder()
        .capacity(capacity_for_data(12).pack())
        .lock(funding_lock)
        .type_(Some(ickb_logic).pack())
        .build();

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .outputs(vec![deposit_output, receipt_output])
        .outputs_data(vec![dao_deposit_data(), receipt_data(0, amount)].pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_EMPTY_RECEIPT);
}

// Build one deposit plus one receipt that claims quantity 2 for that deposit bucket: the receipt overstates how many equal deposits were created, so matching fails.
#[test]
fn forged_receipt_quantity_without_enough_deposits_is_rejected() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, dao) = ickb_logic_and_dao_scripts(&mut context);
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((2_000 * SHANNONS).pack())
            .lock(funding_lock.clone())
            .build(),
        Bytes::new(),
    );

    let amount = 1_000 * SHANNONS;
    let deposit_output = CellOutput::new_builder()
        .capacity(deposit_capacity(&ickb_logic, &dao, 8, amount).pack())
        .lock(ickb_logic.clone())
        .type_(Some(dao).pack())
        .build();
    let receipt_output = CellOutput::new_builder()
        .capacity(capacity_for_data(12).pack())
        .lock(funding_lock)
        .type_(Some(ickb_logic).pack())
        .build();

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .outputs(vec![deposit_output, receipt_output])
        .outputs_data(vec![dao_deposit_data(), receipt_data(2, amount)].pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_RECEIPT_MISMATCH);
}

// Build one 1000-CKB deposit plus one receipt that claims a 1001-CKB bucket: receipt matching is keyed by exact deposit amount, so the mismatched bucket is rejected.
#[test]
fn receipt_for_unmatched_deposit_amount_is_rejected() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, dao) = ickb_logic_and_dao_scripts(&mut context);
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((2_000 * SHANNONS).pack())
            .lock(funding_lock.clone())
            .build(),
        Bytes::new(),
    );

    let deposit_output = CellOutput::new_builder()
        .capacity(deposit_capacity(&ickb_logic, &dao, 8, 1_000 * SHANNONS).pack())
        .lock(ickb_logic.clone())
        .type_(Some(dao).pack())
        .build();
    let receipt_output = CellOutput::new_builder()
        .capacity(capacity_for_data(12).pack())
        .lock(funding_lock)
        .type_(Some(ickb_logic).pack())
        .build();

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .outputs(vec![deposit_output, receipt_output])
        .outputs_data(vec![dao_deposit_data(), receipt_data(1, 1_001 * SHANNONS)].pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_RECEIPT_MISMATCH);
}
