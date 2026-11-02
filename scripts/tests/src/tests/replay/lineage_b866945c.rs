use super::*;

// Replays the live one-pair mainnet claim plus limit_order match tx without changing protocol ownership, so the matched-order continuation should verify unchanged.
#[test]
fn mainnet_tx_b866945c_claim_and_limit_order_match_shape() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"live-mainnet-claim-match-lock");
    let (_ickb_logic, limit_order, owned_owner, dao, xudt) =
        ickb_logic_limit_order_owned_owner_dao_and_xudt_scripts(&mut context);
    let withdraw_header = rpc_header(
        "0x11f119b",
        "0x42d00880035ee",
        "0x005860112be46a5872830402e54f2a0065f8550647f27e0900dab85ea9273607",
    );
    let deposit_header = rpc_header(
        "0xe6b6f1",
        "0x41d018a002c16",
        "0x84af75537a1aeb523f2ad9a1d4432900266e5ec45a24a107007ce395ec7c3507",
    );

    let withdraw_tx = "0x1169c04531dba371e3968aec7ea7c2596b5ce7453b4905f9f9bf85e4c7605dc0";
    let spec = TxSpec {
        inputs: vec![
            at(withdraw_tx, 3, live("0xce749cf8e0d", &user_lock, None, "0x")),
            at(withdraw_tx, 2, live("0x3663a5200", &user_lock, Some(&xudt), "0x04f06631280500000000000000000000")),
            In {
                header: Some(withdraw_header.clone()),
                since: u64_from_hex("0x20041d018a0035ee"),
                ..at(withdraw_tx, 0, live("0xa923feb5eef", &owned_owner, Some(&dao), "0xf1b6e60000000000"))
            },
            at(withdraw_tx, 1, live("0x248202200", &user_lock, Some(&owned_owner), "0xffffffff")),
            at(
                "0x4472915f75bb991221c620ddde53bdcd9f5529600ec2a001ca4b709989d3af8f",
                3,
                live(
                    "0x13ab81f230",
                    &limit_order,
                    Some(&xudt),
                    "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000c16ff286230016037f080e722a000000000000000000000000000000000021",
                ),
            ),
        ],
        outputs: vec![
            live(
                "0x4a221e700",
                &limit_order,
                Some(&xudt),
                "0xfaadf2950c0000000000000000000000010000004472915f75bb991221c620ddde53bdcd9f5529600ec2a001ca4b709989d3af8f020000000000c16ff286230016037f080e722a000000000000000000000000000000000021",
            )
            .into(),
            live("0x3663a5200", &user_lock, Some(&xudt), "0x0a42749b1b0500000000000000000000").into(),
            live("0x17cf7b1158b3", &user_lock, None, "0x").into(),
        ],
        header_deps: vec![withdraw_header, deposit_header],
    };

    let tx = replay(
        &mut context,
        &spec,
        vec![Bytes::new(), Bytes::new(), bytes_from_hex("0x1c00000010000000100000001c000000080000000100000000000000")],
    );
    context
        .verify(&tx, MAX_CYCLES)
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
    let withdraw_header = rpc_header(
        "0x11f119b",
        "0x42d00880035ee",
        "0x005860112be46a5872830402e54f2a0065f8550647f27e0900dab85ea9273607",
    );
    let deposit_header = rpc_header(
        "0xe6b6f1",
        "0x41d018a002c16",
        "0x84af75537a1aeb523f2ad9a1d4432900266e5ec45a24a107007ce395ec7c3507",
    );

    let withdraw_tx = "0x1169c04531dba371e3968aec7ea7c2596b5ce7453b4905f9f9bf85e4c7605dc0";
    let spec = TxSpec {
        inputs: vec![
            at(withdraw_tx, 3, live("0xce749cf8e0d", &user_lock, None, "0x")),
            at(withdraw_tx, 2, live("0x3663a5200", &user_lock, Some(&xudt), "0x04f06631280500000000000000000000")),
            In {
                header: Some(withdraw_header.clone()),
                since: u64_from_hex("0x20041d018a0035ee"),
                ..at(withdraw_tx, 0, live("0xa923feb5eef", &owned_owner, Some(&dao), "0xf1b6e60000000000"))
            },
            at(withdraw_tx, 1, live("0x248202200", &user_lock, Some(&owned_owner), "0xffffffff")),
            // The one-byte args make the mainnet owner cell one CKB larger.
            In::new(Cell {
                capacity: u64_from_hex("0x248202200") + CKB,
                lock: user_lock.clone(),
                type_: Some(poisoned_owned_owner),
                data: bytes_from_hex("0xffffffff").to_vec(),
            }),
            at(
                "0x4472915f75bb991221c620ddde53bdcd9f5529600ec2a001ca4b709989d3af8f",
                3,
                live(
                    "0x13ab81f230",
                    &limit_order,
                    Some(&xudt),
                    "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000c16ff286230016037f080e722a000000000000000000000000000000000021",
                ),
            ),
        ],
        outputs: vec![
            live(
                "0x4a221e700",
                &limit_order,
                Some(&xudt),
                "0xfaadf2950c0000000000000000000000010000004472915f75bb991221c620ddde53bdcd9f5529600ec2a001ca4b709989d3af8f020000000000c16ff286230016037f080e722a000000000000000000000000000000000021",
            )
            .into(),
            live("0x3663a5200", &user_lock, Some(&xudt), "0x0a42749b1b0500000000000000000000").into(),
            live("0x17cf7b1158b3", &user_lock, None, "0x").into(),
        ],
        header_deps: vec![withdraw_header, deposit_header],
    };

    let tx = replay(
        &mut context,
        &spec,
        vec![Bytes::new(), Bytes::new(), bytes_from_hex("0x1c00000010000000100000001c000000080000000100000000000000")],
    );
    fail(&context, &tx, ERROR_NOT_EMPTY_ARGS);
}

