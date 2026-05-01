use super::*;

// Replays the live mixed restake family, but reassigns the fresh owner cell to the attacker, so the later phase2 claim should still follow the stolen branch and succeed.
#[test]
fn weak_live_mixed_restake_can_reassign_later_phase2_claimants() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"user");
    let attacker_lock = named_always_success_lock(&mut context, b"attacker");
    let (ickb_logic, limit_order, owned_owner, dao, xudt) =
        ickb_logic_limit_order_owned_owner_dao_and_xudt_scripts(&mut context);

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

    let deposit_input = out_point_from_hex(
        "0x61e9b3ef505cd54a32c531b7c155b3b332363494bb62fc8a8a60d333fc32227e",
        1,
    );
    context.create_cell_with_out_point(
        deposit_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0xa5e67c62b2c").pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        bytes_from_hex("0x0000000000000000"),
    );
    link_cell_to_header(&mut context, &deposit_input, &fresh_deposit_header);

    let change_input = out_point_from_hex(
        "0x8d79267a9d44e2b0fd5bfa6fda9ce3045b8ed45093f2665a8c7515f4eb8afe03",
        3,
    );
    context.create_cell_with_out_point(
        change_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x13b71a4e3b").pack())
            .lock(user_lock.clone())
            .build(),
        Bytes::new(),
    );

    let udt_input = out_point_from_hex(
        "0x8d79267a9d44e2b0fd5bfa6fda9ce3045b8ed45093f2665a8c7515f4eb8afe03",
        2,
    );
    context.create_cell_with_out_point(
        udt_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x3663a5200").pack())
            .lock(user_lock.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex("0xca482089660200000000000000000000"),
    );

    let owned_input = out_point_from_hex(
        "0x89b6b3ee2c8ab70e4d50c93fbee151f693a95506a5c4904877a7b39969dbd1bf",
        0,
    );
    context.create_cell_with_out_point(
        owned_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0xa5e673651b7").pack())
            .lock(owned_owner.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        bytes_from_hex("0xdb6ede0000000000"),
    );
    link_cell_to_header(&mut context, &owned_input, &prior_withdraw_header);

    let owner_input = out_point_from_hex(
        "0x89b6b3ee2c8ab70e4d50c93fbee151f693a95506a5c4904877a7b39969dbd1bf",
        2,
    );
    context.create_cell_with_out_point(
        owner_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x248202200").pack())
            .lock(user_lock.clone())
            .type_(Some(owned_owner.clone()).pack())
            .build(),
        bytes_from_hex("0xfeffffff"),
    );

    let order_input = out_point_from_hex(
        "0x89b6b3ee2c8ab70e4d50c93fbee151f693a95506a5c4904877a7b39969dbd1bf",
        1,
    );
    context.create_cell_with_out_point(
        order_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x2f9ea8dd8e2c").pack())
            .lock(limit_order.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex(
            "0xfc643f14592300000000000000000000010000001594bf3a65929bf5ee706f3c34e5c92d993adc405e58c3934661aeaa4e19264704000000000000000000000000000000000000000000c16ff28623008cb96cb2f30b290021",
        ),
    );

    context.insert_header(prior_deposit_header.clone());

    let first_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit_input).build())
        .input(CellInput::new_builder().previous_output(change_input).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .input(
            CellInput::new_builder()
                .previous_output(owned_input)
                .since(u64_from_hex("0x20070804f20029d3").pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(owner_input).build())
        .input(CellInput::new_builder().previous_output(order_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xa5e67c62b2c").pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x3a20c2ac1968").pack())
                .lock(limit_order.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x248202200").pack())
                .lock(attacker_lock.clone())
                .type_(Some(owned_owner.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x3663a5200").pack())
                .lock(user_lock.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x13b55a06cc").pack())
                .lock(user_lock.clone())
                .build(),
        ])
        .outputs_data(
            vec![
                bytes_from_hex("0x656fde0000000000"),
                bytes_from_hex(
                    "0xe2755fb7401a00000000000000000000010000001594bf3a65929bf5ee706f3c34e5c92d993adc405e58c3934661aeaa4e19264704000000000000000000000000000000000000000000c16ff28623008cb96cb2f30b290021",
                ),
                bytes_from_hex("0xfeffffff"),
                bytes_from_hex("0x9a569097660200000000000000000000"),
                Bytes::new(),
            ]
            .pack(),
        )
        .header_dep(prior_withdraw_header.hash())
        .header_dep(prior_deposit_header.hash())
        .header_dep(fresh_deposit_header.hash())
        .witnesses(
            vec![
                bytes_from_hex("0x10000000100000001000000010000000"),
                bytes_from_hex("0x55000000100000005500000055000000410000006b2a2e733b7a7be2775158985e05b2ddbbdc67f17877fa3ad62ca6e60d3fc5381b0335e8963f21cb5d62c103590ff51ff9ce4e9fb7e0562cc84314e8b51f09c101"),
                bytes_from_hex("0x10000000100000001000000010000000"),
                bytes_from_hex("0x1c00000010000000100000001c000000080000000100000000000000"),
            ]
            .pack(),
        )
        .build();

    let first_tx = context.complete_tx(first_tx);
    context
        .verify_tx(&first_tx, MAX_CYCLES)
        .expect("weak mixed restake should reassign the fresh owner output");

    let fresh_withdraw_header = rpc_header(
        "0x1010a3f",
        "0x70804f60029d3",
        "0xc43febe9f4cca7518136b1a4bf04290006ebbfbbdbdaf7070088c386cadcf508",
    );
    let first_hash = first_tx.hash();
    let fresh_owned = OutPoint::new(first_hash.clone(), 0);
    let fresh_owner = OutPoint::new(first_hash, 2);
    context.create_cell_with_out_point(
        fresh_owned.clone(),
        first_tx.outputs().get(0).expect("fresh owned output"),
        bytes_from_hex("0x656fde0000000000"),
    );
    context.create_cell_with_out_point(
        fresh_owner.clone(),
        first_tx.outputs().get(2).expect("fresh owner output"),
        bytes_from_hex("0xfeffffff"),
    );
    link_cell_to_header(&mut context, &fresh_owned, &fresh_withdraw_header);

    let later_order_input = out_point_from_hex(
        "0x982bb58af19dd00832ce3e6a22f3df1713ad849c52845fdda60f47739f8f1253",
        0,
    );
    context.create_cell_with_out_point(
        later_order_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x4487f6435e53").pack())
            .lock(limit_order.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex(
            "0x5565b9a23f1100000000000000000000010000001594bf3a65929bf5ee706f3c34e5c92d993adc405e58c3934661aeaa4e19264704000000000000000000000000000000000000000000c16ff28623008cb96cb2f30b290021",
        ),
    );

    let later_udt_input = out_point_from_hex(
        "0x982bb58af19dd00832ce3e6a22f3df1713ad849c52845fdda60f47739f8f1253",
        1,
    );
    context.create_cell_with_out_point(
        later_udt_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x3663a5200").pack())
            .lock(user_lock.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex("0x276736ac670b00000000000000000000"),
    );

    let later_change_input = out_point_from_hex(
        "0x982bb58af19dd00832ce3e6a22f3df1713ad849c52845fdda60f47739f8f1253",
        2,
    );
    context.create_cell_with_out_point(
        later_change_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x30e25b5441").pack())
            .lock(user_lock)
            .build(),
        Bytes::new(),
    );

    let second_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(later_change_input).build())
        .input(CellInput::new_builder().previous_output(later_udt_input).build())
        .input(
            CellInput::new_builder()
                .previous_output(fresh_owned)
                .since(u64_from_hex("0x200708057c0029d3").pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(fresh_owner).build())
        .input(CellInput::new_builder().previous_output(later_order_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x4f0a109fcf57").pack())
                .lock(limit_order)
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x3663a5200").pack())
                .lock(attacker_lock.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x3329ed7100").pack())
                .lock(attacker_lock)
                .build(),
        ])
        .outputs_data(
            vec![
                bytes_from_hex(
                    "0x2da55e45270800000000000000000000010000001594bf3a65929bf5ee706f3c34e5c92d993adc405e58c3934661aeaa4e19264704000000000000000000000000000000000000000000c16ff28623008cb96cb2f30b290021",
                ),
                bytes_from_hex("0x4f279109801400000000000000000000"),
                Bytes::new(),
            ]
            .pack(),
        )
        .header_dep(fresh_withdraw_header.hash())
        .header_dep(fresh_deposit_header.hash())
        .witnesses(
            vec![
                bytes_from_hex("0x5500000010000000550000005500000041000000b884bdb39fe8a0954905a8b1183c4f59c42f8bddf7dc7c4b93785ee5c7a9985e01c9820e3c28cda06f8ad1ab5e6ba546bf5064f24130bd322d6d297d76fd8c3d01"),
                bytes_from_hex("0x10000000100000001000000010000000"),
                bytes_from_hex("0x1c00000010000000100000001c000000080000000100000000000000"),
            ]
            .pack(),
        )
        .build();

    let second_tx = context.complete_tx(second_tx);
    context
        .verify_tx(&second_tx, MAX_CYCLES)
        .expect("reassigned fresh owner output should enable the later phase2 claim");
}

// Replays the same mixed restake family with a signed owner group, then rewrites the fresh owner output to an attacker lock, so the tampered replay should fail on sighash binding.
#[test]
fn sighash_live_mixed_restake_binds_fresh_owner_output() {
    let mut context = Context::default();
    let (privkey, strong_lock, secp_data_dep) = secp_lock(&mut context);
    let weak_lock = named_always_success_lock(&mut context, b"weak-owner");
    let attacker_lock = named_always_success_lock(&mut context, b"attacker");
    let (ickb_logic, limit_order, owned_owner, dao, xudt) =
        ickb_logic_limit_order_owned_owner_dao_and_xudt_scripts(&mut context);

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

    let change_input = out_point_from_hex(
        "0x8d79267a9d44e2b0fd5bfa6fda9ce3045b8ed45093f2665a8c7515f4eb8afe03",
        3,
    );
    context.create_cell_with_out_point(
        change_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x13b71a4e3b").pack())
            .lock(strong_lock.clone())
            .build(),
        Bytes::new(),
    );
    let udt_input = out_point_from_hex(
        "0x8d79267a9d44e2b0fd5bfa6fda9ce3045b8ed45093f2665a8c7515f4eb8afe03",
        2,
    );
    context.create_cell_with_out_point(
        udt_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x3663a5200").pack())
            .lock(strong_lock.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex("0xca482089660200000000000000000000"),
    );
    let owner_input = out_point_from_hex(
        "0x89b6b3ee2c8ab70e4d50c93fbee151f693a95506a5c4904877a7b39969dbd1bf",
        2,
    );
    context.create_cell_with_out_point(
        owner_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x248202200").pack())
            .lock(weak_lock.clone())
            .type_(Some(owned_owner.clone()).pack())
            .build(),
        bytes_from_hex("0xfeffffff"),
    );

    let deposit_input = out_point_from_hex(
        "0x61e9b3ef505cd54a32c531b7c155b3b332363494bb62fc8a8a60d333fc32227e",
        1,
    );
    context.create_cell_with_out_point(
        deposit_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0xa5e67c62b2c").pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        bytes_from_hex("0x0000000000000000"),
    );
    link_cell_to_header(&mut context, &deposit_input, &fresh_deposit_header);

    let owned_input = out_point_from_hex(
        "0x89b6b3ee2c8ab70e4d50c93fbee151f693a95506a5c4904877a7b39969dbd1bf",
        0,
    );
    context.create_cell_with_out_point(
        owned_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0xa5e673651b7").pack())
            .lock(owned_owner.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        bytes_from_hex("0xdb6ede0000000000"),
    );
    link_cell_to_header(&mut context, &owned_input, &prior_withdraw_header);

    let order_input = out_point_from_hex(
        "0x89b6b3ee2c8ab70e4d50c93fbee151f693a95506a5c4904877a7b39969dbd1bf",
        1,
    );
    context.create_cell_with_out_point(
        order_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x2f9ea8dd8e2c").pack())
            .lock(limit_order.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex(
            "0xfc643f14592300000000000000000000010000001594bf3a65929bf5ee706f3c34e5c92d993adc405e58c3934661aeaa4e19264704000000000000000000000000000000000000000000c16ff28623008cb96cb2f30b290021",
        ),
    );

    context.insert_header(prior_deposit_header.clone());

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit_input).build())
        .input(CellInput::new_builder().previous_output(change_input).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .input(
            CellInput::new_builder()
                .previous_output(owned_input)
                .since(u64_from_hex("0x20070804f20029d3").pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(owner_input).build())
        .input(CellInput::new_builder().previous_output(order_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xa5e67c62b2c").pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x3a20c2ac1968").pack())
                .lock(limit_order.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x248202200").pack())
                .lock(weak_lock.clone())
                .type_(Some(owned_owner.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x3663a5200").pack())
                .lock(strong_lock.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x13b55a06cc").pack())
                .lock(strong_lock.clone())
                .build(),
        ])
        .outputs_data(
            vec![
                bytes_from_hex("0x656fde0000000000"),
                bytes_from_hex(
                    "0xe2755fb7401a00000000000000000000010000001594bf3a65929bf5ee706f3c34e5c92d993adc405e58c3934661aeaa4e19264704000000000000000000000000000000000000000000c16ff28623008cb96cb2f30b290021",
                ),
                bytes_from_hex("0xfeffffff"),
                bytes_from_hex("0x9a569097660200000000000000000000"),
                Bytes::new(),
            ]
            .pack(),
        )
        .header_dep(prior_withdraw_header.hash())
        .header_dep(prior_deposit_header.hash())
        .header_dep(fresh_deposit_header.hash())
        .witnesses(
            vec![
                Bytes::new(),
                empty_witness(),
                Bytes::new(),
                bytes_from_hex("0x1c00000010000000100000001c000000080000000100000000000000"),
            ]
            .pack(),
        )
        .cell_dep(secp_data_dep)
        .build();

    let tx = sign_tx_by_input_group(context.complete_tx(tx), &privkey, 1, 2);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("one strong input group should still verify in the live mixed restake shape");

    let tampered_tx = tx
        .as_advanced_builder()
        .set_outputs(vec![
            tx.outputs().get(0).expect("fresh owned output"),
            tx.outputs().get(1).expect("order output"),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x248202200").pack())
                .lock(attacker_lock)
                .type_(Some(owned_owner).pack())
                .build(),
            tx.outputs().get(3).expect("udt output"),
            tx.outputs().get(4).expect("change output"),
        ])
        .build();
    let err = context.verify_tx(&tampered_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_SECP256K1_BLAKE160_SIGHASH_ALL);
}
