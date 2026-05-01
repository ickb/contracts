use super::*;

// Build a plain funding tx that tries to mint xUDT by supplying the xUDT owner-script witness in `output_type`: no live iCKB owner-mode path exists in the cell set, so the fallback witness does not authorize minting and verification fails.
#[test]
fn xudt_owner_script_output_witness_cannot_mint_without_live_owner_mode() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((500 * SHANNONS).pack())
            .lock(funding_lock.clone())
            .build(),
        Bytes::new(),
    );

    let witness = witness_with_output_type(xudt_owner_script_witness(ickb_logic));
    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .output(
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(funding_lock)
                .type_(Some(xudt).pack())
                .build(),
        )
        .output_data(udt_data(1).pack())
        .witness(witness.pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_XUDT_AMOUNT);
}

// Build a plain xUDT self-transfer that increases the amount while supplying the xUDT owner-script witness in `input_type`: without a live iCKB owner-mode route, the fallback witness still cannot authorize minting, so verification fails.
#[test]
fn xudt_owner_script_input_witness_cannot_mint_without_live_owner_mode() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(user_lock.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        udt_data(1),
    );

    let witness = witness_with_input_type(xudt_owner_script_witness(ickb_logic));
    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .output(
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(user_lock)
                .type_(Some(xudt).pack())
                .build(),
        )
        .output_data(udt_data(2).pack())
        .witness(witness.pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_XUDT_AMOUNT);
}
