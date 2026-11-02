use super::*;

// Replays the live mainnet tx that combines an owned_owner claim, a fresh deposit and receipt, and a continuing limit_order match, so the combined transition should verify unchanged.
#[test]
fn mainnet_tx_5f6d0d5b_claim_deposit_and_limit_order_shape() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"live-mainnet-5f6d-lock");
    let (ickb_logic, limit_order, owned_owner, dao, xudt) =
        ickb_logic_limit_order_owned_owner_dao_and_xudt_scripts(&mut context);
    let withdraw_header = rpc_header(
        "0x11b4ad3",
        "0x50d0501003531",
        "0x8b3e331079c70158fafaada83c3c2a00282092094f7e590900477a3182df3507",
    );
    let deposit_header = rpc_header(
        "0x102d943",
        "0x5f301630030fa",
        "0xbd51139d8b84a65574ecbf4c52ca290057c2dd0e1ef38e0800d5521830ef3407",
    );

    let spec = TxSpec {
        inputs: vec![
            at("0xd48e2f9ec9dd567b6d77f7e25bcd0f0cbfca6444240f9d4720d2bbc61cd99e4a", 1, live("0xce777783bd6", &user_lock, None, "0x")),
            at(
                "0x1a8e8a211427f1ac9d07f380638acc915ceebb62d9dfdae2e29529f19bf045cd",
                2,
                live("0x3663a5200", &user_lock, Some(&xudt), "0x85026831280500000000000000000000"),
            ),
            In {
                header: Some(withdraw_header.clone()),
                since: u64_from_hex("0x2005f30163003532"),
                ..at(
                    "0x1a8e8a211427f1ac9d07f380638acc915ceebb62d9dfdae2e29529f19bf045cd",
                    0,
                    live("0xab4adefc030", &owned_owner, Some(&dao), "0x43d9020100000000"),
                )
            },
            at(
                "0x1a8e8a211427f1ac9d07f380638acc915ceebb62d9dfdae2e29529f19bf045cd",
                1,
                live("0x248202200", &user_lock, Some(&owned_owner), "0xffffffff"),
            ),
            at(
                "0xf8aed34a7b086cc50b92bbb292045da54473d1c5e50b0937788f6b84adb22170",
                4,
                live(
                    "0x710eb256272",
                    &limit_order,
                    Some(&xudt),
                    "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000ffffffff0000c16ff286230054043b34e65b2a000000000000000000000000000000000021",
                ),
            ),
        ],
        outputs: vec![
            live(
                "0xfe564f2176",
                &limit_order,
                Some(&xudt),
                "0x00c002da17050000000000000000000001000000f8aed34a7b086cc50b92bbb292045da54473d1c5e50b0937788f6b84adb22170030000000000c16ff286230054043b34e65b2a000000000000000000000000000000000021",
            )
            .into(),
            live("0xad543dd639c", &ickb_logic, Some(&dao), "0x0000000000000000").into(),
            live("0x277cf2a00", &user_lock, Some(&ickb_logic), "0x010000009c511b5bd30a0000").into(),
            live("0x3663a5200", &user_lock, Some(&xudt), "0x85426557100000000000000000000000").into(),
            live("0x12f6704164db", &user_lock, None, "0x").into(),
        ],
        header_deps: vec![withdraw_header, deposit_header],
    };

    let tx = replay(&mut context, &spec, vec![Bytes::new(), Bytes::new(), header_dep_index_witness(1)]);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("live mainnet claim plus fresh deposit and limit_order shape should replay locally");
}
