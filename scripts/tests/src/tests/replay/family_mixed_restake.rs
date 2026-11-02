use super::*;

/// The live mixed restake: one owned cell claimed and one fresh deposit withdrawn into a new owned
/// cell, whose owner cell goes to `fresh_owner`; `funding` pays and `owner` held the claim.
fn mixed_restake(
    (ickb_logic, limit_order, owned_owner, dao, xudt): &(Script, Script, Script, Script, Script),
    funding: &Script,
    owner: &Script,
    fresh_owner: &Script,
) -> TxSpec {
    let fresh_deposit_header = rpc_header(
        "0xde6f65",
        "0x708057c0024e7",
        "0x4351b61319e1e74e7f7d3934507928000be3eb6776bd030700ae793b9ee1fa08",
    );
    let prior_withdraw_header = rpc_header(
        "0x1010888",
        "0x708033f0029d3",
        "0x4e9860eb12aaa751dd7a1bceb8042900226480f5e0cef707003d1ea85adaf508",
    );
    let prior_deposit_header = rpc_header(
        "0xde6edb",
        "0x70804f20024e7",
        "0x649a19ed21d6e74efb0e50024e792800f59bcd9aabb90307001ce5afcee0fa08",
    );

    let funding_tx = "0x8d79267a9d44e2b0fd5bfa6fda9ce3045b8ed45093f2665a8c7515f4eb8afe03";
    let prior_tx = "0x89b6b3ee2c8ab70e4d50c93fbee151f693a95506a5c4904877a7b39969dbd1bf";
    TxSpec {
        inputs: vec![
            In {
                header: Some(fresh_deposit_header.clone()),
                ..at(
                    "0x61e9b3ef505cd54a32c531b7c155b3b332363494bb62fc8a8a60d333fc32227e",
                    1,
                    live("0xa5e67c62b2c", ickb_logic, Some(dao), "0x0000000000000000"),
                )
            },
            at(funding_tx, 3, live("0x13b71a4e3b", funding, None, "0x")),
            at(funding_tx, 2, live("0x3663a5200", funding, Some(xudt), "0xca482089660200000000000000000000")),
            In {
                header: Some(prior_withdraw_header.clone()),
                since: u64_from_hex("0x20070804f20029d3"),
                ..at(prior_tx, 0, live("0xa5e673651b7", owned_owner, Some(dao), "0xdb6ede0000000000"))
            },
            at(prior_tx, 2, live("0x248202200", owner, Some(owned_owner), "0xfeffffff")),
            at(
                prior_tx,
                1,
                live(
                    "0x2f9ea8dd8e2c",
                    limit_order,
                    Some(xudt),
                    "0xfc643f14592300000000000000000000010000001594bf3a65929bf5ee706f3c34e5c92d993adc405e58c3934661aeaa4e19264704000000000000000000000000000000000000000000c16ff28623008cb96cb2f30b290021",
                ),
            ),
        ],
        outputs: vec![
            live("0xa5e67c62b2c", owned_owner, Some(dao), "0x656fde0000000000").into(),
            live(
                "0x3a20c2ac1968",
                limit_order,
                Some(xudt),
                "0xe2755fb7401a00000000000000000000010000001594bf3a65929bf5ee706f3c34e5c92d993adc405e58c3934661aeaa4e19264704000000000000000000000000000000000000000000c16ff28623008cb96cb2f30b290021",
            )
            .into(),
            live("0x248202200", fresh_owner, Some(owned_owner), "0xfeffffff").into(),
            live("0x3663a5200", funding, Some(xudt), "0x9a569097660200000000000000000000").into(),
            live("0x13b55a06cc", funding, None, "0x").into(),
        ],
        header_deps: vec![prior_withdraw_header, prior_deposit_header, fresh_deposit_header],
    }
}

