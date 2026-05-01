use super::*;

// Build a phase2 mint whose xUDT output data encodes `u64::MAX + 1`: this exceeds the supported amount ceiling, so verification fails before any mint can escape the xUDT range.
#[test]
fn oversized_output_udt_amount_is_rejected() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((1_000 * SHANNONS).pack())
            .lock(funding_lock.clone())
            .build(),
        Bytes::new(),
    );
    let receipt_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(12).pack())
            .lock(funding_lock.clone())
            .type_(Some(ickb_logic.clone()).pack())
            .build(),
        receipt_data(1, 1_000 * SHANNONS),
    );
    let receipt_header = insert_header_for_cell(&mut context, &receipt_out_point, 0, GENESIS_AR);

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .input(CellInput::new_builder().previous_output(receipt_out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(funding_lock)
                .type_(Some(xudt).pack())
                .build(),
        )
        .outputs_data(vec![udt_data(u128::from(u64::MAX) + 1)].pack())
        .header_dep(receipt_header.clone())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_AMOUNT_UNREASONABLY_BIG);
}

// Build a plain xUDT transfer that keeps the amount exactly at `u64::MAX`: this is the accepted numeric boundary, so the tx should pass unchanged.
#[test]
fn xudt_amount_at_u64_max_boundary_is_allowed() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
    let (_, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(user_lock.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        udt_data(u128::from(u64::MAX)),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .output(
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(user_lock)
                .type_(Some(xudt).pack())
                .build(),
        )
        .output_data(udt_data(u128::from(u64::MAX)).pack())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("u64::MAX should remain a valid xUDT amount boundary");
}
