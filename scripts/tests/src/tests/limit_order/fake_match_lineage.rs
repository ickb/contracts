use super::*;

// Start from a forged match-shaped cell that already names a fake master, then keep continuing it; the match path accepts because each step stays self-consistent.
#[test]
fn fake_match_lineage_can_keep_advancing_without_real_master() {
    let mut context = Context::default();
    let (limit_order, helper_type) = limit_order_and_helper_type_scripts(&mut context);
    let fake_master = OutPoint::new(Byte32::from_slice(&[7u8; 32]).expect("byte32"), 9);

    let initial_order = context.create_cell(
        cell(1_500 * CKB, &limit_order, Some(&helper_type)),
        order_data_match(0, &fake_master, (1, 1)),
    );

    let first_match_tx = TransactionBuilder::default()
        .input(input(initial_order))
        .output(cell(1_400 * CKB, &limit_order, Some(&helper_type)))
        .output_data(order_data_match(100 * CKB as u128, &fake_master, (1, 1)).pack())
        .build();

    let first_match_tx = context.complete_tx(first_match_tx);
    context
        .verify(&first_match_tx, MAX_CYCLES)
        .expect("a forged match-shaped order should survive one valid-looking match transition without any real master");

    let first_match_out_point = OutPoint::new(first_match_tx.hash(), 0);
    context.create_cell_with_out_point(
        first_match_out_point.clone(),
        first_match_tx.outputs().get(0).expect("first forged match output"),
        order_data_match(100 * CKB as u128, &fake_master, (1, 1)),
    );

    let second_match_tx = TransactionBuilder::default()
        .input(input(first_match_out_point))
        .output(cell(1_300 * CKB, &limit_order, Some(&helper_type)))
        .output_data(order_data_match(200 * CKB as u128, &fake_master, (1, 1)).pack())
        .build();

    let second_match_tx = context.complete_tx(second_match_tx);
    context
        .verify(&second_match_tx, MAX_CYCLES)
        .expect("the fake match-shaped lineage should stay reusable across multiple later match transitions without any real master");
}

// Forge a foreign-token order with unrelated pricing that points at a real master, melt through that master, then show the legitimate order is stranded.
#[test]
fn foreign_token_fake_order_with_arbitrary_info_can_strand_real_order() {
    let mut context = Context::default();
    let (owner_lock, helper_type) = named_lock_and_helper_type_scripts(&mut context, b"owner");
    let limit_order = limit_order_script(&mut context);
    let funding_lock = always_success_lock(&mut context);
    let funding_input = context.create_cell(
        cell(1_700 * CKB, &funding_lock, None),
        Bytes::new(),
    );
    let mint_tx = TransactionBuilder::default()
        .input(input(funding_input))
        .outputs(vec![
            cell(1_500 * CKB, &limit_order, Some(&helper_type)),
            cell(occupied_capacity(&owner_lock, &limit_order, 0), &owner_lock, Some(&limit_order)),
        ])
        .outputs_data(vec![order_data_mint(0, 1, (1, 1)), Bytes::new()].pack())
        .build();
    let mint_tx = context.complete_tx(mint_tx);
    context
        .verify(&mint_tx, MAX_CYCLES)
        .expect("the real order and occupied-capacity-valid master should mint");
    let real_order_out_point =
        seed_verified_output(&mut context, &mint_tx, 0, order_data_mint(0, 1, (1, 1)));
    let real_master_out_point = seed_verified_output(&mut context, &mint_tx, 1, Bytes::new());

    let foreign_token = named_always_success_lock(&mut context, b"foreign-token");
    let master_index: u32 = real_master_out_point.index().unpack();
    let fake_data = order_data_custom(
        123,
        1,
        real_master_out_point
            .tx_hash()
            .as_slice()
            .try_into()
            .expect("master tx hash"),
        master_index.to_le_bytes(),
        (7, 11),
        (0, 0),
        42,
    );
    let phantom_order_out_point = context.create_cell(
        cell(1_500 * CKB, &limit_order, Some(&foreign_token)),
        fake_data,
    );

    let tx = TransactionBuilder::default()
        .input(input(phantom_order_out_point))
        .input(input(real_master_out_point.clone()))
        .output(cell(1_700 * CKB, &owner_lock, None))
        .output_data(Bytes::new().pack())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("a foreign-token fake order with arbitrary pricing should melt against the referenced real master");

    let stranded_order_tx = TransactionBuilder::default()
        .input(input(real_order_out_point))
        .output(cell(1_500 * CKB, &always_success_lock(&mut context), None))
        .output_data(Bytes::new().pack())
        .build();
    let stranded_order_tx = context.complete_tx(stranded_order_tx);
    fail(&context, &stranded_order_tx, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);
}
