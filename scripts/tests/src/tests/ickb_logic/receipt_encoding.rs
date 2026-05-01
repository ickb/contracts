use super::*;

// Build a deposit-creation tx whose receipt output encodes a valid 12-byte receipt plus trailing bytes: creation accounting should ignore the extra suffix, so the deposit and receipt pair still passes.
#[test]
fn receipt_trailing_bytes_do_not_change_creation_accounting() {
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

    let deposit_amount = 1_000 * SHANNONS;
    let deposit_data = dao_deposit_data();
    let deposit_output = CellOutput::new_builder()
        .capacity(deposit_capacity(&ickb_logic, &dao, deposit_data.len(), deposit_amount).pack())
        .lock(ickb_logic.clone())
        .type_(Some(dao).pack())
        .build();
    let receipt_output = CellOutput::new_builder()
        .capacity(capacity_for_data(15).pack())
        .lock(funding_lock)
        .type_(Some(ickb_logic).pack())
        .build();
    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .outputs(vec![deposit_output, receipt_output])
        .outputs_data(
            vec![
                deposit_data,
                receipt_data_with_trailing_bytes(1, deposit_amount, &[0xaa, 0xbb, 0xcc]),
            ]
            .pack(),
        )
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("receipt trailing bytes should not affect accounting");
}

// Build a phase2 mint from a receipt input that has valid receipt fields plus trailing bytes: phase2 parsing should use the canonical prefix only, so conversion still passes.
#[test]
fn receipt_trailing_bytes_do_not_change_phase2_conversion() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * SHANNONS;
    let (receipt_out_point, receipt_header) = create_receipt_input(
        &mut context,
        funding_lock.clone(),
        &ickb_logic,
        receipt_data_with_trailing_bytes(1, deposit_amount, &[0xaa, 0xbb, 0xcc]),
        0,
        GENESIS_AR,
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(receipt_out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(funding_lock)
                .type_(Some(xudt).pack())
                .build(),
        )
        .output_data(udt_data(u128::from(deposit_amount)).pack())
        .header_dep(receipt_header.clone())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("receipt trailing bytes should not affect phase2 conversion");
}

// Build a deposit-creation tx whose receipt output truncates the encoded quantity+amount payload to 9 bytes: the receipt shape is incomplete, so creation fails on encoding.
#[test]
fn truncated_receipt_output_with_small_amount_is_rejected() {
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

    let deposit_amount = 1_000 * SHANNONS;
    let deposit_data = dao_deposit_data();
    let deposit_output = CellOutput::new_builder()
        .capacity(deposit_capacity(&ickb_logic, &dao, deposit_data.len(), deposit_amount).pack())
        .lock(ickb_logic.clone())
        .type_(Some(dao).pack())
        .build();
    let receipt_output = CellOutput::new_builder()
        .capacity(capacity_for_data(9).pack())
        .lock(funding_lock)
        .type_(Some(ickb_logic).pack())
        .build();
    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .outputs(vec![deposit_output, receipt_output])
        .outputs_data(
            vec![dao_deposit_data(), truncated_bytes(receipt_data(1, deposit_amount), 9)].pack(),
        )
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_ENCODING);
}

// Build a phase2 mint from a live receipt input whose stored receipt payload is truncated to 9 bytes: conversion cannot decode the amount bucket, so verification fails on encoding.
#[test]
fn truncated_receipt_input_with_small_amount_is_rejected() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, xudt) = ickb_logic_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * SHANNONS;
    let (receipt_out_point, receipt_header) = create_receipt_input(
        &mut context,
        funding_lock.clone(),
        &ickb_logic,
        truncated_bytes(receipt_data(1, deposit_amount), 9),
        0,
        GENESIS_AR,
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(receipt_out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity(capacity_for_data(16).pack())
                .lock(funding_lock)
                .type_(Some(xudt).pack())
                .build(),
        )
        .output_data(udt_data(u128::from(deposit_amount)).pack())
        .header_dep(receipt_header.clone())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_ENCODING);
}
