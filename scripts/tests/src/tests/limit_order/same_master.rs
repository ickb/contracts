use super::*;

// Continue one match-shaped order into two outputs that both cite the same master; the match path rejects the duplicated metapoint fan-out.
#[test]
fn match_rejects_two_outputs_sharing_one_master() {
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
                .lock(limit_order.clone())
                .type_(Some(helper_type.clone()).pack())
                .build(),
        )
        .output(
            CellOutput::new_builder()
                .capacity(deposit_capacity(&limit_order, &helper_type, 73, 100 * SHANNONS).pack())
                .lock(limit_order)
                .type_(Some(helper_type).pack())
                .build(),
        )
        .outputs_data(
            vec![
                order_data_match(50 * SHANNONS as u128, &master, (1, 1)),
                order_data_match(10 * SHANNONS as u128, &master, (1, 1)),
            ]
            .pack(),
        )
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_LIMIT_ORDER_SAME_MASTER);
}

// Spend two independent match-shaped inputs that already point at the same master; verification rejects the shared metapoint collision on inputs.
#[test]
fn same_master_collision_on_inputs_is_rejected() {
    let mut context = Context::default();
    let (limit_order, helper_type) = limit_order_and_helper_type_scripts(&mut context);
    let master = OutPoint::new(Byte32::zero(), 5);

    let first_order = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_capacity(&limit_order, &helper_type, 73, 1_500 * SHANNONS).pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
        order_data_match(0, &master, (1, 1)),
    );
    let second_order = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_capacity(&limit_order, &helper_type, 73, 1_400 * SHANNONS).pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
        order_data_match(0, &master, (1, 1)),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(first_order).build())
        .input(CellInput::new_builder().previous_output(second_order).build())
        .output(
            CellOutput::new_builder()
                .capacity((2_900 * SHANNONS).pack())
                .lock(always_success_lock(&mut context))
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_LIMIT_ORDER_SAME_MASTER);
}

// Attempt to spend the exact same master input twice; transaction validation blocks the duplicate input before any contract duplicate-master branch can execute.
#[test]
fn duplicate_master_input_shape_is_blocked_before_script_invariants() {
    let mut context = Context::default();
    let (owner_lock, helper_type) = named_lock_and_helper_type_scripts(&mut context, b"owner");
    let (_real_order_out_point, real_master_out_point) =
        build_real_limit_order_and_master(&mut context, owner_lock.clone(), helper_type);

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(real_master_out_point.clone()).build())
        .input(CellInput::new_builder().previous_output(real_master_out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity(400u64.pack())
                .lock(owner_lock)
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();

    let tx = context.complete_tx(tx);
    // Duplicate inputs are rejected by transaction-level validation before the contract can reach
    // its internal DuplicatedMaster branch, so there is no stable limit_order error code to assert.
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect_err("duplicating a master input should be blocked before a reachable DuplicatedMaster path");
}
