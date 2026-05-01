use super::*;

// Mint a fresh pair at the `ckb_min_match_log` encoding boundary, with only output-side validation running, and expect the shape to pass.
#[test]
fn mint_accepts_ckb_min_match_log_64() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let limit_order = limit_order_script(&mut context);
    let helper_type = helper_type_script(&mut context);
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((2_000 * SHANNONS).pack())
            .lock(funding_lock)
            .build(),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .output(
            CellOutput::new_builder()
                .capacity((1_500 * SHANNONS).pack())
                .lock(limit_order.clone())
                .type_(Some(helper_type).pack())
                .build(),
        )
        .output(
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(owner_lock)
                .type_(Some(limit_order).pack())
                .build(),
        )
        .outputs_data(
            vec![
                order_data_custom(0, 0, [0u8; 32], 1i32.to_le_bytes(), (1, 1), (0, 0), 64),
                Bytes::new(),
            ]
            .pack(),
        )
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("ckb_min_match_log == 64 should be accepted at the encoding boundary");
}

// Mint the same fresh pair with `ckb_min_match_log` just above the boundary; the creation path executes validation and rejects the shape.
#[test]
fn mint_rejects_ckb_min_match_log_65() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let limit_order = limit_order_script(&mut context);
    let helper_type = helper_type_script(&mut context);
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((2_000 * SHANNONS).pack())
            .lock(funding_lock)
            .build(),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .output(
            CellOutput::new_builder()
                .capacity((1_500 * SHANNONS).pack())
                .lock(limit_order.clone())
                .type_(Some(helper_type).pack())
                .build(),
        )
        .output(
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(owner_lock)
                .type_(Some(limit_order).pack())
                .build(),
        )
        .outputs_data(
            vec![
                order_data_custom(0, 0, [0u8; 32], 1i32.to_le_bytes(), (1, 1), (0, 0), 65),
                Bytes::new(),
            ]
            .pack(),
        )
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_LIMIT_ORDER_INVALID_CKB_MIN_MATCH_LOG);
}

// Create two would-be master outputs and no order output; the creation path fails as invalid configuration before any duplicate-master-specific conclusion applies.
#[test]
fn two_master_outputs_fail_as_invalid_configuration_not_duplicate_master() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let limit_order = limit_order_script(&mut context);
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((1_000 * SHANNONS).pack())
            .lock(funding_lock)
            .build(),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(owner_lock.clone())
                .type_(Some(limit_order.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(owner_lock)
                .type_(Some(limit_order).pack())
                .build(),
        ])
        .outputs_data(vec![Bytes::new(), Bytes::new()].pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);
}
