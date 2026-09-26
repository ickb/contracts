use super::*;

// Build one deposit plus one receipt whose quantity field is zero: the tx shape includes a receipt cell, but the receipt claims no deposits, so creation fails on the non-empty-receipt invariant.
#[test]
fn zero_quantity_receipt_is_rejected() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, dao) = ickb_logic_and_dao_scripts(&mut context);
    let funding_input = context.create_cell(
        cell(2_000 * CKB, &funding_lock, None),
        Bytes::new(),
    );

    let amount = 1_000 * CKB;
    let deposit_output = cell(deposit_capacity(&ickb_logic, &dao, 8, amount), &ickb_logic, Some(&dao));
    let receipt_output = cell(occupied_capacity(&funding_lock, &ickb_logic, 12), &funding_lock, Some(&ickb_logic));

    let tx = TransactionBuilder::default()
        .input(input(funding_input))
        .outputs(vec![deposit_output, receipt_output])
        .outputs_data(vec![dao_deposit_data(), receipt_data(0, amount)].pack())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_EMPTY_RECEIPT);
}

// Build one deposit plus one receipt that claims quantity 2 for that deposit bucket: the receipt overstates how many equal deposits were created, so matching fails.
#[test]
fn forged_receipt_quantity_without_enough_deposits_is_rejected() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, dao) = ickb_logic_and_dao_scripts(&mut context);
    let funding_input = context.create_cell(
        cell(2_000 * CKB, &funding_lock, None),
        Bytes::new(),
    );

    let amount = 1_000 * CKB;
    let deposit_output = cell(deposit_capacity(&ickb_logic, &dao, 8, amount), &ickb_logic, Some(&dao));
    let receipt_output = cell(occupied_capacity(&funding_lock, &ickb_logic, 12), &funding_lock, Some(&ickb_logic));

    let tx = TransactionBuilder::default()
        .input(input(funding_input))
        .outputs(vec![deposit_output, receipt_output])
        .outputs_data(vec![dao_deposit_data(), receipt_data(2, amount)].pack())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_RECEIPT_MISMATCH);
}

// Build one 1000-CKB deposit plus one receipt that claims a 1001-CKB bucket: receipt matching is keyed by exact deposit amount, so the mismatched bucket is rejected.
#[test]
fn receipt_for_unmatched_deposit_amount_is_rejected() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, dao) = ickb_logic_and_dao_scripts(&mut context);
    let funding_input = context.create_cell(
        cell(2_000 * CKB, &funding_lock, None),
        Bytes::new(),
    );

    let deposit_output = cell(deposit_capacity(&ickb_logic, &dao, 8, 1_000 * CKB), &ickb_logic, Some(&dao));
    let receipt_output = cell(occupied_capacity(&funding_lock, &ickb_logic, 12), &funding_lock, Some(&ickb_logic));

    let tx = TransactionBuilder::default()
        .input(input(funding_input))
        .outputs(vec![deposit_output, receipt_output])
        .outputs_data(vec![dao_deposit_data(), receipt_data(1, 1_001 * CKB)].pack())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_RECEIPT_MISMATCH);
}
