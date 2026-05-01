use super::*;

// Continue a UDT->CKB order with extra funding so the match path preserves value and clears the minimum fill threshold.
#[test]
fn udt_to_ckb_match_passes() {
    let mut context = Context::default();
    let (funding_lock, limit_order, helper_type) = funding_limit_order_and_helper_type_scripts(&mut context);

    let input_order = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_capacity(&limit_order, &helper_type, 73, 1_500 * SHANNONS).pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
        order_data_custom(100, 1, [0u8; 32], 5u32.to_le_bytes(), (0, 0), (1, 1), 4),
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
                order_data_custom(80, 1, [0u8; 32], 5u32.to_le_bytes(), (0, 0), (1, 1), 4),
                Bytes::new(),
            ]
            .pack(),
        )
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("udt-to-ckb partial matches should verify when value and minimum match hold");
}

// Continue the same UDT->CKB shape right at the minimum fill boundary; the match path should still accept the exact-threshold step.
#[test]
fn udt_to_ckb_match_accepts_exact_minimum_partial_fill() {
    let mut context = Context::default();
    let (funding_lock, limit_order, helper_type) = funding_limit_order_and_helper_type_scripts(&mut context);

    let input_order = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_capacity(&limit_order, &helper_type, 73, 1_500 * SHANNONS).pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
        order_data_custom(100, 1, [0u8; 32], 5u32.to_le_bytes(), (0, 0), (1, 1), 4),
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
                .capacity(deposit_capacity(&limit_order, &helper_type, 73, 1_516 * SHANNONS).pack())
                .lock(limit_order)
                .type_(Some(helper_type).pack())
                .build(),
            CellOutput::new_builder()
                .capacity((84 * SHANNONS).pack())
                .lock(funding_lock)
                .build(),
        ])
        .outputs_data(
            vec![
                order_data_custom(84, 1, [0u8; 32], 5u32.to_le_bytes(), (0, 0), (1, 1), 4),
                Bytes::new(),
            ]
            .pack(),
        )
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("udt-to-ckb partial matches should verify when the fill lands exactly on the minimum boundary");
}

// Continue one valid match while also creating an unrelated typed output; the match path should ignore the foreign lock/type pair and still pass.
#[test]
fn valid_match_ignores_foreign_typed_output() {
    let mut context = Context::default();
    let (funding_lock, limit_order, helper_type) = funding_limit_order_and_helper_type_scripts(&mut context);
    let foreign_lock = named_always_success_lock(&mut context, b"foreign-lock");
    let foreign_type = named_always_success_lock(&mut context, b"foreign-type");

    let input_order = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_capacity(&limit_order, &helper_type, 73, 1_500 * SHANNONS).pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
        order_data_custom(100, 1, [0u8; 32], 5u32.to_le_bytes(), (0, 0), (1, 1), 4),
    );
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((300 * SHANNONS).pack())
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
            CellOutput::new_builder()
                .capacity((200 * SHANNONS).pack())
                .lock(foreign_lock)
                .type_(Some(foreign_type).pack())
                .build(),
        ])
        .outputs_data(
            vec![
                order_data_custom(80, 1, [0u8; 32], 5u32.to_le_bytes(), (0, 0), (1, 1), 4),
                Bytes::new(),
                Bytes::new(),
            ]
            .pack(),
        )
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("limit_order should ignore unrelated foreign typed outputs during a valid match");
}

// Continue a CKB->UDT order with both ratios populated; the match path accepts when value moves in the allowed direction.
#[test]
fn ckb_to_udt_match_passes_with_both_ratios_present() {
    let mut context = Context::default();
    let (funding_lock, limit_order, helper_type) = funding_limit_order_and_helper_type_scripts(&mut context);

    let input_order = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_capacity(&limit_order, &helper_type, 73, 1_500 * SHANNONS).pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
        order_data_custom(0, 1, [0u8; 32], 5u32.to_le_bytes(), (1, 1), (1, 1), 4),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(input_order).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(deposit_capacity(&limit_order, &helper_type, 73, 1_480 * SHANNONS).pack())
                .lock(limit_order)
                .type_(Some(helper_type).pack())
                .build(),
            CellOutput::new_builder()
                .capacity((20 * SHANNONS).pack())
                .lock(funding_lock)
                .build(),
        ])
        .outputs_data(
            vec![
                order_data_custom(
                    u128::from(20 * SHANNONS),
                    1,
                    [0u8; 32],
                    5u32.to_le_bytes(),
                    (1, 1),
                    (1, 1),
                    4,
                ),
                Bytes::new(),
            ]
            .pack(),
        )
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("ckb-to-udt partial matches should verify when both ratios are present and value is preserved");
}

