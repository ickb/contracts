use super::*;

// Create two mint pairs with crosswired distances: the script still pairs each order with exactly one master, so creation passes, the swapped master can melt, and the intuitive pairing fails.
#[test]
fn mint_crosswire_swaps_order_masters() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let owner1_lock = named_always_success_lock(&mut context, b"owner1");
    let (owner2_lock, helper_type) = named_lock_and_helper_type_scripts(&mut context, b"owner2");
    let limit_order = limit_order_script(&mut context);
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((4_000 * CKB).pack())
            .lock(funding_lock)
            .build(),
        Bytes::new(),
    );

    let create_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity((1_500 * CKB).pack())
                .lock(limit_order.clone())
                .type_(Some(helper_type.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(occupied_capacity(&owner1_lock, &limit_order, 0).pack())
                .lock(owner1_lock.clone())
                .type_(Some(limit_order.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity((1_500 * CKB).pack())
                .lock(limit_order.clone())
                .type_(Some(helper_type.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(occupied_capacity(&owner2_lock, &limit_order, 0).pack())
                .lock(owner2_lock.clone())
                .type_(Some(limit_order.clone()).pack())
                .build(),
        ])
        .outputs_data(
            vec![
                order_data_mint(0, 3, (1, 1)),
                Bytes::new(),
                order_data_mint(0, -1, (1, 1)),
                Bytes::new(),
            ]
            .pack(),
        )
        .build();

    let create_tx = context.complete_tx(create_tx);
    context
        .verify(&create_tx, MAX_CYCLES)
        .expect("cross-wired master assignment should pass deployed mint validation");

    let tx_hash = create_tx.hash();
    let order1 = OutPoint::new(tx_hash.clone(), 0);
    let master1 = OutPoint::new(tx_hash.clone(), 1);
    let order2 = OutPoint::new(tx_hash.clone(), 2);
    let master2 = OutPoint::new(tx_hash, 3);
    context.create_cell_with_out_point(
        order1.clone(),
        create_tx.outputs().get(0).expect("order1"),
        order_data_mint(0, 3, (1, 1)),
    );
    context.create_cell_with_out_point(
        master1.clone(),
        create_tx.outputs().get(1).expect("master1"),
        Bytes::new(),
    );
    context.create_cell_with_out_point(
        order2.clone(),
        create_tx.outputs().get(2).expect("order2"),
        order_data_mint(0, -1, (1, 1)),
    );
    context.create_cell_with_out_point(
        master2,
        create_tx.outputs().get(3).expect("master2"),
        Bytes::new(),
    );

    let melt_other_users_order = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(order2).build())
        .input(CellInput::new_builder().previous_output(master1.clone()).build())
        .output(
            CellOutput::new_builder()
                .capacity((1_700 * CKB).pack())
                .lock(owner1_lock)
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();
    let melt_other_users_order = context.complete_tx(melt_other_users_order);
    context
        .verify(&melt_other_users_order, MAX_CYCLES)
        .expect("master1 is bound to order2 when mint distances are cross-wired");

    let melt_expected_order = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(order1).build())
        .input(CellInput::new_builder().previous_output(master1).build())
        .output(
            CellOutput::new_builder()
                .capacity((1_700 * CKB).pack())
                .lock(owner2_lock)
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();
    let melt_expected_order = context.complete_tx(melt_expected_order);
    let err = context.verify(&melt_expected_order, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);
}

// Create the same crosswire through sparse output spacing and negative distances; limit_order runs at mint and accepts the crosswired distances, and the far paired master remains the valid melt path.
#[test]
fn sparse_far_distance_limit_order_crosswire_still_rebinds_master_assignment() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let owner1_lock = named_always_success_lock(&mut context, b"owner1");
    let owner2_lock = named_always_success_lock(&mut context, b"owner2");
    let (filler_lock, helper_type) = named_lock_and_helper_type_scripts(&mut context, b"filler");
    let limit_order = limit_order_script(&mut context);
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((8_000 * CKB).pack())
            .lock(funding_lock)
            .build(),
        Bytes::new(),
    );

    let mut outputs = vec![
        CellOutput::new_builder()
            .capacity((1_500 * CKB).pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
    ];
    let mut outputs_data = vec![order_data_mint(0, 11, (1, 1))];
    for _ in 0..8 {
        outputs.push(
            CellOutput::new_builder()
                .capacity((100 * CKB).pack())
                .lock(filler_lock.clone())
                .build(),
        );
        outputs_data.push(Bytes::new());
    }
    outputs.push(
        CellOutput::new_builder()
            .capacity((1_500 * CKB).pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
    );
    outputs_data.push(order_data_mint(0, 3, (1, 1)));
    outputs.push(
        CellOutput::new_builder()
            .capacity((100 * CKB).pack())
            .lock(filler_lock)
            .build(),
    );
    outputs_data.push(Bytes::new());
    outputs.push(
        CellOutput::new_builder()
            .capacity(occupied_capacity(&owner2_lock, &limit_order, 0).pack())
            .lock(owner2_lock.clone())
            .type_(Some(limit_order.clone()).pack())
            .build(),
    );
    outputs_data.push(Bytes::new());
    outputs.push(
        CellOutput::new_builder()
            .capacity(occupied_capacity(&owner1_lock, &limit_order, 0).pack())
            .lock(owner1_lock.clone())
            .type_(Some(limit_order.clone()).pack())
            .build(),
    );
    outputs_data.push(Bytes::new());

    let create_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .outputs(outputs)
        .outputs_data(outputs_data.pack())
        .build();

    let create_tx = context.complete_tx(create_tx);
    context
        .verify(&create_tx, MAX_CYCLES)
        .expect("sparse far-distance crosswire should still pass deployed mint validation");

    let tx_hash = create_tx.hash();
    let order1 = OutPoint::new(tx_hash.clone(), 0);
    let order2 = OutPoint::new(tx_hash.clone(), 9);
    let master2 = OutPoint::new(tx_hash.clone(), 11);
    let master1 = OutPoint::new(tx_hash, 12);
    context.create_cell_with_out_point(
        order1.clone(),
        create_tx.outputs().get(0).expect("order1"),
        order_data_mint(0, 11, (1, 1)),
    );
    context.create_cell_with_out_point(
        order2.clone(),
        create_tx.outputs().get(9).expect("order2"),
        order_data_mint(0, 3, (1, 1)),
    );
    context.create_cell_with_out_point(
        master2.clone(),
        create_tx.outputs().get(11).expect("master2"),
        Bytes::new(),
    );
    context.create_cell_with_out_point(
        master1.clone(),
        create_tx.outputs().get(12).expect("master1"),
        Bytes::new(),
    );

    let melt_crosswired = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(order1.clone()).build())
        .input(CellInput::new_builder().previous_output(master2.clone()).build())
        .output(
            CellOutput::new_builder()
                .capacity((1_700 * CKB).pack())
                .lock(owner2_lock)
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();

    let melt_crosswired = context.complete_tx(melt_crosswired);
    context
        .verify(&melt_crosswired, MAX_CYCLES)
        .expect("far-distance sparse layout still binds order1 to owner2's master");

    let melt_expected = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(order1).build())
        .input(CellInput::new_builder().previous_output(master1).build())
        .output(
            CellOutput::new_builder()
                .capacity((1_700 * CKB).pack())
                .lock(owner1_lock)
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();
    let melt_expected = context.complete_tx(melt_expected);
    let err = context.verify(&melt_expected, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);
}

// A master's type script runs at mint and pairs every order with exactly one master: one order per
// master passes, two orders on one master fail, a master without an order fails, and an order
// pointing at a cell that is no master fails.
#[test]
fn mint_pairs_each_order_with_exactly_one_master() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let (_, helper_type) = named_lock_and_helper_type_scripts(&mut context, b"owner");
    let limit_order = limit_order_script(&mut context);
    let order = || {
        CellOutput::new_builder()
            .capacity(deposit_capacity(&limit_order, &helper_type, 89, 1_500 * CKB).pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build()
    };
    let master = || {
        CellOutput::new_builder()
            .capacity(occupied_capacity(&owner_lock, &limit_order, 0).pack())
            .lock(owner_lock.clone())
            .type_(Some(limit_order.clone()).pack())
            .build()
    };
    let mut mint = |outputs: Vec<(CellOutput, Bytes)>| {
        let funding = funding_cell(&mut context);
        let (cells, data): (Vec<_>, Vec<_>) = outputs.into_iter().unzip();
        let tx = TransactionBuilder::default()
            .input(CellInput::new_builder().previous_output(funding).build())
            .outputs(cells)
            .outputs_data(data.pack())
            .build();
        let tx = context.complete_tx(tx);
        context.verify(&tx, MAX_CYCLES)
    };

    mint(vec![(order(), order_data_mint(0, 1, (1, 1))), (master(), Bytes::new())])
        .expect("one order with its own master mints");

    let two_orders_one_master = mint(vec![
        (order(), order_data_mint(0, 2, (1, 1))),
        (order(), order_data_mint(0, 1, (1, 1))),
        (master(), Bytes::new()),
    ]);
    assert_script_error(two_orders_one_master.unwrap_err(), ERROR_LIMIT_ORDER_SAME_MASTER);

    let master_without_order = mint(vec![
        (order(), order_data_mint(0, 1, (1, 1))),
        (master(), Bytes::new()),
        (master(), Bytes::new()),
    ]);
    assert_script_error(master_without_order.unwrap_err(), ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);

    let plain = CellOutput::new_builder()
        .capacity((100 * CKB).pack())
        .lock(owner_lock.clone())
        .build();
    let order_without_master = mint(vec![
        (order(), order_data_mint(0, 1, (1, 1))),
        (master(), Bytes::new()),
        (order(), order_data_mint(0, 1, (1, 1))),
        (plain, Bytes::new()),
    ]);
    assert_script_error(order_without_master.unwrap_err(), ERROR_LIMIT_ORDER_INVALID_CONFIGURATION);
}
