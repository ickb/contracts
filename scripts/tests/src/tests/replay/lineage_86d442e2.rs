use super::*;

// Replays the live mainnet deposit plus limit_order match entry tx without mutating the protocol state, so the chain-observed opening shape should verify unchanged.
#[test]
fn mainnet_tx_8aaf4923_deposit_and_limit_order_match_shape() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"live-mainnet-owner-lock");
    let (ickb_logic, limit_order) = ickb_logic_and_limit_order_scripts(&mut context);
    let dao = dao_script(&mut context);
    let xudt = xudt_script(&mut context, &ickb_logic);

    let udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x3663a5200").pack())
            .lock(user_lock.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex("0x3ea3bf63450100000000000000000000"),
    );
    let ckb_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x1c64f893723e").pack())
            .lock(user_lock.clone())
            .build(),
        Bytes::new(),
    );
    let order_input = out_point_from_hex(
        "0x86d442e2c056386b114b021194c8dbae30c97be8c943513d5bb0aa67a1ab38e8",
        3,
    );
    context.create_cell_with_out_point(
        order_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0xa6d17e8d17c").pack())
            .lock(limit_order.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex(
            "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000c16ff2862300f76adf1f9a792a000000000000000000000000000000000021",
        ),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .input(CellInput::new_builder().previous_output(ckb_input).build())
        .input(CellInput::new_builder().previous_output(order_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x903384caedb").pack())
                .lock(limit_order.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xadcdc889e23").pack())
                .lock(ickb_logic.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x277cf2a00").pack())
                .lock(user_lock.clone())
                .type_(Some(ickb_logic.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x3663a5200").pack())
                .lock(user_lock.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x12ef83d72ac6").pack())
                .lock(user_lock)
                .build(),
        ])
        .outputs_data(
            vec![
                bytes_from_hex(
                    "0x00c809ae2e01000000000000000000000100000086d442e2c056386b114b021194c8dbae30c97be8c943513d5bb0aa67a1ab38e8020000000000c16ff2862300f76adf1f9a792a000000000000000000000000000000000021",
                ),
                bytes_from_hex("0x0000000000000000"),
                bytes_from_hex("0x01000000238cc6f3da0a0000"),
                bytes_from_hex("0x3edbb5b5160000000000000000000000"),
                Bytes::new(),
            ]
            .pack(),
        )
        .witness(
            bytes_from_hex(
                "0x5500000010000000550000005500000041000000cf897cf8dd14b9878c96353720effdfbeee2f29c14d738e0aa5b0c7a5a8fef9245252a229492ac302204dd5ee341e8e734dc44067c59890c337c42dade98345a01",
            )
            .pack(),
        )
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("live mainnet deposit + limit_order match shape should replay locally");
}

// Replays the live next-step phase2 match from the same lineage without changing its cells, so receipt consumption and continued matching should verify together.
#[test]
fn mainnet_tx_42fe6a4c_phase2_and_limit_order_match_shape() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"live-mainnet-phase2-match-lock");
    let (ickb_logic, limit_order, xudt) = ickb_logic_limit_order_and_xudt_scripts(&mut context);

    let change_input = out_point_from_hex(
        "0x8aaf4923a49d3775c9bf37b0800ce9288beb8075764c8e2ec04310d777444680",
        4,
    );
    context.create_cell_with_out_point(
        change_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x12ef83d72ac6").pack())
            .lock(user_lock.clone())
            .build(),
        Bytes::new(),
    );
    let udt_input = out_point_from_hex(
        "0x8aaf4923a49d3775c9bf37b0800ce9288beb8075764c8e2ec04310d777444680",
        3,
    );
    context.create_cell_with_out_point(
        udt_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x3663a5200").pack())
            .lock(user_lock.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex("0x3edbb5b5160000000000000000000000"),
    );
    let receipt_input = out_point_from_hex(
        "0x8aaf4923a49d3775c9bf37b0800ce9288beb8075764c8e2ec04310d777444680",
        2,
    );
    context.create_cell_with_out_point(
        receipt_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x277cf2a00").pack())
            .lock(user_lock.clone())
            .type_(Some(ickb_logic.clone()).pack())
            .build(),
        bytes_from_hex("0x01000000238cc6f3da0a0000"),
    );
    let receipt_header = rpc_header(
        "0x1237a63",
        "0x4c403280036ce",
        "0x0060b4f7d056e858996feb5548672a00d59fb1cb2b85aa09002b8d5ce8793607",
    );
    link_cell_to_header(&mut context, &receipt_input, &receipt_header);
    let order_input = out_point_from_hex(
        "0x8aaf4923a49d3775c9bf37b0800ce9288beb8075764c8e2ec04310d777444680",
        0,
    );
    context.create_cell_with_out_point(
        order_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x903384caedb").pack())
            .lock(limit_order.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex(
            "0x00c809ae2e01000000000000000000000100000086d442e2c056386b114b021194c8dbae30c97be8c943513d5bb0aa67a1ab38e8020000000000c16ff2862300f76adf1f9a792a000000000000000000000000000000000021",
        ),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(change_input).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .input(CellInput::new_builder().previous_output(receipt_input).build())
        .input(CellInput::new_builder().previous_output(order_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x4a221e700").pack())
                .lock(limit_order)
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x3663a5200").pack())
                .lock(user_lock.clone())
                .type_(Some(xudt).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x1bf091d09adb").pack())
                .lock(user_lock)
                .build(),
        ])
        .outputs_data(
            vec![
                bytes_from_hex(
                    "0x060aa19eb408000000000000000000000100000086d442e2c056386b114b021194c8dbae30c97be8c943513d5bb0aa67a1ab38e8020000000000c16ff2862300f76adf1f9a792a000000000000000000000000000000000021",
                ),
                bytes_from_hex("0x5a6e8c13a90100000000000000000000"),
                Bytes::new(),
            ]
            .pack(),
        )
        .witness(
            bytes_from_hex(
                "0x5500000010000000550000005500000041000000a4f1c5f9ef8580c0f18cd547ff92e025867969174dde4357e033a5342d7fc2a96051b42988e229113baeed5f652e76f0af82ed97b81669cd384fc40c276457c401",
            )
            .pack(),
        )
        .header_dep(receipt_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("live mainnet phase2 plus limit_order match shape should replay locally");
}

// Replays the live claim-and-restake tx that turns two withdrawn owned cells into a fresh deposit, receipt, and limit_order pair, so the historical recomposition should verify unchanged.
#[test]
fn mainnet_tx_86d442e2_claim_and_restake_shape() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"live-mainnet-claim-lock");
    let (ickb_logic, limit_order, owned_owner, dao, xudt) =
        ickb_logic_limit_order_owned_owner_dao_and_xudt_scripts(&mut context);

    let funding_out_point = out_point_from_hex(
        "0x26cb2a2a4b0ec87884c793ec10a2b5124b571f073f899b76af6d7b1623775003",
        1,
    );
    context.create_cell_with_out_point(
        funding_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x2540be391").pack())
            .lock(user_lock.clone())
            .build(),
        Bytes::new(),
    );

    let owned1_out_point = out_point_from_hex(
        "0xa862038bdb8a060a9ac40d1c56523f62dfa4329f89abe3e40624c5c14033041b",
        1,
    );
    let owned0_out_point = out_point_from_hex(
        "0xa862038bdb8a060a9ac40d1c56523f62dfa4329f89abe3e40624c5c14033041b",
        0,
    );
    let owner3_out_point = out_point_from_hex(
        "0xa862038bdb8a060a9ac40d1c56523f62dfa4329f89abe3e40624c5c14033041b",
        3,
    );
    let owner2_out_point = out_point_from_hex(
        "0xa862038bdb8a060a9ac40d1c56523f62dfa4329f89abe3e40624c5c14033041b",
        2,
    );

    let owned1 = CellOutput::new_builder()
        .capacity(u64_from_hex("0xa923feb5eef").pack())
        .lock(owned_owner.clone())
        .type_(Some(dao.clone()).pack())
        .build();
    let owned0 = CellOutput::new_builder()
        .capacity(u64_from_hex("0xabe8fa4120f").pack())
        .lock(owned_owner.clone())
        .type_(Some(dao.clone()).pack())
        .build();
    context.create_cell_with_out_point(
        owned1_out_point.clone(),
        owned1.clone(),
        bytes_from_hex("0xf1b6e60000000000"),
    );
    context.create_cell_with_out_point(
        owned0_out_point.clone(),
        owned0.clone(),
        bytes_from_hex("0xd5280b0100000000"),
    );
    context.create_cell_with_out_point(
        owner3_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x2540be400").pack())
            .lock(user_lock.clone())
            .type_(Some(owned_owner.clone()).pack())
            .build(),
        bytes_from_hex("0xfeffffff"),
    );
    context.create_cell_with_out_point(
        owner2_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x2540be400").pack())
            .lock(user_lock.clone())
            .type_(Some(owned_owner.clone()).pack())
            .build(),
        bytes_from_hex("0xfeffffff"),
    );

    let withdraw_header = rpc_header(
        "0x11e44de",
        "0x6c0046a0035c6",
        "0xaf7963315bd6545847eedd55c64b2a0059b7cdd1800f7709009be20186033607",
    );
    link_cell_to_header(&mut context, &owned1_out_point, &withdraw_header);
    link_cell_to_header(&mut context, &owned0_out_point, &withdraw_header);
    let deposit_header_1 = rpc_header(
        "0xe6b6f1",
        "0x41d018a002c16",
        "0x84af75537a1aeb523f2ad9a1d4432900266e5ec45a24a107007ce395ec7c3507",
    );
    let deposit_header_2 = rpc_header(
        "0x10b28d5",
        "0x65100af003266",
        "0xacc68720a6d07156120740f8ebf02900e248e5bef77cd4080099d9bc0bc03407",
    );
    context.insert_header(deposit_header_1.clone());
    context.insert_header(deposit_header_2.clone());

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_out_point).build())
        .input(
            CellInput::new_builder()
                .previous_output(owned1_out_point)
                .since(u64_from_hex("0x20041d018a0035ee").pack())
                .build(),
        )
        .input(
            CellInput::new_builder()
                .previous_output(owned0_out_point)
                .since(u64_from_hex("0x20065100af0035ea").pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(owner3_out_point).build())
        .input(CellInput::new_builder().previous_output(owner2_out_point).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xadcdc7bbf67").pack())
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
                .capacity(u64_from_hex("0xa6d17e8d17c").pack())
                .lock(limit_order)
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x63eb5b3469").pack())
                .lock(user_lock)
                .build(),
        ])
        .outputs_data(
            vec![
                bytes_from_hex("0x0000000000000000"),
                bytes_from_hex("0x0100000067adb9f3da0a0000"),
                Bytes::new(),
                bytes_from_hex(
                    "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000c16ff2862300f76adf1f9a792a000000000000000000000000000000000021",
                ),
                Bytes::new(),
            ]
            .pack(),
        )
        .witnesses(
            vec![
                Bytes::new(),
                bytes_from_hex("0x1c00000010000000100000001c000000080000000100000000000000"),
                bytes_from_hex("0x1c00000010000000100000001c000000080000000200000000000000"),
            ]
            .pack(),
        )
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_header_1.hash())
        .header_dep(deposit_header_2.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("live mainnet claim + restake shape should replay locally");
}

// Replays the same live claim family, but rotates one fresh owned_owner pair onto a different user lock, so the attempted ownership refresh should fail.
#[test]
fn mainnet_live_claims_cannot_rotate_into_new_owned_owner_pairs() {
    let mut context = Context::default();
    let user1_lock = named_always_success_lock(&mut context, b"live-claim-user1");
    let user2_lock = named_always_success_lock(&mut context, b"live-claim-user2");
    let owned_owner = owned_owner_script(&mut context);
    let dao = dao_script(&mut context);

    let funding_out_point = out_point_from_hex(
        "0x26cb2a2a4b0ec87884c793ec10a2b5124b571f073f899b76af6d7b1623775003",
        1,
    );
    context.create_cell_with_out_point(
        funding_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x2540be391").pack())
            .lock(user1_lock.clone())
            .build(),
        Bytes::new(),
    );

    let owned1_out_point = out_point_from_hex(
        "0xa862038bdb8a060a9ac40d1c56523f62dfa4329f89abe3e40624c5c14033041b",
        1,
    );
    let owned0_out_point = out_point_from_hex(
        "0xa862038bdb8a060a9ac40d1c56523f62dfa4329f89abe3e40624c5c14033041b",
        0,
    );
    let owner3_out_point = out_point_from_hex(
        "0xa862038bdb8a060a9ac40d1c56523f62dfa4329f89abe3e40624c5c14033041b",
        3,
    );
    let owner2_out_point = out_point_from_hex(
        "0xa862038bdb8a060a9ac40d1c56523f62dfa4329f89abe3e40624c5c14033041b",
        2,
    );

    context.create_cell_with_out_point(
        owned1_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0xa923feb5eef").pack())
            .lock(owned_owner.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        bytes_from_hex("0xf1b6e60000000000"),
    );
    context.create_cell_with_out_point(
        owned0_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0xabe8fa4120f").pack())
            .lock(owned_owner.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        bytes_from_hex("0xd5280b0100000000"),
    );
    context.create_cell_with_out_point(
        owner3_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x2540be400").pack())
            .lock(user1_lock.clone())
            .type_(Some(owned_owner.clone()).pack())
            .build(),
        bytes_from_hex("0xfeffffff"),
    );
    context.create_cell_with_out_point(
        owner2_out_point.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x2540be400").pack())
            .lock(user2_lock.clone())
            .type_(Some(owned_owner.clone()).pack())
            .build(),
        bytes_from_hex("0xfeffffff"),
    );

    let withdraw_header = rpc_header(
        "0x11e44de",
        "0x6c0046a0035c6",
        "0xaf7963315bd6545847eedd55c64b2a0059b7cdd1800f7709009be20186033607",
    );
    link_cell_to_header(&mut context, &owned1_out_point, &withdraw_header);
    link_cell_to_header(&mut context, &owned0_out_point, &withdraw_header);
    let deposit_header_1 = rpc_header(
        "0xe6b6f1",
        "0x41d018a002c16",
        "0x84af75537a1aeb523f2ad9a1d4432900266e5ec45a24a107007ce395ec7c3507",
    );
    let deposit_header_2 = rpc_header(
        "0x10b28d5",
        "0x65100af003266",
        "0xacc68720a6d07156120740f8ebf02900e248e5bef77cd4080099d9bc0bc03407",
    );
    context.insert_header(deposit_header_1.clone());
    context.insert_header(deposit_header_2.clone());

    let witness1 = header_dep_index_witness(1);
    let witness2 = header_dep_index_witness(2);

    let rotate_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_out_point).build())
        .input(
            CellInput::new_builder()
                .previous_output(owned1_out_point)
                .since(u64_from_hex("0x20041d018a0035ee").pack())
                .build(),
        )
        .input(
            CellInput::new_builder()
                .previous_output(owned0_out_point)
                .since(u64_from_hex("0x20065100af0035ea").pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(owner3_out_point).build())
        .input(CellInput::new_builder().previous_output(owner2_out_point).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x2540be391").pack())
                .lock(user1_lock.clone())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xa923feb5eef").pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xabe8fa4120f").pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x2540be400").pack())
                .lock(user1_lock)
                .type_(Some(owned_owner.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x2540be400").pack())
                .lock(user2_lock)
                .type_(Some(owned_owner).pack())
                .build(),
        ])
        .outputs_data(
            vec![
                Bytes::new(),
                bytes_from_hex("0xf1b6e60000000000"),
                bytes_from_hex("0xd5280b0100000000"),
                bytes_from_hex("0xffffffff"),
                bytes_from_hex("0xfdffffff"),
            ]
            .pack(),
        )
        .witness(Bytes::new().pack())
        .witness(witness1.pack())
        .witness(witness2.pack())
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_header_1.hash())
        .header_dep(deposit_header_2.hash())
        .build();

    let rotate_tx = context.complete_tx(rotate_tx);
    let err = context.verify_tx(&rotate_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_DAO_NEWLY_CREATED_CELL);
}
