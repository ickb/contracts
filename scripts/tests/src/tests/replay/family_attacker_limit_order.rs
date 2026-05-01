use super::*;

// Replays the live five-claim batch into its later limit_order consumer, but swaps the fresh owner and master to attacker locks, so the recreated order should stay attacker-controlled and melt cleanly.
#[test]
fn weak_lock_derived_five_claim_live_batch_can_flow_into_later_attacker_owned_limit_order_state() {
    let mut context = Context::default();
    let attacker_lock = named_always_success_lock(&mut context, b"attacker");
    let (_ickb_logic, limit_order, owned_owner, dao, xudt) =
        ickb_logic_limit_order_owned_owner_dao_and_xudt_scripts(&mut context);

    let withdraw_header = rpc_header(
        "0x113a006",
        "0x7080295002c78",
        "0x6c0553271bda2153d797ad6b744e2900a8d5c3b324467b08004f90967fde0909",
    );
    let deposit_headers = [
        rpc_header(
            "0xe24036",
            "0x70804f5002572",
            "0x50f82c742d7f354f8f1118fad4882800f9b3b92c91951e0700383324136bf308",
        ),
        rpc_header(
            "0xe23fa0",
            "0x708045f002572",
            "0x54eef1334273354f1632bf98d2882800d0fcbeff70911e0700a58706256af308",
        ),
        rpc_header(
            "0xe23e80",
            "0x708033f002572",
            "0x142d1acd5f5c354f7344cc06ce88280028cf552485891e07007a1bc16268f308",
        ),
        rpc_header(
            "0xe23ac3",
            "0x708068a002571",
            "0x25e907d25410354fcbeb21d7be882800ac3b9cbe326f1e0700872d8ca662f308",
        ),
        rpc_header(
            "0xe2369a",
            "0x7080261002571",
            "0x4ebd64f0b4bb344f635fb1f0ad88280036a6c0eee7511e0700c04cd56075f308",
        ),
    ];

    let batch_tx_hash = byte32_from_hex(
        "0x8eb6c29fb8be1dfe36a6a3ecb6a131b178df6d57fa93866b78acb06664f6e282",
    );
    let owned_capacities = [
        "0xa6256cf8302",
        "0xa625b231491",
        "0xa625f06529d",
        "0xa626031d463",
        "0xa6260cdd291",
    ];
    let owned_data = [
        "0x9a36e20000000000",
        "0xc33ae20000000000",
        "0x803ee20000000000",
        "0xa03fe20000000000",
        "0x3640e20000000000",
    ];

    for index in 0..owned_capacities.len() {
        let out_point = OutPoint::new(batch_tx_hash.clone(), index as u32);
        context.create_cell_with_out_point(
            out_point.clone(),
            CellOutput::new_builder()
                .capacity(u64_from_hex(owned_capacities[index]).pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            bytes_from_hex(owned_data[index]),
        );
        link_cell_to_header(&mut context, &out_point, &withdraw_header);
    }
    for header in &deposit_headers {
        context.insert_header(header.clone());
    }

    for index in 0..owned_capacities.len() {
        let out_point = OutPoint::new(batch_tx_hash.clone(), (index as u32) + 5);
        context.create_cell_with_out_point(
            out_point,
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x2540be400").pack())
                .lock(attacker_lock.clone())
                .type_(Some(owned_owner.clone()).pack())
                .build(),
            bytes_from_hex("0xfbffffff"),
        );
    }

    let later_tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(OutPoint::new(batch_tx_hash.clone(), 4))
                .since(u64_from_hex("0x20070804f5002c7a").pack())
                .build(),
        )
        .input(
            CellInput::new_builder()
                .previous_output(OutPoint::new(batch_tx_hash.clone(), 3))
                .since(u64_from_hex("0x200708045f002c7a").pack())
                .build(),
        )
        .input(
            CellInput::new_builder()
                .previous_output(OutPoint::new(batch_tx_hash.clone(), 2))
                .since(u64_from_hex("0x200708033f002c7a").pack())
                .build(),
        )
        .input(
            CellInput::new_builder()
                .previous_output(OutPoint::new(batch_tx_hash.clone(), 1))
                .since(u64_from_hex("0x200708068a002c79").pack())
                .build(),
        )
        .input(
            CellInput::new_builder()
                .previous_output(OutPoint::new(batch_tx_hash.clone(), 0))
                .since(u64_from_hex("0x2007080261002c79").pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(OutPoint::new(batch_tx_hash.clone(), 9)).build())
        .input(CellInput::new_builder().previous_output(OutPoint::new(batch_tx_hash.clone(), 8)).build())
        .input(CellInput::new_builder().previous_output(OutPoint::new(batch_tx_hash.clone(), 7)).build())
        .input(CellInput::new_builder().previous_output(OutPoint::new(batch_tx_hash.clone(), 6)).build())
        .input(CellInput::new_builder().previous_output(OutPoint::new(batch_tx_hash, 5)).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x23c346000").pack())
                .lock(attacker_lock.clone())
                .type_(Some(limit_order.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x1bea98cf00").pack())
                .lock(limit_order.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x34d6567f32d3").pack())
                .lock(attacker_lock.clone())
                .build(),
        ])
        .outputs_data(
            vec![
                Bytes::new(),
                bytes_from_hex(
                    "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000c16ff2862300bcedcc32756529000000000000000000000000000000000021",
                ),
                Bytes::new(),
            ]
            .pack(),
        )
        .witnesses(
            vec![
                bytes_from_hex("0x1c00000010000000100000001c000000080000000100000000000000"),
                bytes_from_hex("0x1c00000010000000100000001c000000080000000200000000000000"),
                bytes_from_hex("0x1c00000010000000100000001c000000080000000300000000000000"),
                bytes_from_hex("0x1c00000010000000100000001c000000080000000400000000000000"),
                bytes_from_hex("0x1c00000010000000100000001c000000080000000500000000000000"),
                Bytes::new(),
            ]
            .pack(),
        )
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_headers[0].hash())
        .header_dep(deposit_headers[1].hash())
        .header_dep(deposit_headers[2].hash())
        .header_dep(deposit_headers[3].hash())
        .header_dep(deposit_headers[4].hash())
        .build();

    let later_tx = context.complete_tx(later_tx);
    context
        .verify_tx(&later_tx, MAX_CYCLES)
        .expect("weak-lock five-claim batch should reach the attacker-owned limit_order state");

    let later_hash = later_tx.hash();
    let master_out_point = OutPoint::new(later_hash.clone(), 0);
    let order_out_point = OutPoint::new(later_hash, 1);
    context.create_cell_with_out_point(
        master_out_point.clone(),
        later_tx.outputs().get(0).expect("master output"),
        Bytes::new(),
    );
    context.create_cell_with_out_point(
        order_out_point.clone(),
        later_tx.outputs().get(1).expect("order output"),
        bytes_from_hex(
            "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000c16ff2862300bcedcc32756529000000000000000000000000000000000021",
        ),
    );

    let melt_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(order_out_point).build())
        .input(CellInput::new_builder().previous_output(master_out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity((u64_from_hex("0x1bea98cf00") + u64_from_hex("0x23c346000")).pack())
                .lock(attacker_lock)
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();

    let melt_tx = context.complete_tx(melt_tx);
    context
        .verify_tx(&melt_tx, MAX_CYCLES)
        .expect("attacker should melt the resulting attacker-owned limit_order state");
}

// Replays the live eight-claim batch into the historical later consumer, but keeps the fresh owner, master, and change attacker-owned, so the recreated order should also remain attacker-controlled and meltable.
#[test]
fn weak_lock_derived_eight_claim_batch_can_flow_into_later_attacker_owned_limit_order_state() {
    let mut context = Context::default();
    let attacker_lock = named_always_success_lock(&mut context, b"attacker");
    let (ickb_logic, limit_order, owned_owner, dao, xudt) =
        ickb_logic_limit_order_owned_owner_dao_and_xudt_scripts(&mut context);

    let withdraw_header = rpc_header(
        "0x103a436",
        "0x511038b00311b",
        "0xe8527c922537b9559c17b476e1cd290056cedd92ed7d9508008a16eefae93407",
    );
    let deposit_headers = [
        rpc_header(
            "0xfba66c",
            "0x4ad01f0002fbc",
            "0x9fbfc52bcaf3f45450fb87916da82900d8bd5641b1ee50080002ec946c4a3707",
        ),
        rpc_header(
            "0xfba6dc",
            "0x4ad0260002fbc",
            "0x94870c232c01f5541584192170a829004a9ffda878f3500800956f476c4a3707",
        ),
        rpc_header(
            "0xf7ae28",
            "0x57701af002f09",
            "0xc26333efaae29054fae5e5c24095290025db0c4102762f0800c93eb650f43607",
        ),
        rpc_header(
            "0xf7ae5a",
            "0x57701e1002f09",
            "0x0ca04d8cc7e790549aa202be419529009edf535dd5772f080061c8c69bf43607",
        ),
    ];
    for header in &deposit_headers {
        context.insert_header(header.clone());
    }

    let batch_tx_hash = byte32_from_hex(
        "0xaa5670d939c85198afc38595e0334e859c80716fa64b37aecd7fc8b7511056e4",
    );
    let owned_capacities = [
        "0xaac00b02e36",
        "0xaac0158018d",
        "0xaa71806dfc1",
        "0xaa71806dfc1",
        "0xaa71806dfc1",
        "0xaa71806dfc1",
        "0xaa71849baec",
    ];
    let owned_data = [
        "0x6ca6fb0000000000",
        "0xdca6fb0000000000",
        "0x28aef70000000000",
        "0x28aef70000000000",
        "0x28aef70000000000",
        "0x28aef70000000000",
        "0x5aaef70000000000",
    ];
    for index in 0..owned_capacities.len() {
        let out_point = OutPoint::new(batch_tx_hash.clone(), index as u32);
        context.create_cell_with_out_point(
            out_point.clone(),
            CellOutput::new_builder()
                .capacity(u64_from_hex(owned_capacities[index]).pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            bytes_from_hex(owned_data[index]),
        );
        link_cell_to_header(&mut context, &out_point, &withdraw_header);
    }
    for index in 0..owned_capacities.len() {
        let out_point = OutPoint::new(batch_tx_hash.clone(), (index as u32) + 8);
        context.create_cell_with_out_point(
            out_point,
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x2540be400").pack())
                .lock(attacker_lock.clone())
                .type_(Some(owned_owner.clone()).pack())
                .build(),
            bytes_from_hex("0xf8ffffff"),
        );
    }
    context.create_cell_with_out_point(
        OutPoint::new(batch_tx_hash.clone(), 16),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x23c346000").pack())
            .lock(attacker_lock.clone())
            .type_(Some(limit_order.clone()).pack())
            .build(),
        Bytes::new(),
    );
    context.create_cell_with_out_point(
        OutPoint::new(batch_tx_hash.clone(), 17),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x4a221e700").pack())
            .lock(limit_order.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex(
            "0xd45e0323c20200000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff000000000000000000000000000000000000c16ff2862300fe12a4eaa0ca290021",
        ),
    );
    context.create_cell_with_out_point(
        OutPoint::new(batch_tx_hash.clone(), 18),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x142514df48").pack())
            .lock(attacker_lock.clone())
            .build(),
        Bytes::new(),
    );

    let witness_1 = header_dep_index_witness(1);
    let witness_2 = header_dep_index_witness(2);
    let witness_3 = header_dep_index_witness(3);
    let witness_4 = header_dep_index_witness(4);

    let later_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(OutPoint::new(batch_tx_hash.clone(), 17)).build())
        .input(CellInput::new_builder().previous_output(OutPoint::new(batch_tx_hash.clone(), 16)).build())
        .input(CellInput::new_builder().previous_output(OutPoint::new(batch_tx_hash.clone(), 18)).build())
        .input(
            CellInput::new_builder()
                .previous_output(OutPoint::new(batch_tx_hash.clone(), 6))
                .since(u64_from_hex("0x20057701e1003125").pack())
                .build(),
        )
        .input(
            CellInput::new_builder()
                .previous_output(OutPoint::new(batch_tx_hash.clone(), 5))
                .since(u64_from_hex("0x20057701af003125").pack())
                .build(),
        )
        .input(
            CellInput::new_builder()
                .previous_output(OutPoint::new(batch_tx_hash.clone(), 4))
                .since(u64_from_hex("0x20057701af003125").pack())
                .build(),
        )
        .input(
            CellInput::new_builder()
                .previous_output(OutPoint::new(batch_tx_hash.clone(), 3))
                .since(u64_from_hex("0x20057701af003125").pack())
                .build(),
        )
        .input(
            CellInput::new_builder()
                .previous_output(OutPoint::new(batch_tx_hash.clone(), 2))
                .since(u64_from_hex("0x20057701af003125").pack())
                .build(),
        )
        .input(
            CellInput::new_builder()
                .previous_output(OutPoint::new(batch_tx_hash.clone(), 1))
                .since(u64_from_hex("0x2004ad0260003124").pack())
                .build(),
        )
        .input(
            CellInput::new_builder()
                .previous_output(OutPoint::new(batch_tx_hash.clone(), 0))
                .since(u64_from_hex("0x2004ad01f0003124").pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(OutPoint::new(batch_tx_hash.clone(), 14)).build())
        .input(CellInput::new_builder().previous_output(OutPoint::new(batch_tx_hash.clone(), 13)).build())
        .input(CellInput::new_builder().previous_output(OutPoint::new(batch_tx_hash.clone(), 12)).build())
        .input(CellInput::new_builder().previous_output(OutPoint::new(batch_tx_hash.clone(), 11)).build())
        .input(CellInput::new_builder().previous_output(OutPoint::new(batch_tx_hash.clone(), 10)).build())
        .input(CellInput::new_builder().previous_output(OutPoint::new(batch_tx_hash.clone(), 9)).build())
        .input(CellInput::new_builder().previous_output(OutPoint::new(batch_tx_hash, 8)).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xab5f2c8d6b0").pack())
                .lock(ickb_logic.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xab5f2c8d6b0").pack())
                .lock(ickb_logic.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xab5f2c8d6b0").pack())
                .lock(ickb_logic.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xab5f2c8d6b0").pack())
                .lock(ickb_logic.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xab5f2c8d6b0").pack())
                .lock(ickb_logic.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xab5f2c8d6b0").pack())
                .lock(ickb_logic.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x283baec00").pack())
                .lock(attacker_lock.clone())
                .type_(Some(ickb_logic.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x23c346000").pack())
                .lock(attacker_lock.clone())
                .type_(Some(limit_order.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xa4a505d73a0").pack())
                .lock(limit_order.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x372261400").pack())
                .lock(attacker_lock.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x8c3ee380ed").pack())
                .lock(attacker_lock.clone())
                .build(),
        ])
        .outputs_data(
            vec![
                bytes_from_hex("0x0000000000000000"),
                bytes_from_hex("0x0000000000000000"),
                bytes_from_hex("0x0000000000000000"),
                bytes_from_hex("0x0000000000000000"),
                bytes_from_hex("0x0000000000000000"),
                bytes_from_hex("0x0000000000000000"),
                bytes_from_hex("0x06000000b0c4060ab40a0000"),
                Bytes::new(),
                bytes_from_hex(
                    "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000c16ff286230032784f5372e129000000000000000000000000000000000021",
                ),
                bytes_from_hex("0xd45e0323c20200000000000000000000"),
                Bytes::new(),
            ]
            .pack(),
        )
        .witnesses(
            vec![
                bytes_from_hex("0x10000000100000001000000010000000"),
                bytes_from_hex(
                    "0x69000000100000006900000069000000550000005500000010000000550000005500000041000000fe20813789f911946343d69c6fb13cd56c4b87b333d824a699546f500103d21d21c9a11c7775756aaf6532b24dfbea44b05ed28d8f649fd0a79260c6c77a676c00",
                ),
                bytes_from_hex("0x10000000100000001000000010000000"),
                witness_1,
                witness_2.clone(),
                witness_2.clone(),
                witness_2.clone(),
                witness_2,
                witness_3,
                witness_4,
            ]
            .pack(),
        )
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_headers[3].hash())
        .header_dep(deposit_headers[2].hash())
        .header_dep(deposit_headers[1].hash())
        .header_dep(deposit_headers[0].hash())
        .build();

    let later_tx = context.complete_tx(later_tx);
    context
        .verify_tx(&later_tx, MAX_CYCLES)
        .expect("weak-lock mainnet batch should reach the attacker-owned limit_order state");

    let later_hash = later_tx.hash();
    let master_out_point = OutPoint::new(later_hash.clone(), 7);
    let order_out_point = OutPoint::new(later_hash, 8);
    context.create_cell_with_out_point(
        master_out_point.clone(),
        later_tx.outputs().get(7).expect("master output"),
        Bytes::new(),
    );
    context.create_cell_with_out_point(
        order_out_point.clone(),
        later_tx.outputs().get(8).expect("order output"),
        bytes_from_hex(
            "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000c16ff286230032784f5372e129000000000000000000000000000000000021",
        ),
    );

    let melt_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(order_out_point).build())
        .input(CellInput::new_builder().previous_output(master_out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity((u64_from_hex("0xa4a505d73a0") + u64_from_hex("0x23c346000")).pack())
                .lock(attacker_lock)
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();

    let melt_tx = context.complete_tx(melt_tx);
    context
        .verify_tx(&melt_tx, MAX_CYCLES)
        .expect("attacker should melt the resulting mainnet-shaped limit_order state");
}
