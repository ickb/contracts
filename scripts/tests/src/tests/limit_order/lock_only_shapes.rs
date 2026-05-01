use super::*;

// Create a lock-only order with non-empty lock args so no output lock executes, then spend it and hit the args check on the input lock path.
#[test]
fn non_empty_args_output_lock_can_be_created_but_not_spent() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let limit_order_non_empty = data1_script(&mut context, "limit_order", Bytes::from(vec![1]));
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
                .capacity(deposit_capacity(&limit_order_non_empty, &helper_type, 73, 1_500 * SHANNONS).pack())
                .lock(limit_order_non_empty.clone())
                .type_(Some(helper_type.clone()).pack())
                .build(),
        )
        .output_data(order_data_mint(0, 1, (1, 1)).pack())
        .build();
    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("non-empty-args limit_order output lock can be created because output locks do not execute");

    let out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_capacity(&limit_order_non_empty, &helper_type, 73, 1_500 * SHANNONS).pack())
            .lock(limit_order_non_empty.clone())
            .type_(Some(helper_type).pack())
            .build(),
        order_data_mint(0, 1, (1, 1)),
    );
    let spend_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity((1_500 * SHANNONS).pack())
                .lock(always_success_lock(&mut context))
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();
    let spend_tx = context.complete_tx(spend_tx);
    let err = context.verify_tx(&spend_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_NOT_EMPTY_ARGS);
}

// Forge a cell that uses `limit_order` as both lock and type so the type script executes immediately and rejects the misuse at creation.
#[test]
fn cell_using_limit_order_as_both_lock_and_type_is_rejected() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let limit_order = limit_order_script(&mut context);
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
                .capacity(200u64.pack())
                .lock(limit_order.clone())
                .type_(Some(limit_order).pack())
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_LIMIT_ORDER_SCRIPT_MISUSE);
}

// Create a lock-only output with undecodable order bytes, then spend it and fail once the input lock parses the malformed payload.
#[test]
fn lock_only_limit_order_with_invalid_data_can_be_created_but_not_spent() {
    assert_lock_only_limit_order_spend_error(Bytes::from(vec![1, 2, 3]), 4);
}

// Create a lock-only output whose forged data selects an unknown action, then spend it and fail when the input lock validates the action tag.
#[test]
fn lock_only_limit_order_invalid_action_can_be_created_but_not_spent() {
    assert_lock_only_limit_order_spend_error(
        order_data_custom(0, 2, [0u8; 32], [0u8; 4], (1, 1), (0, 0), 0),
        ERROR_LIMIT_ORDER_INVALID_ACTION,
    );
}

// Create a lock-only output with non-zero padding that no output lock checks, then spend it and fail when the input lock enforces zero padding.
#[test]
fn lock_only_limit_order_nonzero_padding_can_be_created_but_not_spent() {
    assert_lock_only_limit_order_spend_error(
        order_data_custom(0, 0, [1u8; 32], 5i32.to_le_bytes(), (1, 1), (0, 0), 0),
        ERROR_LIMIT_ORDER_NON_ZERO_PADDING,
    );
}

// Create a lock-only output with an invalid ratio encoding, then spend it and fail when the input lock evaluates the ratio fields.
#[test]
fn lock_only_limit_order_invalid_ratio_can_be_created_but_not_spent() {
    assert_lock_only_limit_order_spend_error(
        order_data_custom(0, 0, [0u8; 32], 5i32.to_le_bytes(), (1, 0), (0, 0), 0),
        ERROR_LIMIT_ORDER_INVALID_RATIO,
    );
}

// Create a lock-only output with `ckb_min_match_log` above the allowed range, then spend it and fail when the input lock validates the bound.
#[test]
fn lock_only_limit_order_invalid_ckb_min_match_log_can_be_created_but_not_spent() {
    assert_lock_only_limit_order_spend_error(
        order_data_custom(0, 0, [0u8; 32], 5i32.to_le_bytes(), (1, 1), (0, 0), 65),
        ERROR_LIMIT_ORDER_INVALID_CKB_MIN_MATCH_LOG,
    );
}

// Create a lock-only output whose ratios become concave, then spend it and fail when the input lock checks the curve ordering.
#[test]
fn lock_only_limit_order_concave_ratio_can_be_created_but_not_spent() {
    assert_lock_only_limit_order_spend_error(
        order_data_custom(0, 0, [0u8; 32], 5i32.to_le_bytes(), (1, 10), (1, 1), 0),
        ERROR_LIMIT_ORDER_CONCAVE_RATIO,
    );
}

// Create a lock-only output with both ratios null so no output lock runs, then spend it and fail when the input lock rejects the empty price definition.
#[test]
fn lock_only_limit_order_with_both_ratios_null_can_be_created_but_not_spent() {
    assert_lock_only_limit_order_spend_error(
        order_data_custom(0, 0, [0u8; 32], 5i32.to_le_bytes(), (0, 0), (0, 0), 0),
        ERROR_LIMIT_ORDER_BOTH_RATIOS_NULL,
    );
}

// Forge a lock-only order with no UDT type and just-enough occupied capacity; creation skips the lock, but spending reaches the input path and traps generically.
#[test]
fn lock_only_limit_order_missing_udt_type_hits_generic_failure_even_at_valid_capacity() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let limit_order = limit_order_script(&mut context);
    let forged_output = CellOutput::new_builder().lock(limit_order.clone()).build();
    let forged_capacity = forged_output
        .occupied_capacity(
            ckb_testtool::ckb_types::core::Capacity::bytes(73)
                .expect("occupied capacity bytes"),
        )
        .expect("occupied capacity")
        .as_u64();
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((200 * SHANNONS).pack())
            .lock(funding_lock)
            .build(),
        Bytes::new(),
    );

    let create_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .output(
            forged_output
                .clone()
                .as_builder()
                .capacity(forged_capacity.pack())
                .build(),
        )
        .output_data(order_data_custom(0, 0, [0u8; 32], 5i32.to_le_bytes(), (1, 1), (0, 0), 0).pack())
        .build();
    let create_tx = context.complete_tx(create_tx);
    context
        .verify_tx(&create_tx, MAX_CYCLES)
        .expect("lock-only limit_order output without a UDT type can still be created");

    let forged_out_point = context.create_cell(
        forged_output.as_builder().capacity(forged_capacity.pack()).build(),
        order_data_custom(0, 0, [0u8; 32], 5i32.to_le_bytes(), (1, 1), (0, 0), 0),
    );
    let spend_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(forged_out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity((200 * SHANNONS).pack())
                .lock(always_success_lock(&mut context))
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();

    let spend_tx = context.complete_tx(spend_tx);
    let err = context.verify_tx(&spend_tx, MAX_CYCLES).unwrap_err();
    // The committed release binary traps with a generic -1 here instead of surfacing a stable
    // MissingUdtType code, even when the forged order uses an occupied-capacity-valid layout.
    assert_script_error(err, ERROR_SCRIPT_PANIC);
}

// Create a lock-only output with valid mint data plus trailing bytes, then spend it and fail on the input-side length check.
#[test]
fn lock_only_limit_order_with_trailing_bytes_can_be_created_but_hits_length_check_on_spend() {
    let mut data = order_data_mint(0, 1, (1, 1)).to_vec();
    data.push(0xaa);
    assert_lock_only_limit_order_spend_error(Bytes::from(data), 3);
}
