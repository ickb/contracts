use super::*;

// Run an otherwise valid match while adding an unrelated `limit_order` output lock with non-empty args; that extra output lock executes and poisons the whole tx.
#[test]
fn unrelated_non_empty_args_output_lock_poisons_match() {
    let mut context = Context::default();
    let (funding_lock, limit_order, helper_type) = funding_limit_order_and_helper_type_scripts(&mut context);
    let poisoned_lock = data1_script(&mut context, "limit_order", Bytes::from(vec![1]));

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
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(poisoned_lock)
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
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_NOT_EMPTY_ARGS);
}

// Run the same valid match while also creating a plausible order-shaped output under non-empty-args `limit_order`; the extra output lock still executes and aborts the tx.
#[test]
fn order_shaped_non_empty_args_output_poisons_match() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let limit_order = limit_order_script(&mut context);
    let poisoned_lock = data1_script(&mut context, "limit_order", Bytes::from(vec![1]));
    let helper_type = helper_type_script(&mut context);

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
            .capacity((400 * SHANNONS).pack())
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
                .type_(Some(helper_type.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity((80 * SHANNONS).pack())
                .lock(funding_lock)
                .build(),
            CellOutput::new_builder()
                .capacity(deposit_capacity(&poisoned_lock, &helper_type, 73, 300 * SHANNONS).pack())
                .lock(poisoned_lock)
                .type_(Some(helper_type).pack())
                .build(),
        ])
        .outputs_data(
            vec![
                order_data_custom(80, 1, [0u8; 32], 5u32.to_le_bytes(), (0, 0), (1, 1), 4),
                Bytes::new(),
                order_data_mint(0, 1, (1, 1)),
            ]
            .pack(),
        )
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_NOT_EMPTY_ARGS);
}

// Run a valid match but include an unrelated master sibling whose type is non-empty-args `limit_order`; that extra input-side script executes and poisons verification.
#[test]
fn non_empty_args_master_sibling_poisons_match() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let limit_order = limit_order_script(&mut context);
    let poisoned_master_type = data1_script(&mut context, "limit_order", Bytes::from(vec![1]));
    let helper_type = helper_type_script(&mut context);

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
    let poisoned_master_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(owner_lock)
            .type_(Some(poisoned_master_type).pack())
            .build(),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(input_order).build())
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .input(CellInput::new_builder().previous_output(poisoned_master_input).build())
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
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_NOT_EMPTY_ARGS);
}
