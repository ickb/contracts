use super::*;

// Create a mint-shaped output lock with no real master cell behind its distance; creation passes because the transaction holds no master cell, so limit_order never runs, and the order's own lock does not execute on output.
#[test]
fn phantom_mint_output_can_be_created() {
    let mut context = Context::default();
    let (funding_lock, limit_order, helper_type) = funding_limit_order_and_helper_type_scripts(&mut context);

    let funding_input = context.create_cell(
        cell(2_000 * CKB, &funding_lock, None),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(input(funding_input))
        .output(cell(1_500 * CKB, &limit_order, Some(&helper_type)))
        .output_data(order_data_mint(0, 5, (1, 1)).pack())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("phantom order creation should bypass limit_order validation");
}

// Continue that phantom mint into match state using the derived metapoint only; the match path accepts even though no real master input exists.
#[test]
fn phantom_mint_lineage_can_enter_match_without_real_master() {
    let mut context = Context::default();
    let (limit_order, helper_type) = limit_order_and_helper_type_scripts(&mut context);

    let phantom_order_out_point = context.create_cell(
        cell(1_500 * CKB, &limit_order, Some(&helper_type)),
        order_data_mint(0, 5, (1, 1)),
    );
    let phantom_master_out_point = OutPoint::new(phantom_order_out_point.tx_hash(), 5);

    let tx = TransactionBuilder::default()
        .input(input(phantom_order_out_point))
        .output(cell(1_400 * CKB, &limit_order, Some(&helper_type)))
        .output_data(order_data_match(100 * CKB as u128, &phantom_master_out_point, (1, 1)).pack())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("a phantom mint-shaped order should be able to transition into match state without any real master");
}

// Continue a phantom mint into match state but rewrite its metapoint to an unrelated fake master; the match path rejects the lineage rebind.
#[test]
fn phantom_mint_lineage_cannot_rebind_to_an_arbitrary_fake_match_master() {
    let mut context = Context::default();
    let (limit_order, helper_type) = limit_order_and_helper_type_scripts(&mut context);

    let phantom_order_out_point = context.create_cell(
        cell(1_500 * CKB, &limit_order, Some(&helper_type)),
        order_data_mint(0, 5, (1, 1)),
    );
    let fake_master = OutPoint::new(Byte32::from_slice(&[7u8; 32]).expect("byte32"), 9);

    let tx = TransactionBuilder::default()
        .input(input(phantom_order_out_point))
        .output(cell(1_400 * CKB, &limit_order, Some(&helper_type)))
        .output_data(order_data_match(100 * CKB as u128, &fake_master, (1, 1)).pack())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);
}

// Continue a phantom mint into match state while changing its ratio info; the match path still enforces same-order info and rejects the rewrite.
#[test]
fn phantom_limit_order_match_still_requires_same_order_info() {
    let mut context = Context::default();
    let (limit_order, helper_type) = limit_order_and_helper_type_scripts(&mut context);

    let phantom_order_out_point = context.create_cell(
        cell(1_500 * CKB, &limit_order, Some(&helper_type)),
        order_data_mint(0, 5, (1, 1)),
    );
    let phantom_master_out_point = OutPoint::new(phantom_order_out_point.tx_hash(), 5);

    let tx = TransactionBuilder::default()
        .input(input(phantom_order_out_point))
        .output(cell(1_400 * CKB, &limit_order, Some(&helper_type)))
        .output_data(order_data_match(100 * CKB as u128, &phantom_master_out_point, (2, 1)).pack())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_LIMIT_ORDER_DIFFERENT_INFO);
}

// Try to melt a phantom mint without any master input; the melt path rejects because no matching master lock/type pair is present.
#[test]
fn phantom_limit_order_cannot_be_melted_without_a_master_input() {
    let mut context = Context::default();
    let (limit_order, helper_type) = limit_order_and_helper_type_scripts(&mut context);

    let phantom_order_out_point = context.create_cell(
        cell(1_500 * CKB, &limit_order, Some(&helper_type)),
        order_data_mint(0, 5, (1, 1)),
    );

    let tx = TransactionBuilder::default()
        .input(input(phantom_order_out_point))
        .output(cell(1_500 * CKB, &always_success_lock(&mut context), None))
        .output_data(Bytes::new().pack())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);
}

// Try to melt a phantom mint against an unrelated real master cell; the melt path rejects because the derived metapoint does not match that master.
#[test]
fn phantom_limit_order_cannot_be_melted_with_an_unrelated_master() {
    let mut context = Context::default();
    let (limit_order, helper_type) = limit_order_and_helper_type_scripts(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");

    let phantom_order_out_point = context.create_cell(
        cell(1_500 * CKB, &limit_order, Some(&helper_type)),
        order_data_mint(0, 5, (1, 1)),
    );
    let unrelated_master_out_point = context.create_cell(
        cell(occupied_capacity(&owner_lock, &limit_order, 0), &owner_lock, Some(&limit_order)),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(input(phantom_order_out_point))
        .input(input(unrelated_master_out_point))
        .output(cell(1_700 * CKB, &owner_lock, None))
        .output_data(Bytes::new().pack())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);
}

// Create a lock-only output that already carries match data and a fake master; creation succeeds because output locks do not execute.
#[test]
fn lock_only_limit_order_output_can_be_created_with_match_order_data() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let helper_type = helper_type_script(&mut context);
    let limit_order = limit_order_script(&mut context);
    let funding_input = context.create_cell(
        cell(2_000 * CKB, &funding_lock, None),
        Bytes::new(),
    );
    let fake_master = OutPoint::new(Byte32::from_slice(&[7u8; 32]).expect("byte32"), 9);

    let tx = TransactionBuilder::default()
        .input(input(funding_input))
        .output(cell(1_500 * CKB, &limit_order, Some(&helper_type)))
        .output_data(order_data_match(0, &fake_master, (1, 1)).pack())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("lock-only output can be created with MatchOrderData");
}
