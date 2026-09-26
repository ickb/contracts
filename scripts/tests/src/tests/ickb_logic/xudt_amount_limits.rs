use super::*;

// Build a phase2 mint whose xUDT output data encodes `u64::MAX + 1`: this exceeds the supported amount ceiling, so verification fails before any mint can escape the xUDT range.
#[test]
fn oversized_output_udt_amount_is_rejected() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let funding_input = context.create_cell(
        cell(1_000 * CKB, &funding_lock, None),
        Bytes::new(),
    );
    let receipt_out_point = create_receipt(&mut context, &funding_lock, &ickb_logic, 1, 1_000 * CKB);
    let receipt_header = insert_header_for_cell(&mut context, &receipt_out_point, 0, GENESIS_AR);

    let tx = TransactionBuilder::default()
        .input(input(funding_input))
        .input(input(receipt_out_point))
        .output(cell(occupied_capacity(&funding_lock, &xudt, 16), &funding_lock, Some(&xudt)))
        .outputs_data(vec![udt_data(u128::from(u64::MAX) + 1)].pack())
        .header_dep(receipt_header.clone())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_AMOUNT_UNREASONABLY_BIG);
}

// Build a plain xUDT transfer that keeps the amount exactly at `u64::MAX`: this is the accepted numeric boundary, so the tx should pass unchanged.
#[test]
fn xudt_amount_at_u64_max_boundary_is_allowed() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
    let (_, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let udt_input = create_udt(&mut context, &user_lock, &xudt, u128::from(u64::MAX));

    let tx = TransactionBuilder::default()
        .input(input(udt_input))
        .output(cell(occupied_capacity(&user_lock, &xudt, 16), &user_lock, Some(&xudt)))
        .output_data(udt_data(u128::from(u64::MAX)).pack())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("u64::MAX should remain a valid xUDT amount boundary");
}