// Replays the live mixed deposit plus phase2 follow-up from the same lineage without mutating protocol ownership, so the repeated deposit batch and reminted order should verify unchanged.
#[test]
fn mainnet_tx_dc92e6a3_mixed_deposit_phase2_and_limit_order_shape() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"live-mainnet-dc92-lock");
    let (ickb_logic, limit_order) = ickb_logic_and_limit_order_scripts(&mut context);
    let dao = dao_script(&mut context);
    let xudt = xudt_script(&mut context, &ickb_logic);
    let receipt_header = rpc_header(
        "0x1221617",
        "0x692013d003686",
        "0xe4eb5cb10ad9bf58d51562fcbd5f2a00a185f72597719c09007118d6a4723607",
    );

    let match_tx = "0x4472915f75bb991221c620ddde53bdcd9f5529600ec2a001ca4b709989d3af8f";
    let deposit = || live("0xadaee77b40f", &ickb_logic, Some(&dao), "0x0000000000000000").into();
    let spec = TxSpec {
        inputs: vec![
            at(
                "0xb866945c810a90eea0a000beb18cb64b5a3b3e29dec5f40550786872e2576b07",
                0,
                live(
                    "0x4a221e700",
                    &limit_order,
                    Some(&xudt),
                    "0xfaadf2950c0000000000000000000000010000004472915f75bb991221c620ddde53bdcd9f5529600ec2a001ca4b709989d3af8f020000000000c16ff286230016037f080e722a000000000000000000000000000000000021",
                ),
            ),
            at(match_tx, 2, live("0x23c346000", &user_lock, Some(&limit_order), "0x")),
            at(match_tx, 5, live("0x3be3a5b1d278", &user_lock, None, "0x")),
            at(match_tx, 4, live("0x372261400", &user_lock, Some(&xudt), "0xab216b4e180900000000000000000000")),
            In {
                header: Some(receipt_header.clone()),
                ..at(match_tx, 1, live("0x283baec00", &user_lock, Some(&ickb_logic), "0x01000000d0a29a05d90a0000"))
            },
        ],
        outputs: vec![
            deposit(),
            deposit(),
            deposit(),
            deposit(),
            deposit(),
            live("0x283baec00", &user_lock, Some(&ickb_logic), "0x050000000fa2b505d90a0000").into(),
            live("0x23c346000", &user_lock, Some(&limit_order), "0x").into(),
            live(
                "0x49a1fa47875",
                &limit_order,
                Some(&xudt),
                "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000c16ff286230079120f720e722a000000000000000000000000000000000021",
            )
            .into(),
            live("0x372261400", &user_lock, Some(&xudt), "0x7236cb323d1200000000000000000000").into(),
            live("0x1077fd7ab48", &user_lock, None, "0x").into(),
        ],
        header_deps: vec![receipt_header],
    };

    // The fixture keeps none of the live witnesses: its stand-in locks need none.
    let tx = replay(&mut context, &spec, vec![]);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("live mainnet mixed deposit phase1 + phase2 + limit_order shape should replay locally");
}
