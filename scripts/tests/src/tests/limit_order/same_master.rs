use super::*;

// Continue one match-shaped order into two outputs that both cite the same master; the match path rejects the duplicated metapoint fan-out.
#[test]
fn match_rejects_two_outputs_sharing_one_master() {
    let mut context = Context::default();
    let (limit_order, helper_type) = limit_order_and_helper_type_scripts(&mut context);
    let master = OutPoint::new(Byte32::zero(), 5);

    let input_order_out_point = context.create_cell(
        cell(deposit_capacity(&limit_order, &helper_type, 89, 1_500 * CKB), &limit_order, Some(&helper_type)),
        order_data_match(0, &master, (1, 1)),
    );

    let tx = TransactionBuilder::default()
        .input(input(input_order_out_point))
        .output(
            cell(deposit_capacity(&limit_order, &helper_type, 89, 1_400 * CKB), &limit_order, Some(&helper_type)),
        )
        .output(
            cell(deposit_capacity(&limit_order, &helper_type, 89, 100 * CKB), &limit_order, Some(&helper_type)),
        )
        .outputs_data(
            vec![
                order_data_match(50 * CKB as u128, &master, (1, 1)),
                order_data_match(10 * CKB as u128, &master, (1, 1)),
            ]
            .pack(),
        )
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_LIMIT_ORDER_SAME_MASTER);
}

// Spend two independent match-shaped inputs that already point at the same master; verification rejects the shared metapoint collision on inputs.
#[test]
fn same_master_collision_on_inputs_is_rejected() {
    let mut context = Context::default();
    let (limit_order, helper_type) = limit_order_and_helper_type_scripts(&mut context);
    let master = OutPoint::new(Byte32::zero(), 5);

    let first_order = context.create_cell(
        cell(deposit_capacity(&limit_order, &helper_type, 89, 1_500 * CKB), &limit_order, Some(&helper_type)),
        order_data_match(0, &master, (1, 1)),
    );
    let second_order = context.create_cell(
        cell(deposit_capacity(&limit_order, &helper_type, 89, 1_400 * CKB), &limit_order, Some(&helper_type)),
        order_data_match(0, &master, (1, 1)),
    );

    let tx = TransactionBuilder::default()
        .input(input(first_order))
        .input(input(second_order))
        .output(cell(2_900 * CKB, &always_success_lock(&mut context), None))
        .output_data(Bytes::new().pack())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_LIMIT_ORDER_SAME_MASTER);
}

// Attempt to spend the exact same master input twice; transaction validation blocks the duplicate input before any contract duplicate-master branch can execute.
#[test]
fn duplicate_master_input_shape_is_blocked_before_script_invariants() {
    let mut context = Context::default();
    let (owner_lock, helper_type) = named_lock_and_helper_type_scripts(&mut context, b"owner");
    let (_real_order_out_point, real_master_out_point) =
        build_real_limit_order_and_master(&mut context, owner_lock.clone(), helper_type);

    let tx = TransactionBuilder::default()
        .input(input(real_master_out_point.clone()))
        .input(input(real_master_out_point))
        .output(cell(400 * CKB, &owner_lock, None))
        .output_data(Bytes::new().pack())
        .build();

    let tx = context.complete_tx(tx);
    // Duplicate inputs are rejected by transaction-level validation before the contract can reach
    // its internal DuplicatedMaster branch, so there is no stable limit_order error code to assert.
    context
        .verify(&tx, MAX_CYCLES)
        .expect_err("duplicating a master input should be blocked before a reachable DuplicatedMaster path");
}
