use super::*;

// Replays the live one-pair mainnet claim plus limit_order match tx without changing protocol ownership, so the matched-order continuation should verify unchanged.
#[test]
fn mainnet_tx_b866945c_claim_and_limit_order_match_shape() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"live-mainnet-claim-match-lock");
    let (_ickb_logic, limit_order, owned_owner, dao, xudt) =
        ickb_logic_limit_order_owned_owner_dao_and_xudt_scripts(&mut context);

    let change_input_out_point = out_point_from_hex(
        "0x1169c04531dba371e3968aec7ea7c2596b5ce7453b4905f9f9bf85e4c7605dc0",
        3,
    );
    context.create_cell_with_out_point(
        change_input_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0xce749cf8e0d").pack())
            .lock(user_lock.clone())
            .build(),
        Bytes::new(),
    );

    let udt_input_out_point = out_point_from_hex(
        "0x1169c04531dba371e3968aec7ea7c2596b5ce7453b4905f9f9bf85e4c7605dc0",
        2,
    );
    context.create_cell_with_out_point(
        udt_input_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x3663a5200").pack())
            .lock(user_lock.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex("0x04f06631280500000000000000000000"),
    );

    let owned_input_out_point = out_point_from_hex(
        "0x1169c04531dba371e3968aec7ea7c2596b5ce7453b4905f9f9bf85e4c7605dc0",
        0,
    );
    context.create_cell_with_out_point(
        owned_input_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0xa923feb5eef").pack())
            .lock(owned_owner.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        bytes_from_hex("0xf1b6e60000000000"),
    );

    let owner_input_out_point = out_point_from_hex(
        "0x1169c04531dba371e3968aec7ea7c2596b5ce7453b4905f9f9bf85e4c7605dc0",
        1,
    );
    context.create_cell_with_out_point(
        owner_input_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x248202200").pack())
            .lock(user_lock.clone())
            .type_(Some(owned_owner).pack())
            .build(),
        bytes_from_hex("0xffffffff"),
    );

    let order_input_out_point = out_point_from_hex(
        "0x4472915f75bb991221c620ddde53bdcd9f5529600ec2a001ca4b709989d3af8f",
        3,
    );
    context.create_cell_with_out_point(
        order_input_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x13ab81f230").pack())
            .lock(limit_order.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex(
            "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000c16ff286230016037f080e722a000000000000000000000000000000000021",
        ),
    );

    let withdraw_header = rpc_header(
        "0x11f119b",
        "0x42d00880035ee",
        "0x005860112be46a5872830402e54f2a0065f8550647f27e0900dab85ea9273607",
    );
    link_cell_to_header(&mut context, &owned_input_out_point, &withdraw_header);
    let deposit_header = rpc_header(
        "0xe6b6f1",
        "0x41d018a002c16",
        "0x84af75537a1aeb523f2ad9a1d4432900266e5ec45a24a107007ce395ec7c3507",
    );
    context.insert_header(deposit_header.clone());

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(change_input_out_point).build())
        .input(CellInput::new_builder().previous_output(udt_input_out_point).build())
        .input(
            CellInput::new_builder()
                .previous_output(owned_input_out_point)
                .since(u64_from_hex("0x20041d018a0035ee").pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(owner_input_out_point).build())
        .input(CellInput::new_builder().previous_output(order_input_out_point).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x4a221e700").pack())
                .lock(limit_order.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x3663a5200").pack())
                .lock(user_lock.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x17cf7b1158b3").pack())
                .lock(user_lock)
                .build(),
        ])
        .outputs_data(
            vec![
                bytes_from_hex(
                    "0xfaadf2950c0000000000000000000000010000004472915f75bb991221c620ddde53bdcd9f5529600ec2a001ca4b709989d3af8f020000000000c16ff286230016037f080e722a000000000000000000000000000000000021",
                ),
                bytes_from_hex("0x0a42749b1b0500000000000000000000"),
                Bytes::new(),
            ]
            .pack(),
        )
        .witnesses(
            vec![
                Bytes::new(),
                Bytes::new(),
                bytes_from_hex("0x1c00000010000000100000001c000000080000000100000000000000"),
            ]
            .pack(),
        )
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("live mainnet one-pair claim plus limit_order match shape should replay locally");
}

// Replays the same live one-pair claim, but injects a sibling owned_owner script with non-empty args into the inputs, so the replay should fail with the input-poisoning check.
#[test]
fn mainnet_tx_b866945c_is_input_poisoned_by_non_empty_args_owned_owner_sibling() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"live-mainnet-claim-match-lock");
    let (ickb_logic, limit_order) = ickb_logic_and_limit_order_scripts(&mut context);
    let owned_owner = owned_owner_script(&mut context);
    let poisoned_owned_owner = data1_script(&mut context, "owned_owner", Bytes::from(vec![1]));
    let dao = dao_script(&mut context);
    let xudt = xudt_script(&mut context, &ickb_logic);

    let change_input_out_point = out_point_from_hex(
        "0x1169c04531dba371e3968aec7ea7c2596b5ce7453b4905f9f9bf85e4c7605dc0",
        3,
    );
    context.create_cell_with_out_point(
        change_input_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0xce749cf8e0d").pack())
            .lock(user_lock.clone())
            .build(),
        Bytes::new(),
    );

    let udt_input_out_point = out_point_from_hex(
        "0x1169c04531dba371e3968aec7ea7c2596b5ce7453b4905f9f9bf85e4c7605dc0",
        2,
    );
    context.create_cell_with_out_point(
        udt_input_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x3663a5200").pack())
            .lock(user_lock.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex("0x04f06631280500000000000000000000"),
    );

    let owned_input_out_point = out_point_from_hex(
        "0x1169c04531dba371e3968aec7ea7c2596b5ce7453b4905f9f9bf85e4c7605dc0",
        0,
    );
    context.create_cell_with_out_point(
        owned_input_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0xa923feb5eef").pack())
            .lock(owned_owner.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        bytes_from_hex("0xf1b6e60000000000"),
    );

    let owner_input_out_point = out_point_from_hex(
        "0x1169c04531dba371e3968aec7ea7c2596b5ce7453b4905f9f9bf85e4c7605dc0",
        1,
    );
    context.create_cell_with_out_point(
        owner_input_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x248202200").pack())
            .lock(user_lock.clone())
            .type_(Some(owned_owner.clone()).pack())
            .build(),
        bytes_from_hex("0xffffffff"),
    );

    let poisoned_owner_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x248202200").pack())
            .lock(user_lock.clone())
            .type_(Some(poisoned_owned_owner).pack())
            .build(),
        bytes_from_hex("0xffffffff"),
    );

    let order_input_out_point = out_point_from_hex(
        "0x4472915f75bb991221c620ddde53bdcd9f5529600ec2a001ca4b709989d3af8f",
        3,
    );
    context.create_cell_with_out_point(
        order_input_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x13ab81f230").pack())
            .lock(limit_order.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex(
            "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000c16ff286230016037f080e722a000000000000000000000000000000000021",
        ),
    );

    let withdraw_header = rpc_header(
        "0x11f119b",
        "0x42d00880035ee",
        "0x005860112be46a5872830402e54f2a0065f8550647f27e0900dab85ea9273607",
    );
    link_cell_to_header(&mut context, &owned_input_out_point, &withdraw_header);
    let deposit_header = rpc_header(
        "0xe6b6f1",
        "0x41d018a002c16",
        "0x84af75537a1aeb523f2ad9a1d4432900266e5ec45a24a107007ce395ec7c3507",
    );
    context.insert_header(deposit_header.clone());

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(change_input_out_point).build())
        .input(CellInput::new_builder().previous_output(udt_input_out_point).build())
        .input(
            CellInput::new_builder()
                .previous_output(owned_input_out_point)
                .since(u64_from_hex("0x20041d018a0035ee").pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(owner_input_out_point).build())
        .input(CellInput::new_builder().previous_output(poisoned_owner_input).build())
        .input(CellInput::new_builder().previous_output(order_input_out_point).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x4a221e700").pack())
                .lock(limit_order.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x3663a5200").pack())
                .lock(user_lock.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x17cf7b1158b3").pack())
                .lock(user_lock)
                .build(),
        ])
        .outputs_data(
            vec![
                bytes_from_hex(
                    "0xfaadf2950c0000000000000000000000010000004472915f75bb991221c620ddde53bdcd9f5529600ec2a001ca4b709989d3af8f020000000000c16ff286230016037f080e722a000000000000000000000000000000000021",
                ),
                bytes_from_hex("0x0a42749b1b0500000000000000000000"),
                Bytes::new(),
            ]
            .pack(),
        )
        .witnesses(
            vec![
                Bytes::new(),
                Bytes::new(),
                bytes_from_hex("0x1c00000010000000100000001c000000080000000100000000000000"),
            ]
            .pack(),
        )
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_NOT_EMPTY_ARGS);
}

// Replays the live mixed deposit plus phase2 follow-up from the same lineage without mutating protocol ownership, so the repeated deposit batch and reminted order should verify unchanged.
#[test]
fn mainnet_tx_dc92e6a3_mixed_deposit_phase2_and_limit_order_shape() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"live-mainnet-dc92-lock");
    let (ickb_logic, limit_order) = ickb_logic_and_limit_order_scripts(&mut context);
    let dao = dao_script(&mut context);
    let xudt = xudt_script(&mut context, &ickb_logic);

    let order_input_out_point = out_point_from_hex(
        "0xb866945c810a90eea0a000beb18cb64b5a3b3e29dec5f40550786872e2576b07",
        0,
    );
    context.create_cell_with_out_point(
        order_input_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x4a221e700").pack())
            .lock(limit_order.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex(
            "0xfaadf2950c0000000000000000000000010000004472915f75bb991221c620ddde53bdcd9f5529600ec2a001ca4b709989d3af8f020000000000c16ff286230016037f080e722a000000000000000000000000000000000021",
        ),
    );

    let master_input_out_point = out_point_from_hex(
        "0x4472915f75bb991221c620ddde53bdcd9f5529600ec2a001ca4b709989d3af8f",
        2,
    );
    context.create_cell_with_out_point(
        master_input_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x23c346000").pack())
            .lock(user_lock.clone())
            .type_(Some(limit_order.clone()).pack())
            .build(),
        Bytes::new(),
    );

    let receipt_input_out_point = out_point_from_hex(
        "0x4472915f75bb991221c620ddde53bdcd9f5529600ec2a001ca4b709989d3af8f",
        1,
    );
    context.create_cell_with_out_point(
        receipt_input_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x283baec00").pack())
            .lock(user_lock.clone())
            .type_(Some(ickb_logic.clone()).pack())
            .build(),
        bytes_from_hex("0x01000000d0a29a05d90a0000"),
    );

    let udt_input_out_point = out_point_from_hex(
        "0x4472915f75bb991221c620ddde53bdcd9f5529600ec2a001ca4b709989d3af8f",
        4,
    );
    context.create_cell_with_out_point(
        udt_input_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x372261400").pack())
            .lock(user_lock.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex("0xab216b4e180900000000000000000000"),
    );

    let change_input_out_point = out_point_from_hex(
        "0x4472915f75bb991221c620ddde53bdcd9f5529600ec2a001ca4b709989d3af8f",
        5,
    );
    context.create_cell_with_out_point(
        change_input_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x3be3a5b1d278").pack())
            .lock(user_lock.clone())
            .build(),
        Bytes::new(),
    );

    let receipt_header = rpc_header(
        "0x1221617",
        "0x692013d003686",
        "0xe4eb5cb10ad9bf58d51562fcbd5f2a00a185f72597719c09007118d6a4723607",
    );
    link_cell_to_header(&mut context, &receipt_input_out_point, &receipt_header);

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(order_input_out_point).build())
        .input(CellInput::new_builder().previous_output(master_input_out_point).build())
        .input(CellInput::new_builder().previous_output(change_input_out_point).build())
        .input(CellInput::new_builder().previous_output(udt_input_out_point).build())
        .input(CellInput::new_builder().previous_output(receipt_input_out_point).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xadaee77b40f").pack())
                .lock(ickb_logic.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xadaee77b40f").pack())
                .lock(ickb_logic.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xadaee77b40f").pack())
                .lock(ickb_logic.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xadaee77b40f").pack())
                .lock(ickb_logic.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xadaee77b40f").pack())
                .lock(ickb_logic.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x283baec00").pack())
                .lock(user_lock.clone())
                .type_(Some(ickb_logic.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x23c346000").pack())
                .lock(user_lock.clone())
                .type_(Some(limit_order.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x49a1fa47875").pack())
                .lock(limit_order)
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x372261400").pack())
                .lock(user_lock.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x1077fd7ab48").pack())
                .lock(user_lock)
                .build(),
        ])
        .outputs_data(
            vec![
                bytes_from_hex("0x0000000000000000"),
                bytes_from_hex("0x0000000000000000"),
                bytes_from_hex("0x0000000000000000"),
                bytes_from_hex("0x0000000000000000"),
                bytes_from_hex("0x0000000000000000"),
                bytes_from_hex("0x050000000fa2b505d90a0000"),
                Bytes::new(),
                bytes_from_hex(
                    "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000c16ff286230079120f720e722a000000000000000000000000000000000021",
                ),
                bytes_from_hex("0x7236cb323d1200000000000000000000"),
                Bytes::new(),
            ]
            .pack(),
        )
        .header_dep(receipt_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("live mainnet mixed deposit phase1 + phase2 + limit_order shape should replay locally");
}
