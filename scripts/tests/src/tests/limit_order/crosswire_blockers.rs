use super::*;

// Forge two mint-state real orders with different capacities, then swap their master distances; the later lock path rejects the crosswired match continuation.
#[test]
fn distinct_mint_capacities_block_master_crosswire() {
    let mut context = Context::default();
    let owner1_lock = named_always_success_lock(&mut context, b"owner1");
    let (owner2_lock, helper_type) = named_lock_and_helper_type_scripts(&mut context, b"owner2");
    let (order1_input, master1_input) = build_real_limit_order_and_master_with_capacity(
        &mut context,
        owner1_lock.clone(),
        helper_type.clone(),
        1_600 * SHANNONS,
    );
    let (order2_input, master2_input) = build_real_limit_order_and_master_with_capacity(
        &mut context,
        owner2_lock.clone(),
        helper_type.clone(),
        1_500 * SHANNONS,
    );

    let limit_order = limit_order_script(&mut context);
    let crosswired_order1_data = order_data_match(100 * SHANNONS as u128, &master2_input, (1, 1));
    let crosswired_order2_data = order_data_match(100 * SHANNONS as u128, &master1_input, (1, 1));
    let crosswire_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(order1_input).build())
        .input(CellInput::new_builder().previous_output(order2_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity((1_500 * SHANNONS).pack())
                .lock(limit_order.clone())
                .type_(Some(helper_type.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity((1_400 * SHANNONS).pack())
                .lock(limit_order)
                .type_(Some(helper_type).pack())
                .build(),
        ])
        .outputs_data(vec![crosswired_order1_data.clone(), crosswired_order2_data].pack())
        .build();

    let crosswire_tx = context.complete_tx(crosswire_tx);
    let err = context.verify_tx(&crosswire_tx, MAX_CYCLES).unwrap_err();
    assert_script_error_in(
        err,
        &[
            ERROR_LIMIT_ORDER_DECREASING_VALUE,
            ERROR_LIMIT_ORDER_INVALID_MATCH,
        ],
    );
}

// Continue two real lineages to different match progress, then crosswire the continued outputs; the match path rejects because neither output can extend both histories.
#[test]
fn distinct_match_progress_blocks_master_crosswire() {
    let mut context = Context::default();
    let owner1_lock = named_always_success_lock(&mut context, b"owner1");
    let (owner2_lock, helper_type) = named_lock_and_helper_type_scripts(&mut context, b"owner2");
    let (order1_input, master1_input) = build_real_limit_order_and_master_with_capacity(
        &mut context,
        owner1_lock.clone(),
        helper_type.clone(),
        1_600 * SHANNONS,
    );
    let (order2_input, master2_input) = build_real_limit_order_and_master_with_capacity(
        &mut context,
        owner2_lock.clone(),
        helper_type.clone(),
        1_500 * SHANNONS,
    );
    let limit_order = limit_order_script(&mut context);

    let order1_matched_data = order_data_match(100 * SHANNONS as u128, &master1_input, (1, 1));
    let order1_match_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(order1_input).build())
        .output(
            CellOutput::new_builder()
                .capacity((1_500 * SHANNONS).pack())
                .lock(limit_order.clone())
                .type_(Some(helper_type.clone()).pack())
                .build(),
        )
        .output_data(order1_matched_data.clone().pack())
        .build();
    let order1_match_tx = context.complete_tx(order1_match_tx);
    context
        .verify_tx(&order1_match_tx, MAX_CYCLES)
        .expect("order1 should enter its first match state");
    let order1_matched = OutPoint::new(order1_match_tx.hash(), 0);
    context.create_cell_with_out_point(
        order1_matched.clone(),
        order1_match_tx.outputs().get(0).expect("order1 matched output"),
        order1_matched_data,
    );

    let order2_matched_data = order_data_match(200 * SHANNONS as u128, &master2_input, (1, 1));
    let order2_match_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(order2_input).build())
        .output(
            CellOutput::new_builder()
                .capacity((1_300 * SHANNONS).pack())
                .lock(limit_order.clone())
                .type_(Some(helper_type.clone()).pack())
                .build(),
        )
        .output_data(order2_matched_data.clone().pack())
        .build();
    let order2_match_tx = context.complete_tx(order2_match_tx);
    context
        .verify_tx(&order2_match_tx, MAX_CYCLES)
        .expect("order2 should enter a later match state with different progress");
    let order2_matched = OutPoint::new(order2_match_tx.hash(), 0);
    context.create_cell_with_out_point(
        order2_matched.clone(),
        order2_match_tx.outputs().get(0).expect("order2 matched output"),
        order2_matched_data,
    );

    let crosswired_order1_data = order_data_match(200 * SHANNONS as u128, &master2_input, (1, 1));
    let crosswired_order2_data = order_data_match(300 * SHANNONS as u128, &master1_input, (1, 1));
    let crosswire_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(order1_matched).build())
        .input(CellInput::new_builder().previous_output(order2_matched).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity((1_400 * SHANNONS).pack())
                .lock(limit_order.clone())
                .type_(Some(helper_type.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity((1_200 * SHANNONS).pack())
                .lock(limit_order)
                .type_(Some(helper_type).pack())
                .build(),
        ])
        .outputs_data(vec![crosswired_order1_data.clone(), crosswired_order2_data].pack())
        .build();

    let crosswire_tx = context.complete_tx(crosswire_tx);
    let err = context.verify_tx(&crosswire_tx, MAX_CYCLES).unwrap_err();
    assert_script_error_in(
        err,
        &[
            ERROR_LIMIT_ORDER_DECREASING_VALUE,
            ERROR_LIMIT_ORDER_INVALID_MATCH,
        ],
    );
}

