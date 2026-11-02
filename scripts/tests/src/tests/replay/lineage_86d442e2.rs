use super::*;

// Replays the live mainnet deposit plus limit_order match entry tx without mutating the protocol state, so the chain-observed opening shape should verify unchanged.
#[test]
fn mainnet_tx_8aaf4923_deposit_and_limit_order_match_shape() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"live-mainnet-owner-lock");
    let (ickb_logic, limit_order) = ickb_logic_and_limit_order_scripts(&mut context);
    let dao = dao_script(&mut context);
    let xudt = xudt_script(&mut context, &ickb_logic);

    let spec = TxSpec {
        inputs: vec![
            In::new(live("0x3663a5200", &user_lock, Some(&xudt), "0x3ea3bf63450100000000000000000000")),
            In::new(live("0x1c64f893723e", &user_lock, None, "0x")),
            at(
                "0x86d442e2c056386b114b021194c8dbae30c97be8c943513d5bb0aa67a1ab38e8",
                3,
                live(
                    "0xa6d17e8d17c",
                    &limit_order,
                    Some(&xudt),
                    "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000c16ff2862300f76adf1f9a792a000000000000000000000000000000000021",
                ),
            ),
        ],
        outputs: vec![
            live(
                "0x903384caedb",
                &limit_order,
                Some(&xudt),
                "0x00c809ae2e01000000000000000000000100000086d442e2c056386b114b021194c8dbae30c97be8c943513d5bb0aa67a1ab38e8020000000000c16ff2862300f76adf1f9a792a000000000000000000000000000000000021",
            )
            .into(),
            live("0xadcdc889e23", &ickb_logic, Some(&dao), "0x0000000000000000").into(),
            live("0x277cf2a00", &user_lock, Some(&ickb_logic), "0x01000000238cc6f3da0a0000").into(),
            live("0x3663a5200", &user_lock, Some(&xudt), "0x3edbb5b5160000000000000000000000").into(),
            live("0x12ef83d72ac6", &user_lock, None, "0x").into(),
        ],
        ..Default::default()
    };

    let tx = replay(
        &mut context,
        &spec,
        vec![bytes_from_hex(
            "0x5500000010000000550000005500000041000000cf897cf8dd14b9878c96353720effdfbeee2f29c14d738e0aa5b0c7a5a8fef9245252a229492ac302204dd5ee341e8e734dc44067c59890c337c42dade98345a01",
        )],
    );
    context
        .verify(&tx, MAX_CYCLES)
        .expect("live mainnet deposit + limit_order match shape should replay locally");
}

// Replays the live next-step phase2 match from the same lineage without changing its cells, so receipt consumption and continued matching should verify together.
#[test]
fn mainnet_tx_42fe6a4c_phase2_and_limit_order_match_shape() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"live-mainnet-phase2-match-lock");
    let (ickb_logic, limit_order, xudt) = ickb_logic_limit_order_and_xudt_scripts(&mut context);
    let receipt_header = rpc_header(
        "0x1237a63",
        "0x4c403280036ce",
        "0x0060b4f7d056e858996feb5548672a00d59fb1cb2b85aa09002b8d5ce8793607",
    );

    let deposit_tx = "0x8aaf4923a49d3775c9bf37b0800ce9288beb8075764c8e2ec04310d777444680";
    let spec = TxSpec {
        inputs: vec![
            at(deposit_tx, 4, live("0x12ef83d72ac6", &user_lock, None, "0x")),
            at(deposit_tx, 3, live("0x3663a5200", &user_lock, Some(&xudt), "0x3edbb5b5160000000000000000000000")),
            In {
                header: Some(receipt_header.clone()),
                ..at(deposit_tx, 2, live("0x277cf2a00", &user_lock, Some(&ickb_logic), "0x01000000238cc6f3da0a0000"))
            },
            at(
                deposit_tx,
                0,
                live(
                    "0x903384caedb",
                    &limit_order,
                    Some(&xudt),
                    "0x00c809ae2e01000000000000000000000100000086d442e2c056386b114b021194c8dbae30c97be8c943513d5bb0aa67a1ab38e8020000000000c16ff2862300f76adf1f9a792a000000000000000000000000000000000021",
                ),
            ),
        ],
        outputs: vec![
            live(
                "0x4a221e700",
                &limit_order,
                Some(&xudt),
                "0x060aa19eb408000000000000000000000100000086d442e2c056386b114b021194c8dbae30c97be8c943513d5bb0aa67a1ab38e8020000000000c16ff2862300f76adf1f9a792a000000000000000000000000000000000021",
            )
            .into(),
            live("0x3663a5200", &user_lock, Some(&xudt), "0x5a6e8c13a90100000000000000000000").into(),
            live("0x1bf091d09adb", &user_lock, None, "0x").into(),
        ],
        header_deps: vec![receipt_header],
    };

    let tx = replay(
        &mut context,
        &spec,
        vec![bytes_from_hex(
            "0x5500000010000000550000005500000041000000a4f1c5f9ef8580c0f18cd547ff92e025867969174dde4357e033a5342d7fc2a96051b42988e229113baeed5f652e76f0af82ed97b81669cd384fc40c276457c401",
        )],
    );
    context
        .verify(&tx, MAX_CYCLES)
        .expect("live mainnet phase2 plus limit_order match shape should replay locally");
}

