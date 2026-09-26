use super::*;

// Build a phase2 receipt-to-xUDT conversion without including the receipt's header in `header_deps`: the script needs that header to price the receipt, so verification fails with the missing-item path.
#[test]
fn phase2_conversion_without_receipt_header_dep_is_rejected() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * CKB;
    let receipt_out_point = create_receipt(&mut context, &funding_lock, &ickb_logic, 1, deposit_amount);
    insert_header_for_cell(&mut context, &receipt_out_point, 0, GENESIS_AR);

    let tx = TransactionBuilder::default()
        .input(input(receipt_out_point))
        .output(cell(occupied_capacity(&funding_lock, &xudt, 16), &funding_lock, Some(&xudt)))
        .output_data(udt_data(u128::from(deposit_amount)).pack())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_ITEM_MISSING);
}

// Build one phase2 mint from two receipts that point at two different headers and supply both headers: multi-header conversion is allowed when every receipt has its own dependency, so verification passes.
#[test]
fn phase2_conversion_with_two_receipts_from_distinct_headers_passes() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let first_amount = 1_000 * CKB;
    let second_amount = 1_200 * CKB;
    let first_receipt = create_receipt(&mut context, &funding_lock, &ickb_logic, 1, first_amount);
    let second_receipt = create_receipt(&mut context, &funding_lock, &ickb_logic, 1, second_amount);
    let first_header = gen_header(0, GENESIS_AR as u64, 0, 0, 1);
    let second_header = gen_header(1, SYNTHETIC_WITHDRAW_AR, 1, 1, 1);
    link_cell_to_header(&mut context, &first_receipt, &first_header);
    link_cell_to_header(&mut context, &second_receipt, &second_header);

    let expected = u128::from(first_amount)
        + (u128::from(second_amount) * u128::from(GENESIS_AR) / u128::from(SYNTHETIC_WITHDRAW_AR));
    let tx = TransactionBuilder::default()
        .input(input(first_receipt))
        .input(input(second_receipt))
        .output(cell(occupied_capacity(&funding_lock, &xudt, 16), &funding_lock, Some(&xudt)))
        .output_data(udt_data(expected).pack())
        .header_dep(first_header.hash())
        .header_dep(second_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("phase2 conversion should support multiple receipts from distinct headers");
}

// Build the same two-receipt phase2 shape but omit one of the two required receipt headers: the transaction cannot price every input receipt, so verification fails.
#[test]
fn phase2_conversion_with_one_missing_receipt_header_dep_is_rejected() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let first_amount = 1_000 * CKB;
    let second_amount = 1_200 * CKB;
    let first_receipt = create_receipt(&mut context, &funding_lock, &ickb_logic, 1, first_amount);
    let second_receipt = create_receipt(&mut context, &funding_lock, &ickb_logic, 1, second_amount);
    let first_header = gen_header(0, GENESIS_AR as u64, 0, 0, 1);
    let second_header = gen_header(1, SYNTHETIC_WITHDRAW_AR, 1, 1, 1);
    link_cell_to_header(&mut context, &first_receipt, &first_header);
    link_cell_to_header(&mut context, &second_receipt, &second_header);

    let tx = TransactionBuilder::default()
        .input(input(first_receipt))
        .input(input(second_receipt))
        .output(cell(occupied_capacity(&funding_lock, &xudt, 16), &funding_lock, Some(&xudt)))
        .output_data(udt_data(u128::from(first_amount + second_amount)).pack())
        .header_dep(first_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_ITEM_MISSING);
}

// Build a phase2 conversion whose receipt header has accumulated rate zero: this is a malformed pricing header, so the conversion path panics instead of minting from an invalid rate.
#[test]
fn phase2_conversion_with_zero_accumulated_rate_header_is_rejected() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * CKB;
    let receipt_out_point = create_receipt(&mut context, &funding_lock, &ickb_logic, 1, deposit_amount);
    let malformed_header = gen_header(0, 0, 0, 0, 1);
    link_cell_to_header(&mut context, &receipt_out_point, &malformed_header);

    let tx = TransactionBuilder::default()
        .input(input(receipt_out_point))
        .output(cell(occupied_capacity(&funding_lock, &xudt, 16), &funding_lock, Some(&xudt)))
        .output_data(udt_data(u128::from(deposit_amount)).pack())
        .header_dep(malformed_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_SCRIPT_PANIC);
}
