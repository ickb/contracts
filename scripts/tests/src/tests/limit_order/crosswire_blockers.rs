use super::*;

/// Two real mint-state orders and their masters, under owners `owner1` and `owner2`, holding
/// 1_600 and 1_500 CKB.
fn two_real_pairs(context: &mut Context, env: &Env) -> ((In, OutPoint), (In, OutPoint)) {
    let mut pair = |owner: &[u8], capacity: u64| {
        let owner_lock = named_always_success_lock(context, owner);
        let (order, master) =
            build_real_limit_order_and_master_with_capacity(context, owner_lock, env.helper.clone(), capacity);
        (live(context, order), master)
    };
    (pair(b"owner1", 1_600 * CKB), pair(b"owner2", 1_500 * CKB))
}

/// Continues `orders` into match outputs of these capacities and data in one transaction.
fn continue_orders(env: &Env, orders: Vec<In>, outputs: Vec<(u64, Bytes)>) -> TxSpec {
    TxSpec {
        inputs: orders,
        outputs: outputs.into_iter().map(|(capacity, data)| env.order(capacity, data).into()).collect(),
        ..Default::default()
    }
}

/// Asserts the crosswire fails the match path, whichever of its value checks catches it first.
fn assert_crosswire_blocked(context: &mut Context, crosswire: &TxSpec) {
    let err = crosswire.verify(context).unwrap_err();
    assert_script_error_in(err, &[ERROR_LIMIT_ORDER_DECREASING_VALUE, ERROR_LIMIT_ORDER_INVALID_MATCH]);
}

// Forge two mint-state real orders with different capacities, then swap their master distances; the later lock path rejects the crosswired match continuation.
#[test]
fn distinct_mint_capacities_block_master_crosswire() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let ((order1, master1), (order2, master2)) = two_real_pairs(&mut context, &env);
    let crosswire = continue_orders(
        &env,
        vec![order1, order2],
        vec![
            (1_500 * CKB, order_data_match(100 * CKB as u128, &master2, (1, 1))),
            (1_400 * CKB, order_data_match(100 * CKB as u128, &master1, (1, 1))),
        ],
    );
    assert_crosswire_blocked(&mut context, &crosswire);
}

// Continue two real lineages to different match progress, then crosswire the continued outputs; the match path rejects because neither output can extend both histories.
#[test]
fn distinct_match_progress_blocks_master_crosswire() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let ((order1, master1), (order2, master2)) = two_real_pairs(&mut context, &env);
    let order1_match = order_data_match(100 * CKB as u128, &master1, (1, 1));
    let order1_match = continue_orders(&env, vec![order1], vec![(1_500 * CKB, order1_match)]);
    let order1_matched = order1_match.accept(&mut context, "order1 should enter its first match state").remove(0);
    let order2_match = order_data_match(200 * CKB as u128, &master2, (1, 1));
    let order2_match = continue_orders(&env, vec![order2], vec![(1_300 * CKB, order2_match)]);
    let order2_matched =
        order2_match.accept(&mut context, "order2 should enter a later match state with different progress").remove(0);

    let crosswire = continue_orders(
        &env,
        vec![order1_matched, order2_matched],
        vec![
            (1_400 * CKB, order_data_match(200 * CKB as u128, &master2, (1, 1))),
            (1_200 * CKB, order_data_match(300 * CKB as u128, &master1, (1, 1))),
        ],
    );
    assert_crosswire_blocked(&mut context, &crosswire);
}

// Spend one real order from mint shape and one from match shape, then swap their master references; the continued match path still rejects the mixed crosswire.
#[test]
fn real_mint_and_match_orders_cannot_crosswire_masters() {
    let mut context = Context::default();
    let env = Env::new(&mut context);
    let ((mint_order, master1), (match_order_seed, master2)) = two_real_pairs(&mut context, &env);
    let seed_match = order_data_match(100 * CKB as u128, &master2, (1, 1));
    let seed_match = continue_orders(&env, vec![match_order_seed], vec![(1_400 * CKB, seed_match)]);
    let match_order = seed_match.accept(&mut context, "the second real order should enter match state before the mixed crosswire")
        .remove(0);

    let crosswire = continue_orders(
        &env,
        vec![mint_order, match_order],
        vec![
            (1_500 * CKB, order_data_match(100 * CKB as u128, &master2, (1, 1))),
            (1_300 * CKB, order_data_match(200 * CKB as u128, &master1, (1, 1))),
        ],
    );
    assert_crosswire_blocked(&mut context, &crosswire);
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
        cell(u64_from_hex("0x23c346000"), &owner_a_lock, Some(&limit_order)),
        Bytes::new(),
    );
    context.create_cell_with_out_point(
        master_b_input.clone(),
        cell(u64_from_hex("0x230489e00"), &owner_b_lock, Some(&limit_order)),
        Bytes::new(),
    );
    context.create_cell_with_out_point(
        order_a_input.clone(),
        cell(u64_from_hex("0x641d5c5e1c4"), &limit_order, Some(&helper_type)),
        bytes_from_hex(
            "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000c16ff28623005b03be74cf7a2a000000000000000000000000000000000021",
        ),
    );
    context.create_cell_with_out_point(
        order_b_input.clone(),
        cell(u64_from_hex("0x4a221e700"), &limit_order, Some(&helper_type)),
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
        .input(input(order_a_input))
        .input(input(order_b_input))
        .outputs(vec![
            cell(u64_from_hex("0x4a221e700"), &limit_order, Some(&helper_type)),
            cell(u64_from_hex("0x74deeef1ad"), &limit_order, Some(&helper_type)),
            cell(u64_from_hex("0x5ccf6d6f017"), &change_lock, None),
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
    fail(&context, &crosswire_tx, ERROR_LIMIT_ORDER_DIFFERENT_INFO);
}