// Replays the live claim-and-restake tx that turns two withdrawn owned cells into a fresh deposit, receipt, and limit_order pair, so the historical recomposition should verify unchanged.
#[test]
fn mainnet_tx_86d442e2_claim_and_restake_shape() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"live-mainnet-claim-lock");
    let (ickb_logic, limit_order, owned_owner, dao, xudt) =
        ickb_logic_limit_order_owned_owner_dao_and_xudt_scripts(&mut context);
    let withdraw_header = rpc_header(
        "0x11e44de",
        "0x6c0046a0035c6",
        "0xaf7963315bd6545847eedd55c64b2a0059b7cdd1800f7709009be20186033607",
    );
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

    let withdraw_tx = "0xa862038bdb8a060a9ac40d1c56523f62dfa4329f89abe3e40624c5c14033041b";
    let spec = TxSpec {
        inputs: vec![
            at("0x26cb2a2a4b0ec87884c793ec10a2b5124b571f073f899b76af6d7b1623775003", 1, live("0x2540be391", &user_lock, None, "0x")),
            In {
                header: Some(withdraw_header.clone()),
                since: u64_from_hex("0x20041d018a0035ee"),
                ..at(withdraw_tx, 1, live("0xa923feb5eef", &owned_owner, Some(&dao), "0xf1b6e60000000000"))
            },
            In {
                header: Some(withdraw_header.clone()),
                since: u64_from_hex("0x20065100af0035ea"),
                ..at(withdraw_tx, 0, live("0xabe8fa4120f", &owned_owner, Some(&dao), "0xd5280b0100000000"))
            },
            at(withdraw_tx, 3, live("0x2540be400", &user_lock, Some(&owned_owner), "0xfeffffff")),
            at(withdraw_tx, 2, live("0x2540be400", &user_lock, Some(&owned_owner), "0xfeffffff")),
        ],
        outputs: vec![
            live("0xadcdc7bbf67", &ickb_logic, Some(&dao), "0x0000000000000000").into(),
            live("0x283baec00", &user_lock, Some(&ickb_logic), "0x0100000067adb9f3da0a0000").into(),
            live("0x23c346000", &user_lock, Some(&limit_order), "0x").into(),
            live(
                "0xa6d17e8d17c",
                &limit_order,
                Some(&xudt),
                "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000c16ff2862300f76adf1f9a792a000000000000000000000000000000000021",
            )
            .into(),
            live("0x63eb5b3469", &user_lock, None, "0x").into(),
        ],
        header_deps: vec![withdraw_header, deposit_header_1, deposit_header_2],
    };

    let tx = replay(
        &mut context,
        &spec,
        vec![
            Bytes::new(),
            bytes_from_hex("0x1c00000010000000100000001c000000080000000100000000000000"),
            bytes_from_hex("0x1c00000010000000100000001c000000080000000200000000000000"),
        ],
    );
    context
        .verify(&tx, MAX_CYCLES)
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
    let withdraw_header = rpc_header(
        "0x11e44de",
        "0x6c0046a0035c6",
        "0xaf7963315bd6545847eedd55c64b2a0059b7cdd1800f7709009be20186033607",
    );
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

    let withdraw_tx = "0xa862038bdb8a060a9ac40d1c56523f62dfa4329f89abe3e40624c5c14033041b";
    let spec = TxSpec {
        inputs: vec![
            at("0x26cb2a2a4b0ec87884c793ec10a2b5124b571f073f899b76af6d7b1623775003", 1, live("0x2540be391", &user1_lock, None, "0x")),
            In {
                header: Some(withdraw_header.clone()),
                since: u64_from_hex("0x20041d018a0035ee"),
                ..at(withdraw_tx, 1, live("0xa923feb5eef", &owned_owner, Some(&dao), "0xf1b6e60000000000"))
            },
            In {
                header: Some(withdraw_header.clone()),
                since: u64_from_hex("0x20065100af0035ea"),
                ..at(withdraw_tx, 0, live("0xabe8fa4120f", &owned_owner, Some(&dao), "0xd5280b0100000000"))
            },
            at(withdraw_tx, 3, live("0x2540be400", &user1_lock, Some(&owned_owner), "0xfeffffff")),
            at(withdraw_tx, 2, live("0x2540be400", &user2_lock, Some(&owned_owner), "0xfeffffff")),
        ],
        outputs: vec![
            live("0x2540be391", &user1_lock, None, "0x").into(),
            live("0xa923feb5eef", &owned_owner, Some(&dao), "0xf1b6e60000000000").into(),
            live("0xabe8fa4120f", &owned_owner, Some(&dao), "0xd5280b0100000000").into(),
            live("0x2540be400", &user1_lock, Some(&owned_owner), "0xffffffff").into(),
            live("0x2540be400", &user2_lock, Some(&owned_owner), "0xfdffffff").into(),
        ],
        header_deps: vec![withdraw_header, deposit_header_1, deposit_header_2],
    };

    let rotate_tx = replay(&mut context, &spec, vec![Bytes::new(), header_dep_index_witness(1), header_dep_index_witness(2)]);
    // The node also requires a deposit, not a claim, at a new request's index: the same rule under test.
    assert_script_error(context.verify_where_node_also_rejects(&rotate_tx, MAX_CYCLES, "The DAO output data at index 1").unwrap_err(), ERROR_DAO_NEWLY_CREATED_CELL);
}