// Spend one real order from mint shape and one from match shape, then swap their master references; the continued match path still rejects the mixed crosswire.
#[test]
fn real_mint_and_match_orders_cannot_crosswire_masters() {
    let mut context = Context::default();
    let owner1_lock = named_always_success_lock(&mut context, b"owner1");
    let (owner2_lock, helper_type) = named_lock_and_helper_type_scripts(&mut context, b"owner2");
    let (mint_order_input, master1_input) = build_real_limit_order_and_master_with_capacity(
        &mut context,
        owner1_lock.clone(),
        helper_type.clone(),
        1_600 * SHANNONS,
    );
    let (match_order_seed, master2_input) = build_real_limit_order_and_master_with_capacity(
        &mut context,
        owner2_lock.clone(),
        helper_type.clone(),
        1_500 * SHANNONS,
    );
    let limit_order = limit_order_script(&mut context);

    let match_order_data = order_data_match(100 * SHANNONS as u128, &master2_input, (1, 1));
    let seed_match_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(match_order_seed).build())
        .output(
            CellOutput::new_builder()
                .capacity((1_400 * SHANNONS).pack())
                .lock(limit_order.clone())
                .type_(Some(helper_type.clone()).pack())
                .build(),
        )
        .output_data(match_order_data.clone().pack())
        .build();
    let seed_match_tx = context.complete_tx(seed_match_tx);
    context
        .verify_tx(&seed_match_tx, MAX_CYCLES)
        .expect("the second real order should enter match state before the mixed crosswire");
    let match_order_input = OutPoint::new(seed_match_tx.hash(), 0);
    context.create_cell_with_out_point(
        match_order_input.clone(),
        seed_match_tx.outputs().get(0).expect("match-state order"),
        match_order_data,
    );

    let crosswired_mint_data = order_data_match(100 * SHANNONS as u128, &master2_input, (1, 1));
    let crosswired_match_data = order_data_match(200 * SHANNONS as u128, &master1_input, (1, 1));
    let crosswire_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(mint_order_input).build())
        .input(CellInput::new_builder().previous_output(match_order_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity((1_500 * SHANNONS).pack())
                .lock(limit_order.clone())
                .type_(Some(helper_type.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity((1_300 * SHANNONS).pack())
                .lock(limit_order)
                .type_(Some(helper_type).pack())
                .build(),
        ])
        .outputs_data(vec![crosswired_mint_data.clone(), crosswired_match_data].pack())
        .build();

    let crosswire_tx = context.complete_tx(crosswire_tx);
    let err = context.verify_tx(&crosswire_tx, MAX_CYCLES).unwrap_err();
    assert_script_error_in(
        err,
        &[
            ERROR_LIMIT_ORDER_DECREASING_VALUE,
            ERROR_LIMIT_ORDER_INVALID_MATCH,
        ],
    );
}

// Replay two live mainnet-shaped match cells with different checked info, then swap their masters; verification reaches the match path and fails on different info.
#[test]
fn different_info_mainnet_orders_cannot_crosswire_during_match() {
    let mut context = Context::default();
    let (_owner_a_key, owner_a_lock, _owner_a_secp_data_dep) = secp_lock(&mut context);
    let (_owner_b_key, owner_b_lock, _owner_b_secp_data_dep) = secp_lock(&mut context);
    let helper_type = named_always_success_lock(&mut context, b"live-helper-type");
    let limit_order = limit_order_script(&mut context);
    let change_lock = named_always_success_lock(&mut context, b"live-change");

    let master_a_input = out_point_from_hex(
        "0x30f8ed8a415adae46b94626ddd986179ab9c3d88513c7c96edc2a62e46f250b9",
        3,
    );
    let order_a_input = out_point_from_hex(
        "0x30f8ed8a415adae46b94626ddd986179ab9c3d88513c7c96edc2a62e46f250b9",
        4,
    );
    let master_b_input = out_point_from_hex(
        "0x04f46b529e33c003d9c75a8cd1cc384bcf4a5e21b1712423ebc664fbcde7df40",
        0,
    );
    let order_b_input = out_point_from_hex(
        "0x04f46b529e33c003d9c75a8cd1cc384bcf4a5e21b1712423ebc664fbcde7df40",
        1,
    );

    context.create_cell_with_out_point(
        master_a_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x23c346000").pack())
            .lock(owner_a_lock.clone())
            .type_(Some(limit_order.clone()).pack())
            .build(),
        Bytes::new(),
    );
    context.create_cell_with_out_point(
        master_b_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x230489e00").pack())
            .lock(owner_b_lock.clone())
            .type_(Some(limit_order.clone()).pack())
            .build(),
        Bytes::new(),
    );
    context.create_cell_with_out_point(
        order_a_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x641d5c5e1c4").pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
        bytes_from_hex(
            "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000c16ff28623005b03be74cf7a2a000000000000000000000000000000000021",
        ),
    );
    context.create_cell_with_out_point(
        order_b_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x4a221e700").pack())
            .lock(limit_order.clone())
            .type_(Some(helper_type.clone()).pack())
            .build(),
        bytes_from_hex(
            "0xff2222085e0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff000000000000000000000000000000000000c16ff28623000a1338d4d7672a0021",
        ),
    );

    let crosswired_order_a_data = order_data_custom(
        u128::from_le_bytes(
            hex::decode("12f549ca370500000000000000000000")
                .expect("live order a matched amount")
                .try_into()
                .expect("u128 amount"),
        ),
        1,
        master_b_input
            .tx_hash()
            .as_slice()
            .try_into()
            .expect("master b tx hash"),
        0u32.to_le_bytes(),
        (
            u64_from_hex("0x002386f26fc10000"),
            u64_from_hex("0x002a7acf74be035b"),
        ),
        (0, 0),
        0x21,
    );
    let crosswired_order_b_data = order_data_custom(
        0,
        1,
        master_a_input
            .tx_hash()
            .as_slice()
            .try_into()
            .expect("master a tx hash"),
        3u32.to_le_bytes(),
        (0, 0),
        (
            u64_from_hex("0x002386f26fc10000"),
            u64_from_hex("0x002a67d7d438130a"),
        ),
        0x21,
    );

    let crosswire_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(order_a_input).build())
        .input(CellInput::new_builder().previous_output(order_b_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x4a221e700").pack())
                .lock(limit_order.clone())
                .type_(Some(helper_type.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x74deeef1ad").pack())
                .lock(limit_order.clone())
                .type_(Some(helper_type.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x5ccf6d6f017").pack())
                .lock(change_lock)
                .build(),
        ])
        .outputs_data(
            vec![
                crosswired_order_a_data.clone(),
                crosswired_order_b_data.clone(),
                Bytes::new(),
            ]
            .pack(),
        )
        .build();

    let crosswire_tx = context.complete_tx(crosswire_tx);
    let err = context.verify_tx(&crosswire_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_LIMIT_ORDER_DIFFERENT_INFO);
}
