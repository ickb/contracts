use super::*;

// Build one DAO deposit plus one matching receipt with amount 999 CKB: this exercises the lower deposit bound, so creation must fail because the claimed deposit bucket is below the minimum supported size.
#[test]
fn deposit_below_minimum_is_rejected() {
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

    let amount = 999 * SHANNONS;
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
        .outputs_data(vec![dao_deposit_data(), receipt_data(1, amount)].pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_DEPOSIT_TOO_SMALL);
}

// Build one DAO deposit plus one matching receipt with amount exactly 1000 CKB: this is the minimum accepted bucket, so the creation tx should pass at the lower boundary.
#[test]
fn deposit_at_minimum_is_accepted() {
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
        .outputs_data(vec![dao_deposit_data(), receipt_data(1, amount)].pack())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("deposit at the minimum boundary should verify");
}

// Build one DAO deposit plus one matching receipt with amount 1,000,001 CKB: this crosses the maximum deposit bucket, so creation must fail because the receipt claims an oversized deposit.
#[test]
fn deposit_above_maximum_is_rejected() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, dao) = ickb_logic_and_dao_scripts(&mut context);
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((1_100_000 * SHANNONS).pack())
            .lock(funding_lock.clone())
            .build(),
        Bytes::new(),
    );

    let amount = 1_000_001 * SHANNONS;
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
        .outputs_data(vec![dao_deposit_data(), receipt_data(1, amount)].pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_DEPOSIT_TOO_BIG);
}

// Build one DAO deposit plus one matching receipt with amount exactly 1,000,000 CKB: this is the maximum accepted bucket, so the creation tx should still verify at the upper boundary.
#[test]
fn deposit_at_maximum_is_accepted() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, dao) = ickb_logic_and_dao_scripts(&mut context);
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((1_100_000 * SHANNONS).pack())
            .lock(funding_lock.clone())
            .build(),
        Bytes::new(),
    );

    let amount = 1_000_000 * SHANNONS;
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
        .outputs_data(vec![dao_deposit_data(), receipt_data(1, amount)].pack())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("deposit at the maximum boundary should verify");
}
