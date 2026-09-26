use super::*;

// Run an otherwise valid match while adding an unrelated `limit_order` output lock with non-empty args; that extra output lock executes and poisons the whole tx.
#[test]
fn unrelated_non_empty_args_output_lock_poisons_match() {
    let mut context = Context::default();
    let (funding_lock, limit_order, helper_type) = funding_limit_order_and_helper_type_scripts(&mut context);
    let poisoned_lock = data1_script(&mut context, "limit_order", Bytes::from(vec![1]));

    let input_order = context.create_cell(
        cell(deposit_capacity(&limit_order, &helper_type, 89, 1_500 * CKB), &limit_order, Some(&helper_type)),
        order_data_custom(100, 1, [0u8; 32], 5u32.to_le_bytes(), (0, 0), (1, 1), 4),
    );
    let funding_input = context.create_cell(
        cell(100 * CKB, &funding_lock, None),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(input(input_order))
        .input(input(funding_input))
        .outputs(vec![
            cell(deposit_capacity(&limit_order, &helper_type, 89, 1_520 * CKB), &limit_order, Some(&helper_type)),
            cell(80 * CKB, &funding_lock, None),
            cell(200 * CKB, &poisoned_lock, None),
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
    fail(&context, &tx, ERROR_NOT_EMPTY_ARGS);
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
        cell(deposit_capacity(&limit_order, &helper_type, 89, 1_500 * CKB), &limit_order, Some(&helper_type)),
        order_data_custom(100, 1, [0u8; 32], 5u32.to_le_bytes(), (0, 0), (1, 1), 4),
    );
    let funding_input = context.create_cell(
        cell(400 * CKB, &funding_lock, None),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(input(input_order))
        .input(input(funding_input))
        .outputs(vec![
            cell(deposit_capacity(&limit_order, &helper_type, 89, 1_520 * CKB), &limit_order, Some(&helper_type)),
            cell(80 * CKB, &funding_lock, None),
            cell(deposit_capacity(&poisoned_lock, &helper_type, 89, 300 * CKB), &poisoned_lock, Some(&helper_type)),
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
    fail(&context, &tx, ERROR_NOT_EMPTY_ARGS);
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
        cell(deposit_capacity(&limit_order, &helper_type, 89, 1_500 * CKB), &limit_order, Some(&helper_type)),
        order_data_custom(100, 1, [0u8; 32], 5u32.to_le_bytes(), (0, 0), (1, 1), 4),
    );
    let funding_input = context.create_cell(
        cell(100 * CKB, &funding_lock, None),
        Bytes::new(),
    );
    let poisoned_master_input = context.create_cell(
        cell(200 * CKB, &owner_lock, Some(&poisoned_master_type)),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(input(input_order))
        .input(input(funding_input))
        .input(input(poisoned_master_input))
        .outputs(vec![
            cell(deposit_capacity(&limit_order, &helper_type, 89, 1_520 * CKB), &limit_order, Some(&helper_type)),
            cell(80 * CKB, &funding_lock, None),
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
    fail(&context, &tx, ERROR_NOT_EMPTY_ARGS);
}