// Continue a CKB->UDT order by exactly the advertised minimum fill; the match path accepts the boundary case.
#[test]
fn ckb_to_udt_match_accepts_exact_minimum_partial_fill() {
    let mut context = Context::default();
    let (limit_order, helper_type) = limit_order_and_helper_type_scripts(&mut context);

    let input_order_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_capacity(&limit_order, &helper_type, 73, 1_500 * SHANNONS).pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
        order_data_custom(0, 1, [0u8; 32], 5u32.to_le_bytes(), (1, 1), (0, 0), 4),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(input_order_out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity(deposit_capacity(&limit_order, &helper_type, 73, 1_500 * SHANNONS - 16).pack())
                .lock(limit_order)
                .type_(Some(helper_type).pack())
                .build(),
        )
        .output_data(order_data_custom(16, 1, [0u8; 32], 5u32.to_le_bytes(), (1, 1), (0, 0), 4).pack())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("ckb-to-udt partial matches should verify when the fill lands exactly on the minimum boundary");
}

// Continue a UDT->CKB order with too little UDT progress for its declared minimum; the match path rejects the underfilled step.
#[test]
fn udt_to_ckb_match_rejects_small_udt_delta_under_minimum() {
    let mut context = Context::default();
    let (funding_lock, limit_order, helper_type) = funding_limit_order_and_helper_type_scripts(&mut context);

    let input_order = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_capacity(&limit_order, &helper_type, 73, 1_500 * SHANNONS).pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
        order_data_custom(100, 1, [0u8; 32], 5u32.to_le_bytes(), (0, 0), (1, 1), 8),
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
                order_data_custom(99, 1, [0u8; 32], 5u32.to_le_bytes(), (0, 0), (1, 1), 8),
                Bytes::new(),
            ]
            .pack(),
        )
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_LIMIT_ORDER_INSUFFICIENT_MATCH);
}

// Continue a UDT->CKB order whose `ckb_min_match_log` demands a larger fill than the delta shown; the match path rejects the insufficient continuation.
#[test]
fn udt_to_ckb_match_rejects_large_minimum_partial_fill() {
    let mut context = Context::default();
    let (funding_lock, limit_order, helper_type) = funding_limit_order_and_helper_type_scripts(&mut context);

    let input_order = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_capacity(&limit_order, &helper_type, 73, 1_500 * SHANNONS).pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
        order_data_custom(100, 1, [0u8; 32], 5u32.to_le_bytes(), (0, 0), (1, 1), 40),
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
                order_data_custom(99, 1, [0u8; 32], 5u32.to_le_bytes(), (0, 0), (1, 1), 40),
                Bytes::new(),
            ]
            .pack(),
        )
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_LIMIT_ORDER_INSUFFICIENT_MATCH);
}

// Continue a match-shaped order while reversing progress in the wrong direction; the match path rejects the inconsistent state transition.
#[test]
fn match_rejects_invalid_direction_change() {
    let mut context = Context::default();
    let (limit_order, helper_type) = limit_order_and_helper_type_scripts(&mut context);
    let master = OutPoint::new(Byte32::zero(), 5);

    let input_order_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_capacity(&limit_order, &helper_type, 73, 1_500 * SHANNONS).pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
        order_data_match(100 * SHANNONS as u128, &master, (1, 1)),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(input_order_out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity(deposit_capacity(&limit_order, &helper_type, 73, 1_400 * SHANNONS).pack())
                .lock(limit_order)
                .type_(Some(helper_type).pack())
                .build(),
        )
        .output_data(order_data_match(50 * SHANNONS as u128, &master, (1, 1)).pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_LIMIT_ORDER_INVALID_MATCH);
}

