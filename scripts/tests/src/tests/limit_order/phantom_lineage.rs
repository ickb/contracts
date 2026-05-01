use super::*;

// Create a mint-shaped output lock with no real master cell behind its distance; creation passes because no master lock executes on output.
#[test]
fn phantom_mint_output_can_be_created() {
    let mut context = Context::default();
    let (funding_lock, limit_order, helper_type) = funding_limit_order_and_helper_type_scripts(&mut context);

    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((2_000 * SHANNONS).pack())
            .lock(funding_lock.clone())
            .build(),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .output(
            CellOutput::new_builder()
                .capacity((1_500 * SHANNONS).pack())
                .lock(limit_order)
                .type_(Some(helper_type).pack())
                .build(),
        )
        .output_data(order_data_mint(0, 5, (1, 1)).pack())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("phantom order creation should bypass limit_order validation");
}

// Continue that phantom mint into match state using the derived metapoint only; the match path accepts even though no real master input exists.
#[test]
fn phantom_mint_lineage_can_enter_match_without_real_master() {
    let mut context = Context::default();
    let (limit_order, helper_type) = limit_order_and_helper_type_scripts(&mut context);

    let phantom_order_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity((1_500 * SHANNONS).pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
        order_data_mint(0, 5, (1, 1)),
    );
    let phantom_master_out_point = OutPoint::new(phantom_order_out_point.tx_hash(), 5);

    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(phantom_order_out_point)
                .build(),
        )
        .output(
            CellOutput::new_builder()
                .capacity((1_400 * SHANNONS).pack())
                .lock(limit_order)
                .type_(Some(helper_type).pack())
                .build(),
        )
        .output_data(order_data_match(100 * SHANNONS as u128, &phantom_master_out_point, (1, 1)).pack())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("a phantom mint-shaped order should be able to transition into match state without any real master");
}

// Continue a phantom mint into match state but rewrite its metapoint to an unrelated fake master; the match path rejects the lineage rebind.
#[test]
fn phantom_mint_lineage_cannot_rebind_to_an_arbitrary_fake_match_master() {
    let mut context = Context::default();
    let (limit_order, helper_type) = limit_order_and_helper_type_scripts(&mut context);

    let phantom_order_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity((1_500 * SHANNONS).pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
        order_data_mint(0, 5, (1, 1)),
    );
    let fake_master = OutPoint::new(Byte32::from_slice(&[7u8; 32]).expect("byte32"), 9);

    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(phantom_order_out_point)
                .build(),
        )
        .output(
            CellOutput::new_builder()
                .capacity((1_400 * SHANNONS).pack())
                .lock(limit_order)
                .type_(Some(helper_type).pack())
                .build(),
        )
        .output_data(order_data_match(100 * SHANNONS as u128, &fake_master, (1, 1)).pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);
}

// Continue a phantom mint into match state while changing its ratio info; the match path still enforces same-order info and rejects the rewrite.
#[test]
fn phantom_limit_order_match_still_requires_same_order_info() {
    let mut context = Context::default();
    let (limit_order, helper_type) = limit_order_and_helper_type_scripts(&mut context);

    let phantom_order_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity((1_500 * SHANNONS).pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
        order_data_mint(0, 5, (1, 1)),
    );
    let phantom_master_out_point = OutPoint::new(phantom_order_out_point.tx_hash(), 5);

    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(phantom_order_out_point)
                .build(),
        )
        .output(
            CellOutput::new_builder()
                .capacity((1_400 * SHANNONS).pack())
                .lock(limit_order)
                .type_(Some(helper_type).pack())
                .build(),
        )
        .output_data(order_data_match(100 * SHANNONS as u128, &phantom_master_out_point, (2, 1)).pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_LIMIT_ORDER_DIFFERENT_INFO);
}

// Try to melt a phantom mint without any master input; the melt path rejects because no matching master lock/type pair is present.
#[test]
fn phantom_limit_order_cannot_be_melted_without_a_master_input() {
    let mut context = Context::default();
    let (limit_order, helper_type) = limit_order_and_helper_type_scripts(&mut context);

    let phantom_order_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity((1_500 * SHANNONS).pack())
            .lock(limit_order)
            .type_(Some(helper_type).pack())
            .build(),
        order_data_mint(0, 5, (1, 1)),
    );

    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(phantom_order_out_point)
                .build(),
        )
        .output(
            CellOutput::new_builder()
                .capacity((1_500 * SHANNONS).pack())
                .lock(always_success_lock(&mut context))
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);
}

// Try to melt a phantom mint against an unrelated real master cell; the melt path rejects because the derived metapoint does not match that master.
#[test]
fn phantom_limit_order_cannot_be_melted_with_an_unrelated_master() {
    let mut context = Context::default();
    let (limit_order, helper_type) = limit_order_and_helper_type_scripts(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");

    let phantom_order_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity((1_500 * SHANNONS).pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
        order_data_mint(0, 5, (1, 1)),
    );
    let unrelated_master_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(owner_lock.clone())
            .type_(Some(limit_order).pack())
            .build(),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(phantom_order_out_point)
                .build(),
        )
        .input(
            CellInput::new_builder()
                .previous_output(unrelated_master_out_point)
                .build(),
        )
        .output(
            CellOutput::new_builder()
                .capacity((1_700 * SHANNONS).pack())
                .lock(owner_lock)
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);
}

// Create a lock-only output that already carries match data and a fake master; creation succeeds because output locks do not execute.
#[test]
fn lock_only_limit_order_output_can_be_created_with_match_order_data() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let helper_type = helper_type_script(&mut context);
    let limit_order = limit_order_script(&mut context);
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((2_000 * SHANNONS).pack())
            .lock(funding_lock)
            .build(),
        Bytes::new(),
    );
    let fake_master = OutPoint::new(Byte32::from_slice(&[7u8; 32]).expect("byte32"), 9);

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .output(
            CellOutput::new_builder()
                .capacity((1_500 * SHANNONS).pack())
                .lock(limit_order)
                .type_(Some(helper_type).pack())
                .build(),
        )
        .output_data(order_data_match(0, &fake_master, (1, 1)).pack())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("lock-only output can be created with MatchOrderData");
}
