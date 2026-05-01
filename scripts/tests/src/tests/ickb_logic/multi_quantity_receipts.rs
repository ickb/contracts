use super::*;

// Build a creation tx with two equal DAO deposits and one receipt whose quantity field is 2: this checks that receipt accounting can collapse repeated same-amount buckets, so the tx should pass.
#[test]
fn repeated_deposit_bucket_can_be_matched_by_one_multi_quantity_receipt() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, dao) = ickb_logic_and_dao_scripts(&mut context);
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((3_000 * SHANNONS).pack())
            .lock(funding_lock.clone())
            .build(),
        Bytes::new(),
    );

    let deposit_amount = 1_000 * SHANNONS;
    let deposit_data = dao_deposit_data();
    let deposit_output = || {
        CellOutput::new_builder()
            .capacity(deposit_capacity(&ickb_logic, &dao, deposit_data.len(), deposit_amount).pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build()
    };
    let receipt_output = CellOutput::new_builder()
        .capacity(capacity_for_data(12).pack())
        .lock(funding_lock)
        .type_(Some(ickb_logic.clone()).pack())
        .build();

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .outputs(vec![deposit_output(), deposit_output(), receipt_output])
        .outputs_data(vec![deposit_data.clone(), deposit_data, receipt_data(2, deposit_amount)].pack())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("one receipt with quantity 2 should match two equal-sized deposits");
}

// Build a phase2 conversion that spends one quantity-2 receipt into one xUDT output: the receipt should mint the combined value of both deposits, so verification passes.
#[test]
fn multi_quantity_receipt_can_be_converted_in_phase2() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * SHANNONS;
    let receipt_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(12).pack())
            .lock(funding_lock.clone())
            .type_(Some(ickb_logic.clone()).pack())
            .build(),
        receipt_data(2, deposit_amount),
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
        .output_data(udt_data(u128::from(2 * deposit_amount)).pack())
        .header_dep(receipt_header.clone())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("a valid receipt with quantity 2 should mint the combined iCKB amount in phase2");
}