// Re-emit a match-shaped order without changing capacity or matched amount; the match path rejects the no-op continuation.
#[test]
fn match_rejects_unchanged_shape() {
    let mut context = Context::default();
    let (limit_order, helper_type) = limit_order_and_helper_type_scripts(&mut context);
    let master = OutPoint::new(Byte32::zero(), 5);

    let input_order_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_capacity(&limit_order, &helper_type, 73, 1_500 * SHANNONS).pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
        order_data_match(100 * SHANNONS as u128, &master, (1, 1)),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(input_order_out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity(deposit_capacity(&limit_order, &helper_type, 73, 1_500 * SHANNONS).pack())
                .lock(limit_order)
                .type_(Some(helper_type).pack())
                .build(),
        )
        .output_data(order_data_match(100 * SHANNONS as u128, &master, (1, 1)).pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_LIMIT_ORDER_INVALID_MATCH);
}

// Continue a match-shaped order while reducing total order value; the match path catches the decreasing-value transition.
#[test]
fn match_rejects_decreasing_value() {
    let mut context = Context::default();
    let (limit_order, helper_type) = limit_order_and_helper_type_scripts(&mut context);
    let master = OutPoint::new(Byte32::zero(), 5);

    let input_order_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_capacity(&limit_order, &helper_type, 73, 1_500 * SHANNONS).pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
        order_data_match(0, &master, (1, 1)),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(input_order_out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity(deposit_capacity(&limit_order, &helper_type, 73, 1_400 * SHANNONS).pack())
                .lock(limit_order)
                .type_(Some(helper_type).pack())
                .build(),
        )
        .output_data(order_data_match(50 * SHANNONS as u128, &master, (1, 1)).pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_LIMIT_ORDER_DECREASING_VALUE);
}

// Continue a CKB->UDT order by less than its minimum partial fill; the match path rejects the too-small delta.
#[test]
fn match_rejects_too_small_partial_fill() {
    let mut context = Context::default();
    let (limit_order, helper_type) = limit_order_and_helper_type_scripts(&mut context);

    let input_order_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_capacity(&limit_order, &helper_type, 73, 1_500 * SHANNONS).pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
        order_data_custom(0, 1, [0u8; 32], 5u32.to_le_bytes(), (1, 1), (0, 0), 4),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(input_order_out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity(deposit_capacity(&limit_order, &helper_type, 73, 1_500 * SHANNONS - 8).pack())
                .lock(limit_order)
                .type_(Some(helper_type).pack())
                .build(),
        )
        .output_data(order_data_custom(8, 1, [0u8; 32], 5u32.to_le_bytes(), (1, 1), (0, 0), 4).pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_LIMIT_ORDER_INSUFFICIENT_MATCH);
}

// Continue an almost-empty CKB->UDT order down to zero unoccupied value; the input lock traps generically instead of surfacing a stable typed error.
#[test]
fn zero_unoccupied_match_shape_hits_generic_failure() {
    let mut context = Context::default();
    let (limit_order, helper_type) = limit_order_and_helper_type_scripts(&mut context);

    let input_order_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_capacity(&limit_order, &helper_type, 73, 8).pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
        order_data_custom(0, 1, [0u8; 32], 5u32.to_le_bytes(), (1, 1), (0, 0), 4),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(input_order_out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity(deposit_capacity(&limit_order, &helper_type, 73, 0).pack())
                .lock(limit_order)
                .type_(Some(helper_type).pack())
                .build(),
        )
        .output_data(order_data_custom(8, 1, [0u8; 32], 5u32.to_le_bytes(), (1, 1), (0, 0), 4).pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    // The deployed release binary traps with a generic -1 here instead of surfacing a stable
    // typed error for this zero-unoccupied transition shape.
    assert_script_error(err, ERROR_SCRIPT_PANIC);
}
