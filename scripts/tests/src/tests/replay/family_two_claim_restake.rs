use super::*;

// Replays the live two-claim restake family, but reassigns the fresh owner pair to the attacker, so the later wider phase2 claim should still validate on the stolen path.
#[test]
fn weak_two_claim_live_restake_can_reassign_later_phase2_claimants() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"user");
    let attacker_lock = named_always_success_lock(&mut context, b"attacker");
    let (ickb_logic, limit_order, owned_owner, dao, xudt) =
        ickb_logic_limit_order_owned_owner_dao_and_xudt_scripts(&mut context);

    let fresh_deposit_header = rpc_header(
        "0xe35b0c",
        "0x708068b00259a",
        "0xe895f69a88f84b4fda777e8e518d280070b848fbd65c260700a933035c24f108",
    );
    let prior_withdraw_header = rpc_header(
        "0x1010221",
        "0x70803e00029d2",
        "0x4ea3fdead627a751fd5843469f0429008498e3e307a2f70700a3d7f546d1f508",
    );
    let prior_deposit_header = rpc_header(
        "0xde687f",
        "0x708059e0024e6",
        "0x5b1bddaec554e74efcad531d34792800d43522f7ee8c030700112419e9ddfa08",
    );

    let deposit_input = out_point_from_hex(
        "0xcb91164d9075add730a748150fdd2e32a750cef5051dafed33d1c2aa2cde6806",
        1,
    );
    context.create_cell_with_out_point(
        deposit_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0xa6386d7249d").pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        bytes_from_hex("0x0000000000000000"),
    );
    link_cell_to_header(&mut context, &deposit_input, &fresh_deposit_header);

    let change_input = out_point_from_hex(
        "0xb07711b92aa8c0028bbdd50a23e332f6efb1c77a735f3ac6090f6d12f581bd7c",
        6,
    );
    context.create_cell_with_out_point(
        change_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x13bbb34e26").pack())
            .lock(user_lock.clone())
            .build(),
        Bytes::new(),
    );

    let udt_input = out_point_from_hex(
        "0xb07711b92aa8c0028bbdd50a23e332f6efb1c77a735f3ac6090f6d12f581bd7c",
        5,
    );
    context.create_cell_with_out_point(
        udt_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x3663a5200").pack())
            .lock(user_lock.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex("0xa0bfc96d660200000000000000000000"),
    );

    let owned_input_1 = out_point_from_hex(
        "0xb07711b92aa8c0028bbdd50a23e332f6efb1c77a735f3ac6090f6d12f581bd7c",
        1,
    );
    let owned_input_0 = out_point_from_hex(
        "0xb07711b92aa8c0028bbdd50a23e332f6efb1c77a735f3ac6090f6d12f581bd7c",
        0,
    );
    context.create_cell_with_out_point(
        owned_input_1.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0xa5e609441a1").pack())
            .lock(owned_owner.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        bytes_from_hex("0x7f68de0000000000"),
    );
    context.create_cell_with_out_point(
        owned_input_0.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0xa5e609441a1").pack())
            .lock(owned_owner.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        bytes_from_hex("0x7f68de0000000000"),
    );
    link_cell_to_header(&mut context, &owned_input_1, &prior_withdraw_header);
    link_cell_to_header(&mut context, &owned_input_0, &prior_withdraw_header);

    let owner_input_4 = out_point_from_hex(
        "0xb07711b92aa8c0028bbdd50a23e332f6efb1c77a735f3ac6090f6d12f581bd7c",
        4,
    );
    let owner_input_3 = out_point_from_hex(
        "0xb07711b92aa8c0028bbdd50a23e332f6efb1c77a735f3ac6090f6d12f581bd7c",
        3,
    );
    context.create_cell_with_out_point(
        owner_input_4.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x248202200").pack())
            .lock(user_lock.clone())
            .type_(Some(owned_owner.clone()).pack())
            .build(),
        bytes_from_hex("0xfdffffff"),
    );
    context.create_cell_with_out_point(
        owner_input_3.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x248202200").pack())
            .lock(user_lock.clone())
            .type_(Some(owned_owner.clone()).pack())
            .build(),
        bytes_from_hex("0xfdffffff"),
    );

    let order_input = out_point_from_hex(
        "0xb07711b92aa8c0028bbdd50a23e332f6efb1c77a735f3ac6090f6d12f581bd7c",
        2,
    );
    context.create_cell_with_out_point(
        order_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x10186de6e79f").pack())
            .lock(limit_order.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex(
            "0x7995e51aa23e00000000000000000000010000001594bf3a65929bf5ee706f3c34e5c92d993adc405e58c3934661aeaa4e19264704000000000000000000000000000000000000000000c16ff28623008cb96cb2f30b290021",
        ),
    );

    context.insert_header(prior_deposit_header.clone());

    let first_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit_input).build())
        .input(CellInput::new_builder().previous_output(change_input).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .input(
            CellInput::new_builder()
                .previous_output(owned_input_1)
                .since(u64_from_hex("0x200708059e0029d2").pack())
                .build(),
        )
        .input(
            CellInput::new_builder()
                .previous_output(owned_input_0)
                .since(u64_from_hex("0x200708059e0029d2").pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(owner_input_4).build())
        .input(CellInput::new_builder().previous_output(owner_input_3).build())
        .input(CellInput::new_builder().previous_output(order_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xa6386d7249d").pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x251c9490476f").pack())
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
                .capacity(u64_from_hex("0x160032266c").pack())
                .lock(user_lock.clone())
                .build(),
        ])
        .outputs_data(
            vec![
                bytes_from_hex("0x0c5be30000000000"),
                bytes_from_hex(
                    "0xb0905b6c712c00000000000000000000010000001594bf3a65929bf5ee706f3c34e5c92d993adc405e58c3934661aeaa4e19264704000000000000000000000000000000000000000000c16ff28623008cb96cb2f30b290021",
                ),
                bytes_from_hex("0xfeffffff"),
                bytes_from_hex("0xa9dfe3cd7e0b00000000000000000000"),
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
                bytes_from_hex("0x5500000010000000550000005500000041000000eff31644579c4b2c86d669a691385f9b4725d5a595e9ab73f87ec12e1aeba23a605146dc166dc44af64d34f5e4bde8ba29577a0fb004381de88e1510ccebff9d00"),
                bytes_from_hex("0x10000000100000001000000010000000"),
                bytes_from_hex("0x1c00000010000000100000001c000000080000000100000000000000"),
                bytes_from_hex("0x1c00000010000000100000001c000000080000000100000000000000"),
            ]
            .pack(),
        )
        .build();

    let first_tx = context.complete_tx(first_tx);
    context
        .verify_tx(&first_tx, MAX_CYCLES)
        .expect("weak two-claim restake should reassign the fresh owner output");

    let fresh_withdraw_header = rpc_header(
        "0x10103e8",
        "0x70805a70029d2",
        "0xd40d0661fe4ba7514c6aad5ca6042900ef8b210b50aef7070068e414c7d3f508",
    );
    let first_hash = first_tx.hash();
    let later_change_input = OutPoint::new(first_hash.clone(), 4);
    let later_udt_input = OutPoint::new(first_hash.clone(), 3);
    let fresh_owned = OutPoint::new(first_hash.clone(), 0);
    let fresh_owner = OutPoint::new(first_hash.clone(), 2);
    let later_order_input = OutPoint::new(first_hash, 1);
    context.create_cell_with_out_point(
        fresh_owned.clone(),
        first_tx.outputs().get(0).expect("fresh owned output"),
        bytes_from_hex("0x0c5be30000000000"),
    );
    context.create_cell_with_out_point(
        fresh_owner.clone(),
        first_tx.outputs().get(2).expect("fresh owner output"),
        bytes_from_hex("0xfeffffff"),
    );
    context.create_cell_with_out_point(
        later_udt_input.clone(),
        first_tx.outputs().get(3).expect("later udt output"),
        bytes_from_hex("0xa9dfe3cd7e0b00000000000000000000"),
    );
    context.create_cell_with_out_point(
        later_change_input.clone(),
        first_tx.outputs().get(4).expect("later change output"),
        Bytes::new(),
    );
    context.create_cell_with_out_point(
        later_order_input.clone(),
        first_tx.outputs().get(1).expect("later order output"),
        bytes_from_hex(
            "0xb0905b6c712c00000000000000000000010000001594bf3a65929bf5ee706f3c34e5c92d993adc405e58c3934661aeaa4e19264704000000000000000000000000000000000000000000c16ff28623008cb96cb2f30b290021",
        ),
    );
    link_cell_to_header(&mut context, &fresh_owned, &fresh_withdraw_header);

    let second_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(later_change_input).build())
        .input(CellInput::new_builder().previous_output(later_udt_input).build())
        .input(
            CellInput::new_builder()
                .previous_output(fresh_owned)
                .since(u64_from_hex("0x200708068b0029d2").pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(fresh_owner).build())
        .input(CellInput::new_builder().previous_output(later_order_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x2f83c2399248").pack())
                .lock(limit_order)
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x3663a5200").pack())
                .lock(attacker_lock.clone())
                .type_(Some(xudt.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x332dfe9760").pack())
                .lock(attacker_lock)
                .build(),
        ])
        .outputs_data(
            vec![
                bytes_from_hex(
                    "0xdd5ad75c702300000000000000000000010000001594bf3a65929bf5ee706f3c34e5c92d993adc405e58c3934661aeaa4e19264704000000000000000000000000000000000000000000c16ff28623008cb96cb2f30b290021",
                ),
                bytes_from_hex("0x7c1568dd7f1400000000000000000000"),
                Bytes::new(),
            ]
            .pack(),
        )
        .header_dep(fresh_withdraw_header.hash())
        .header_dep(fresh_deposit_header.hash())
        .witnesses(
            vec![
                bytes_from_hex("0x5500000010000000550000005500000041000000fc411ffb5ded2280a62a31fbd1750bca07b40c069496a3a9acec2f0664f980ea585f3774a887a5d7dfa3c06e65f9a9ae63be34b964fcb028dd1be0a2040e69a400"),
                bytes_from_hex("0x10000000100000001000000010000000"),
                bytes_from_hex("0x1c00000010000000100000001c000000080000000100000000000000"),
            ]
            .pack(),
        )
        .build();

    let second_tx = context.complete_tx(second_tx);
    context
        .verify_tx(&second_tx, MAX_CYCLES)
        .expect("reassigned fresh owner output should enable the wider phase2 claim");
}

// Replays the same two-claim restake family with a signed owner group, then swaps a fresh owner output to an attacker lock, so the tampered replay should fail on sighash binding.
#[test]
fn sighash_two_claim_live_restake_binds_fresh_owner_output() {
    let mut context = Context::default();
    let (privkey, strong_lock, secp_data_dep) = secp_lock(&mut context);
    let weak_lock = named_always_success_lock(&mut context, b"weak-owner");
    let attacker_lock = named_always_success_lock(&mut context, b"attacker");
    let (ickb_logic, limit_order, owned_owner, dao, xudt) =
        ickb_logic_limit_order_owned_owner_dao_and_xudt_scripts(&mut context);

    let fresh_deposit_header = rpc_header(
        "0xe35b0c",
        "0x708068b00259a",
        "0xe895f69a88f84b4fda777e8e518d280070b848fbd65c260700a933035c24f108",
    );
    let prior_withdraw_header = rpc_header(
        "0x1010221",
        "0x70803e00029d2",
        "0x4ea3fdead627a751fd5843469f0429008498e3e307a2f70700a3d7f546d1f508",
    );
    let prior_deposit_header = rpc_header(
        "0xde687f",
        "0x708059e0024e6",
        "0x5b1bddaec554e74efcad531d34792800d43522f7ee8c030700112419e9ddfa08",
    );

    let deposit_input = out_point_from_hex(
        "0xcb91164d9075add730a748150fdd2e32a750cef5051dafed33d1c2aa2cde6806",
        1,
    );
    context.create_cell_with_out_point(
        deposit_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0xa6386d7249d").pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        bytes_from_hex("0x0000000000000000"),
    );
    link_cell_to_header(&mut context, &deposit_input, &fresh_deposit_header);

    let change_input = out_point_from_hex(
        "0xb07711b92aa8c0028bbdd50a23e332f6efb1c77a735f3ac6090f6d12f581bd7c",
        6,
    );
    context.create_cell_with_out_point(
        change_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x13bbb34e26").pack())
            .lock(strong_lock.clone())
            .build(),
        Bytes::new(),
    );

    let udt_input = out_point_from_hex(
        "0xb07711b92aa8c0028bbdd50a23e332f6efb1c77a735f3ac6090f6d12f581bd7c",
        5,
    );
    context.create_cell_with_out_point(
        udt_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x3663a5200").pack())
            .lock(strong_lock.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex("0xa0bfc96d660200000000000000000000"),
    );

    let owned_input_1 = out_point_from_hex(
        "0xb07711b92aa8c0028bbdd50a23e332f6efb1c77a735f3ac6090f6d12f581bd7c",
        1,
    );
    let owned_input_0 = out_point_from_hex(
        "0xb07711b92aa8c0028bbdd50a23e332f6efb1c77a735f3ac6090f6d12f581bd7c",
        0,
    );
    context.create_cell_with_out_point(
        owned_input_1.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0xa5e609441a1").pack())
            .lock(owned_owner.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        bytes_from_hex("0x7f68de0000000000"),
    );
    context.create_cell_with_out_point(
        owned_input_0.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0xa5e609441a1").pack())
            .lock(owned_owner.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        bytes_from_hex("0x7f68de0000000000"),
    );
    link_cell_to_header(&mut context, &owned_input_1, &prior_withdraw_header);
    link_cell_to_header(&mut context, &owned_input_0, &prior_withdraw_header);

    let owner_input_4 = out_point_from_hex(
        "0xb07711b92aa8c0028bbdd50a23e332f6efb1c77a735f3ac6090f6d12f581bd7c",
        4,
    );
    let owner_input_3 = out_point_from_hex(
        "0xb07711b92aa8c0028bbdd50a23e332f6efb1c77a735f3ac6090f6d12f581bd7c",
        3,
    );
    context.create_cell_with_out_point(
        owner_input_4.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x248202200").pack())
            .lock(weak_lock.clone())
            .type_(Some(owned_owner.clone()).pack())
            .build(),
        bytes_from_hex("0xfdffffff"),
    );
    context.create_cell_with_out_point(
        owner_input_3.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x248202200").pack())
            .lock(weak_lock.clone())
            .type_(Some(owned_owner.clone()).pack())
            .build(),
        bytes_from_hex("0xfdffffff"),
    );

    let order_input = out_point_from_hex(
        "0xb07711b92aa8c0028bbdd50a23e332f6efb1c77a735f3ac6090f6d12f581bd7c",
        2,
    );
    context.create_cell_with_out_point(
        order_input.clone(),
        CellOutput::new_builder()
            .capacity(u64_from_hex("0x10186de6e79f").pack())
            .lock(limit_order.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        bytes_from_hex(
            "0x7995e51aa23e00000000000000000000010000001594bf3a65929bf5ee706f3c34e5c92d993adc405e58c3934661aeaa4e19264704000000000000000000000000000000000000000000c16ff28623008cb96cb2f30b290021",
        ),
    );

    context.insert_header(prior_deposit_header.clone());

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit_input).build())
        .input(CellInput::new_builder().previous_output(change_input).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .input(
            CellInput::new_builder()
                .previous_output(owned_input_1)
                .since(u64_from_hex("0x200708059e0029d2").pack())
                .build(),
        )
        .input(
            CellInput::new_builder()
                .previous_output(owned_input_0)
                .since(u64_from_hex("0x200708059e0029d2").pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(owner_input_4).build())
        .input(CellInput::new_builder().previous_output(owner_input_3).build())
        .input(CellInput::new_builder().previous_output(order_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(u64_from_hex("0xa6386d7249d").pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(u64_from_hex("0x251c9490476f").pack())
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
                .capacity(u64_from_hex("0x160032266c").pack())
                .lock(strong_lock.clone())
                .build(),
        ])
        .outputs_data(
            vec![
                bytes_from_hex("0x0c5be30000000000"),
                bytes_from_hex(
                    "0xb0905b6c712c00000000000000000000010000001594bf3a65929bf5ee706f3c34e5c92d993adc405e58c3934661aeaa4e19264704000000000000000000000000000000000000000000c16ff28623008cb96cb2f30b290021",
                ),
                bytes_from_hex("0xfeffffff"),
                bytes_from_hex("0xa9dfe3cd7e0b00000000000000000000"),
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
                bytes_from_hex("0x1c00000010000000100000001c000000080000000100000000000000"),
            ]
            .pack(),
        )
        .cell_dep(secp_data_dep)
        .build();

    let tx = sign_tx_by_input_group(context.complete_tx(tx), &privkey, 1, 2);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("one strong input group should still verify in the two-claim restake");

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
