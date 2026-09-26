use super::*;

// Build a creation tx with two equal DAO deposits and one receipt whose quantity field is 2: this checks that receipt accounting can collapse repeated same-amount buckets, so the tx should pass.
#[test]
fn repeated_deposit_bucket_can_be_matched_by_one_multi_quantity_receipt() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, dao) = ickb_logic_and_dao_scripts(&mut context);
    let funding_input = context.create_cell(
        cell(3_000 * CKB, &funding_lock, None),
        Bytes::new(),
    );

    let deposit_amount = 1_000 * CKB;
    let deposit_data = dao_deposit_data();
    let deposit_output = || {
        cell(deposit_capacity(&ickb_logic, &dao, deposit_data.len(), deposit_amount), &ickb_logic, Some(&dao))
    };
    let receipt_output = cell(occupied_capacity(&funding_lock, &ickb_logic, 12), &funding_lock, Some(&ickb_logic));

    let tx = TransactionBuilder::default()
        .input(input(funding_input))
        .outputs(vec![deposit_output(), deposit_output(), receipt_output])
        .outputs_data(vec![deposit_data.clone(), deposit_data, receipt_data(2, deposit_amount)].pack())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("one receipt with quantity 2 should match two equal-sized deposits");
}

// Build a phase2 conversion that spends one quantity-2 receipt into one xUDT output: the receipt should mint the combined value of both deposits, so verification passes.
#[test]
fn multi_quantity_receipt_can_be_converted_in_phase2() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * CKB;
    let receipt_out_point = create_receipt(&mut context, &funding_lock, &ickb_logic, 2, deposit_amount);
    let receipt_header = insert_header_for_cell(&mut context, &receipt_out_point, 0, GENESIS_AR);

    let tx = TransactionBuilder::default()
        .input(input(receipt_out_point))
        .output(cell(occupied_capacity(&funding_lock, &xudt, 16), &funding_lock, Some(&xudt)))
        .output_data(udt_data(u128::from(2 * deposit_amount)).pack())
        .header_dep(receipt_header.clone())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("a valid receipt with quantity 2 should mint the combined iCKB amount in phase2");
}