// Replays the live mixed restake family, but reassigns the fresh owner cell to the attacker, so the later phase2 claim should still follow the stolen branch and succeed.
#[test]
fn weak_live_mixed_restake_can_reassign_later_phase2_claimants() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"user");
    let attacker_lock = named_always_success_lock(&mut context, b"attacker");
    let scripts = ickb_logic_limit_order_owned_owner_dao_and_xudt_scripts(&mut context);
    let (_, limit_order, _, _, xudt) = &scripts;

    let first = mixed_restake(&scripts, &user_lock, &user_lock, &attacker_lock);
    let first_tx = replay(
        &mut context,
        &first,
        vec![
            bytes_from_hex("0x10000000100000001000000010000000"),
            bytes_from_hex("0x55000000100000005500000055000000410000006b2a2e733b7a7be2775158985e05b2ddbbdc67f17877fa3ad62ca6e60d3fc5381b0335e8963f21cb5d62c103590ff51ff9ce4e9fb7e0562cc84314e8b51f09c101"),
            bytes_from_hex("0x10000000100000001000000010000000"),
            bytes_from_hex("0x1c00000010000000100000001c000000080000000100000000000000"),
        ],
    );
    context
        .verify(&first_tx, MAX_CYCLES)
        .expect("weak mixed restake should reassign the fresh owner output");

    let fresh_withdraw_header = rpc_header(
        "0x1010a3f",
        "0x70804f60029d3",
        "0xc43febe9f4cca7518136b1a4bf04290006ebbfbbdbdaf7070088c386cadcf508",
    );
    let first_hash = first_tx.hash();
    let later_tx = "0x982bb58af19dd00832ce3e6a22f3df1713ad849c52845fdda60f47739f8f1253";
    let second = TxSpec {
        inputs: vec![
            at(later_tx, 2, live("0x30e25b5441", &user_lock, None, "0x")),
            at(later_tx, 1, live("0x3663a5200", &user_lock, Some(xudt), "0x276736ac670b00000000000000000000")),
            In {
                out_point: OutPoint::new(first_hash.clone(), 0),
                header: Some(fresh_withdraw_header.clone()),
                since: u64_from_hex("0x200708057c0029d3"),
                ..In::new(first.outputs[0].cell.clone())
            },
            In { out_point: OutPoint::new(first_hash, 2), ..In::new(first.outputs[2].cell.clone()) },
            at(
                later_tx,
                0,
                live(
                    "0x4487f6435e53",
                    limit_order,
                    Some(xudt),
                    "0x5565b9a23f1100000000000000000000010000001594bf3a65929bf5ee706f3c34e5c92d993adc405e58c3934661aeaa4e19264704000000000000000000000000000000000000000000c16ff28623008cb96cb2f30b290021",
                ),
            ),
        ],
        outputs: vec![
            live(
                "0x4f0a109fcf57",
                limit_order,
                Some(xudt),
                "0x2da55e45270800000000000000000000010000001594bf3a65929bf5ee706f3c34e5c92d993adc405e58c3934661aeaa4e19264704000000000000000000000000000000000000000000c16ff28623008cb96cb2f30b290021",
            )
            .into(),
            live("0x3663a5200", &attacker_lock, Some(xudt), "0x4f279109801400000000000000000000").into(),
            live("0x3329ed7100", &attacker_lock, None, "0x").into(),
        ],
        header_deps: vec![fresh_withdraw_header, first.header_deps[2].clone()],
    };

    let second_tx = replay(
        &mut context,
        &second,
        vec![
            bytes_from_hex("0x5500000010000000550000005500000041000000b884bdb39fe8a0954905a8b1183c4f59c42f8bddf7dc7c4b93785ee5c7a9985e01c9820e3c28cda06f8ad1ab5e6ba546bf5064f24130bd322d6d297d76fd8c3d01"),
            bytes_from_hex("0x10000000100000001000000010000000"),
            bytes_from_hex("0x1c00000010000000100000001c000000080000000100000000000000"),
        ],
    );
    context
        .verify(&second_tx, MAX_CYCLES)
        .expect("reassigned fresh owner output should enable the later phase2 claim");
}

// Replays the same mixed restake family with a signed owner group, then rewrites the fresh owner output to an attacker lock, so the tampered replay should fail on sighash binding.
#[test]
fn sighash_live_mixed_restake_binds_fresh_owner_output() {
    let mut context = Context::default();
    let (privkey, strong_lock, secp_data_dep) = secp_lock(&mut context);
    let weak_lock = named_always_success_lock(&mut context, b"weak-owner");
    let attacker_lock = named_always_success_lock(&mut context, b"attacker");
    let scripts = ickb_logic_limit_order_owned_owner_dao_and_xudt_scripts(&mut context);

    let spec = mixed_restake(&scripts, &strong_lock, &weak_lock, &weak_lock);
    let tx = replay(
        &mut context,
        &spec,
        vec![
            Bytes::new(),
            empty_witness(),
            Bytes::new(),
            bytes_from_hex("0x1c00000010000000100000001c000000080000000100000000000000"),
        ],
    );
    let tx = sign_tx_by_input_group(tx.as_advanced_builder().cell_dep(secp_data_dep).build(), &privkey, 1, 2);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("one strong input group should still verify in the live mixed restake shape");

    let mut outputs: Vec<CellOutput> = tx.outputs().into_iter().collect();
    outputs[2] = cell(u64_from_hex("0x248202200"), &attacker_lock, Some(&scripts.2));
    let tampered_tx = tx.as_advanced_builder().set_outputs(outputs).build();
    fail(&context, &tampered_tx, ERROR_SECP256K1_BLAKE160_SIGHASH_ALL);
}
