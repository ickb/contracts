use super::*;

// Replays the live mainnet phase1 tx that opens two DAO deposits plus a receipt and limit_order pair, so the initial creation shape should verify unchanged.
#[test]
fn mainnet_tx_30f8ed8a_phase1_and_limit_order_shape() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"live-mainnet-user-lock");
    let (ickb_logic, limit_order) = ickb_logic_and_limit_order_scripts(&mut context);
    let dao = dao_script(&mut context);
    let xudt = xudt_script(&mut context, &ickb_logic);

    let ckb_input_1 = context.create_cell(
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x4dac4518c238").pack())
            .lock(user_lock.clone())
            .build(),
        Bytes::new(),
    );
    let ckb_input_2 = context.create_cell(
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x2e90edcf91").pack())
            .lock(user_lock.clone())
            .build(),
        Bytes::new(),
    );
    let udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x372261400").pack())
            .lock(user_lock.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex("0x3dc895f28c4300000000000000000000"),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(ckb_input_1).build())
        .input(CellInput::new_builder().previous_output(ckb_input_2).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xadd2b97d89e").pack())
                .lock(ickb_logic.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xadd2b97d89e").pack())
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
                .capacity(u64_from_hex("0x641d5c5e1c4").pack())
                .lock(limit_order.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x372261400").pack())
                .lock(user_lock.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x31d9e921a329").pack())
                .lock(user_lock)
                .build(),
        ])
        .outputs_data(
            vec![
                bytes_from_hex("0x0000000000000000"),
                bytes_from_hex("0x0000000000000000"),
                bytes_from_hex("0x020000009ec6d542db0a0000"),
                Bytes::new(),
                bytes_from_hex(
                    "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000c16ff28623005b03be74cf7a2a000000000000000000000000000000000021",
                ),
                bytes_from_hex("0x3dc895f28c4300000000000000000000"),
                Bytes::new(),
            ]
            .pack(),
        )
        .witness(
            bytes_from_hex(
                "0xcd0100001000000068010000680100005401000002a37dbe47e7783c571d7866cb7208945e886396b41ce33aa3fdbd0c932a5cf552a81d51cb234e2d53fdf63ee832b83100230e1af6900e393b91ce157291e01e5910f833109d9bf4b57d4c0f73b538f48d5f31e5f73bf81f8c3d61957a81c5d03ca69c47beb82e98fa2492171039e70cbce0121e5c5ef3c275eae2282acc952377d280d9320d7862ad09b32103900596a08ba01a51863a8aac3f5ac1969360ae301d000000007b2274797065223a22776562617574686e2e676574222c226368616c6c656e6765223a224f444978597a51304e47597a4f574935597a49784d574d35597a466c4d6a55785a474e68596a6b314e57526c4d574535596d51784d5459794f474977595759334f474d774e7a67774f444d344e6d4d35596d49334d77222c226f726967696e223a2268747470733a2f2f6170702e6a6f792e6964222c2263726f73734f726967696e223a66616c73657d6100000061000000100000001400000016000000000000020001470000004c4f5951598cfec73f7fa9edefa6c916b2ee106e6d714f3146f22965e91df777ed5292601cff007375626b65790000000100000000000000000000000000000000000000004fa6",
            )
            .pack(),
        )
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("live mainnet phase1 + limit_order shape should replay locally");
}

// Replays the live mixed phase2 follow-up for the same mainnet lineage without changing the protocol cells, so receipt consumption and order matching should still verify together.
#[test]
fn mainnet_tx_f9404724_mixed_phase2_and_limit_order_shape() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"live-mainnet-phase2-lock");
    let (ickb_logic, limit_order, xudt) = ickb_logic_limit_order_and_xudt_scripts(&mut context);

    let order_out_point = out_point_from_hex(
        "0xe473654b3c0cb2fbd245051bc42befbe6c0bada95b983160e95382901d02895a",
        0,
    );
    context.create_cell_with_out_point(
        order_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x4a221e700").pack())
            .lock(limit_order.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex(
            "0x12f549ca3705000000000000000000000100000030f8ed8a415adae46b94626ddd986179ab9c3d88513c7c96edc2a62e46f250b9030000000000c16ff28623005b03be74cf7a2a000000000000000000000000000000000021",
        ),
    );
    let master_input = out_point_from_hex(
        "0x30f8ed8a415adae46b94626ddd986179ab9c3d88513c7c96edc2a62e46f250b9",
        3,
    );
    context.create_cell_with_out_point(
        master_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x23c346000").pack())
            .lock(user_lock.clone())
            .type_(Some(limit_order.clone()).pack())
            .build(),
        Bytes::new(),
    );
    let change_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x31d9e921a329").pack())
            .lock(user_lock.clone())
            .build(),
        Bytes::new(),
    );
    let udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x372261400").pack())
            .lock(user_lock.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex("0x3dc895f28c4300000000000000000000"),
    );
    let receipt_out_point = out_point_from_hex(
        "0x30f8ed8a415adae46b94626ddd986179ab9c3d88513c7c96edc2a62e46f250b9",
        2,
    );
    context.create_cell_with_out_point(
        receipt_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x283baec00").pack())
            .lock(user_lock.clone())
            .type_(Some(ickb_logic.clone()).pack())
            .build(),
        bytes_from_hex("0x020000009ec6d542db0a0000"),
    );
    let receipt_header = rpc_header(
        "0x123b44c",
        "0x4c601440036da",
        "0xa4c19f2d57d2ee584ff1f02e7d682a00de295e6644c6ac09002db393e1853607",
    );
    link_cell_to_header(&mut context, &receipt_out_point, &receipt_header);

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(order_out_point).build())
        .input(CellInput::new_builder().previous_output(master_input).build())
        .input(CellInput::new_builder().previous_output(change_input).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .input(CellInput::new_builder().previous_output(receipt_out_point).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x372261400").pack())
                .lock(user_lock.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x31e34b32ce59").pack())
                .lock(user_lock)
                .build(),
        ])
        .outputs_data(
            vec![
                bytes_from_hex("0x0708b959f55a00000000000000000000"),
                Bytes::new(),
            ]
            .pack(),
        )
        .witnesses(vec![Bytes::new(), Bytes::new()].pack())
        .header_dep(receipt_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("live mainnet mixed phase2 + limit_order shape should replay locally");
}
