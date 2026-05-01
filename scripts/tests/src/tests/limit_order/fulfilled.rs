use super::*;

// Continue a fulfilled CKB->UDT-shaped match cell as if matching again; the lock path traps before a typed fulfilled-order error is surfaced.
#[test]
fn fulfilled_ckb_to_udt_shape_cannot_reopen_as_match() {
    let mut context = Context::default();
    let (limit_order, helper_type) = limit_order_and_helper_type_scripts(&mut context);
    let master = OutPoint::new(Byte32::zero(), 5);

    let input_order = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_capacity(&limit_order, &helper_type, 73, 0).pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
        order_data_match(0, &master, (1, 1)),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(input_order).build())
        .output(
            CellOutput::new_builder()
                .capacity(deposit_capacity(&limit_order, &helper_type, 73, 0).pack())
                .lock(limit_order)
                .type_(Some(helper_type).pack())
                .build(),
        )
        .output_data(order_data_match(1, &master, (1, 1)).pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    // The deployed release binary traps with a generic -1 here instead of surfacing the
    // internal AttemptToChangeFulfilled branch as a typed script error.
    assert_script_error(err, ERROR_SCRIPT_PANIC);
}

// Forge a fulfilled UDT->CKB-shaped cell and try to continue it; this shape never reaches the inner fulfilled guard and instead fails the outer match validation.
#[test]
fn fulfilled_udt_to_ckb_shape_cannot_reach_guard_and_fails_as_invalid_match() {
    let mut context = Context::default();
    let (funding_lock, limit_order, helper_type) = funding_limit_order_and_helper_type_scripts(&mut context);

    let input_order = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_capacity(&limit_order, &helper_type, 73, 1_500 * SHANNONS).pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
        order_data_custom(0, 1, [0u8; 32], 5u32.to_le_bytes(), (0, 0), (1, 1), 4),
    );
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((100 * SHANNONS).pack())
            .lock(funding_lock.clone())
            .build(),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(input_order).build())
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(deposit_capacity(&limit_order, &helper_type, 73, 1_520 * SHANNONS).pack())
                .lock(limit_order)
                .type_(Some(helper_type).pack())
                .build(),
            CellOutput::new_builder()
                .capacity((80 * SHANNONS).pack())
                .lock(funding_lock)
                .build(),
        ])
        .outputs_data(
            vec![
                // The contract's fulfilled-order guard for UDT -> CKB sits behind `i.udt > o.udt`,
                // so a zero-UDT input can only be observed failing the outer match-shape check.
                order_data_custom(0, 1, [0u8; 32], 5u32.to_le_bytes(), (0, 0), (1, 1), 4),
                Bytes::new(),
            ]
            .pack(),
        )
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_LIMIT_ORDER_INVALID_MATCH);
}
