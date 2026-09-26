use super::*;

// Build a plain funding tx that tries to mint xUDT by supplying the xUDT owner-script witness in `output_type`: no live iCKB owner-mode path exists in the cell set, so the fallback witness does not authorize minting and verification fails.
#[test]
fn xudt_owner_script_output_witness_cannot_mint_without_live_owner_mode() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let funding_input = context.create_cell(
        cell(500 * CKB, &funding_lock, None),
        Bytes::new(),
    );

    let witness = witness_with_output_type(xudt_owner_script_witness(ickb_logic));
    let tx = TransactionBuilder::default()
        .input(input(funding_input))
        .output(cell(occupied_capacity(&funding_lock, &xudt, 16), &funding_lock, Some(&xudt)))
        .output_data(udt_data(1).pack())
        .witness(witness.pack())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_XUDT_AMOUNT);
}

// Build a plain xUDT self-transfer that increases the amount while supplying the xUDT owner-script witness in `input_type`: without a live iCKB owner-mode route, the fallback witness still cannot authorize minting, so verification fails.
#[test]
fn xudt_owner_script_input_witness_cannot_mint_without_live_owner_mode() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let udt_input = create_udt(&mut context, &user_lock, &xudt, 1);

    let witness = witness_with_input_type(xudt_owner_script_witness(ickb_logic));
    let tx = TransactionBuilder::default()
        .input(input(udt_input))
        .output(cell(occupied_capacity(&user_lock, &xudt, 16), &user_lock, Some(&xudt)))
        .output_data(udt_data(2).pack())
        .witness(witness.pack())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_XUDT_AMOUNT);
}
