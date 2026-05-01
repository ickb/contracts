use super::*;

// Even with weak locks, some crosswired layouts are still stopped by the DAO's same-index withdrawal rules before any later claim can rotate.
#[test]
fn crosswired_batch_is_blocked_by_dao_index_rules() {
    let mut context = Context::default();
    let user1_lock = named_always_success_lock(&mut context, b"user1");
    let user2_lock = named_always_success_lock(&mut context, b"user2");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let amount1 = 1_000 * SHANNONS;
    let amount2 = 1_100 * SHANNONS;
    let total1 = deposit_capacity(&ickb_logic, &dao, 8, amount1);
    let total2 = deposit_capacity(&ickb_logic, &dao, 8, amount2);
    let header1 = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);
    let header2 = gen_header(1555, GENESIS_AR as u64, 35, 1000, 1000);
    let deposit1 = context.create_cell(
        CellOutput::new_builder()
            .capacity(total1.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    let deposit2 = context.create_cell(
        CellOutput::new_builder()
            .capacity(total2.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    link_cell_to_header(&mut context, &deposit1, &header1);
    link_cell_to_header(&mut context, &deposit2, &header2);
    let udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(user1_lock.clone())
            .type_(Some(xudt).pack())
            .build(),
        udt_data(u128::from(amount1 + amount2)),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit1).build())
        .input(CellInput::new_builder().previous_output(deposit2).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(total1.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(user1_lock)
                .type_(Some(owned_owner.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(total2.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(user2_lock)
                .type_(Some(owned_owner).pack())
                .build(),
        ])
        .outputs_data(
            vec![
                withdrawal_request_data(1554),
                owner_distance_data(1),
                withdrawal_request_data(1555),
                owner_distance_data(-3),
            ]
            .pack(),
        )
        .header_dep(header1.hash())
        .header_dep(header2.hash())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, -20);
}

// This two-way batch keeps DAO index rules satisfied, so weak phase 1 owner locks are enough to rotate later claim ownership.
#[test]
fn weak_lock_valid_dao_batch_can_crosswire_claims() {
    let mut context = Context::default();
    let user1_lock = named_always_success_lock(&mut context, b"user1");
    let user2_lock = named_always_success_lock(&mut context, b"user2");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let amount1 = 1_000 * SHANNONS;
    let amount2 = 1_100 * SHANNONS;
    let total1 = deposit_capacity(&ickb_logic, &dao, 8, amount1);
    let total2 = deposit_capacity(&ickb_logic, &dao, 8, amount2);
    let header1 = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);
    let header2 = gen_header(1555, GENESIS_AR as u64, 35, 1000, 1000);
    let deposit1 = context.create_cell(
        CellOutput::new_builder()
            .capacity(total1.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    let deposit2 = context.create_cell(
        CellOutput::new_builder()
            .capacity(total2.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    link_cell_to_header(&mut context, &deposit1, &header1);
    link_cell_to_header(&mut context, &deposit2, &header2);
    let udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(user1_lock.clone())
            .type_(Some(xudt).pack())
            .build(),
        udt_data(u128::from(amount1 + amount2)),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit1).build())
        .input(CellInput::new_builder().previous_output(deposit2).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(total1.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(total2.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(user1_lock)
                .type_(Some(owned_owner.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(user2_lock)
                .type_(Some(owned_owner).pack())
                .build(),
        ])
        .outputs_data(
            vec![
                withdrawal_request_data(1554),
                withdrawal_request_data(1555),
                owner_distance_data(-1),
                owner_distance_data(-3),
            ]
            .pack(),
        )
        .header_dep(header1.hash())
        .header_dep(header2.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("under weak owner locks, owned_owner accepts crosswired owner assignments when DAO index rules are still satisfied");
}

// The later phase 2 claim follows the crosswired owner cell, not the user's intended pairing, once phase 1 accepted the weak-lock batch.
#[test]
fn weak_lock_crosswired_batch_reassigns_phase2_claims() {
    let mut context = Context::default();
    let user1_lock = named_always_success_lock(&mut context, b"user1");
    let user2_lock = named_always_success_lock(&mut context, b"user2");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let amount1 = 1_000 * SHANNONS;
    let amount2 = 1_200 * SHANNONS;
    let total1 = deposit_capacity(&ickb_logic, &dao, 8, amount1);
    let total2 = deposit_capacity(&ickb_logic, &dao, 8, amount2);
    let deposit_header1 = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);
    let deposit_header2 = gen_header(1555, GENESIS_AR as u64, 35, 1000, 1000);
    let withdraw_header1 = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    let withdraw_header2 = gen_header(2_000_621, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    let deposit1 = context.create_cell(
        CellOutput::new_builder()
            .capacity(total1.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    let deposit2 = context.create_cell(
        CellOutput::new_builder()
            .capacity(total2.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    link_cell_to_header(&mut context, &deposit1, &deposit_header1);
    link_cell_to_header(&mut context, &deposit2, &deposit_header2);
    let udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(user1_lock.clone())
            .type_(Some(xudt).pack())
            .build(),
        udt_data(u128::from(amount1 + amount2)),
    );

    let create_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit1).build())
        .input(CellInput::new_builder().previous_output(deposit2).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(total1.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(total2.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(user1_lock.clone())
                .type_(Some(owned_owner.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(user2_lock.clone())
                .type_(Some(owned_owner.clone()).pack())
                .build(),
        ])
        .outputs_data(
            vec![
                withdrawal_request_data(1554),
                withdrawal_request_data(1555),
                owner_distance_data(-1),
                owner_distance_data(-3),
            ]
            .pack(),
        )
        .header_dep(deposit_header1.hash())
        .header_dep(deposit_header2.hash())
        .build();
    let create_tx = context.complete_tx(create_tx);
    context
        .verify_tx(&create_tx, MAX_CYCLES)
        .expect("under weak owner locks, the crosswired phase1 batch should verify");

    let batch_hash = create_tx.hash();
    let owned1 = OutPoint::new(batch_hash.clone(), 0);
    let owned2 = OutPoint::new(batch_hash.clone(), 1);
    let owner1 = OutPoint::new(batch_hash.clone(), 2);
    let owner2 = OutPoint::new(batch_hash, 3);
    context.create_cell_with_out_point(owned1.clone(), create_tx.outputs().get(0).expect("owned1"), withdrawal_request_data(1554));
    context.create_cell_with_out_point(owned2.clone(), create_tx.outputs().get(1).expect("owned2"), withdrawal_request_data(1555));
    context.create_cell_with_out_point(owner1.clone(), create_tx.outputs().get(2).expect("owner1"), owner_distance_data(-1));
    context.create_cell_with_out_point(owner2.clone(), create_tx.outputs().get(3).expect("owner2"), owner_distance_data(-3));
    link_cell_to_header(&mut context, &owned1, &withdraw_header1);
    link_cell_to_header(&mut context, &owned2, &withdraw_header2);
    context.insert_header(deposit_header1.clone());
    context.insert_header(deposit_header2.clone());

    let owned2_output = create_tx.outputs().get(1).expect("owned2 output");
    let exact_crosswired_capacity = dao_maximum_withdraw_capacity(
        &owned2_output,
        withdrawal_request_data(1555).len(),
        GENESIS_AR as u64,
        SYNTHETIC_WITHDRAW_AR,
    );
    let claim_with_crosswired_pair = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(owned2.clone())
                .since(0x2003e802340002f3u64.pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(owner1.clone()).build())
        .output(
            CellOutput::new_builder()
                .capacity(exact_crosswired_capacity.pack())
                .lock(user1_lock.clone())
                .build(),
        )
        .output_data(Bytes::new().pack())
        .header_dep(withdraw_header2.hash())
        .header_dep(deposit_header2.hash())
        .witness(header_dep_index_witness(1).pack())
        .build();
    let claim_with_crosswired_pair = context.complete_tx(claim_with_crosswired_pair);
    context
        .verify_tx(&claim_with_crosswired_pair, MAX_CYCLES)
        .expect("under weak owner locks, user1's owner cell should successfully claim the crosswired second withdrawal request");

    let claim_with_intended_pair = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(owned1)
                .since(0x2003e800000002f4u64.pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(owner1).build())
        .output(
            CellOutput::new_builder()
                .capacity(dao_maximum_withdraw_capacity(
                    &create_tx.outputs().get(0).expect("owned1 output"),
                    withdrawal_request_data(1554).len(),
                    GENESIS_AR as u64,
                    SYNTHETIC_WITHDRAW_AR,
                ).pack())
                .lock(user1_lock)
                .build(),
        )
        .output_data(Bytes::new().pack())
        .header_dep(withdraw_header1.hash())
        .header_dep(deposit_header1.hash())
        .witness(header_dep_index_witness(1).pack())
        .build();
    let claim_with_intended_pair = context.complete_tx(claim_with_intended_pair);
    let err = context.verify_tx(&claim_with_intended_pair, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_OWNED_OWNER_MISMATCH);
}

// The same weak-lock claim rotation generalizes beyond two users: a three-way phase 1 crosswire can rotate later phase 2 ownership.
#[test]
fn weak_lock_three_way_crosswire_rotates_claims() {
    let mut context = Context::default();
    let user1_lock = named_always_success_lock(&mut context, b"user1");
    let user2_lock = named_always_success_lock(&mut context, b"user2");
    let user3_lock = named_always_success_lock(&mut context, b"user3");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let amount1 = 1_000 * SHANNONS;
    let amount2 = 1_100 * SHANNONS;
    let amount3 = 1_200 * SHANNONS;
    let total1 = deposit_capacity(&ickb_logic, &dao, 8, amount1);
    let total2 = deposit_capacity(&ickb_logic, &dao, 8, amount2);
    let total3 = deposit_capacity(&ickb_logic, &dao, 8, amount3);
    let deposit_header1 = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);
    let deposit_header2 = gen_header(1555, GENESIS_AR as u64, 35, 1000, 1000);
    let deposit_header3 = gen_header(1556, GENESIS_AR as u64, 35, 1000, 1000);
    let withdraw_header2 = gen_header(2_000_621, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);

    let deposit1 = context.create_cell(CellOutput::new_builder().capacity(total1.pack()).lock(ickb_logic.clone()).type_(Some(dao.clone()).pack()).build(), dao_deposit_data());
    let deposit2 = context.create_cell(CellOutput::new_builder().capacity(total2.pack()).lock(ickb_logic.clone()).type_(Some(dao.clone()).pack()).build(), dao_deposit_data());
    let deposit3 = context.create_cell(CellOutput::new_builder().capacity(total3.pack()).lock(ickb_logic.clone()).type_(Some(dao.clone()).pack()).build(), dao_deposit_data());
    link_cell_to_header(&mut context, &deposit1, &deposit_header1);
    link_cell_to_header(&mut context, &deposit2, &deposit_header2);
    link_cell_to_header(&mut context, &deposit3, &deposit_header3);
    let udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(user1_lock.clone())
            .type_(Some(xudt).pack())
            .build(),
        udt_data(u128::from(amount1 + amount2 + amount3)),
    );

    let create_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit1).build())
        .input(CellInput::new_builder().previous_output(deposit2).build())
        .input(CellInput::new_builder().previous_output(deposit3).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .outputs(vec![
            CellOutput::new_builder().capacity(total1.pack()).lock(owned_owner.clone()).type_(Some(dao.clone()).pack()).build(),
            CellOutput::new_builder().capacity(total2.pack()).lock(owned_owner.clone()).type_(Some(dao.clone()).pack()).build(),
            CellOutput::new_builder().capacity(total3.pack()).lock(owned_owner.clone()).type_(Some(dao.clone()).pack()).build(),
            CellOutput::new_builder().capacity(200u64.pack()).lock(user1_lock.clone()).type_(Some(owned_owner.clone()).pack()).build(),
            CellOutput::new_builder().capacity(200u64.pack()).lock(user2_lock.clone()).type_(Some(owned_owner.clone()).pack()).build(),
            CellOutput::new_builder().capacity(200u64.pack()).lock(user3_lock).type_(Some(owned_owner).pack()).build(),
        ])
        .outputs_data(
            vec![
                withdrawal_request_data(1554),
                withdrawal_request_data(1555),
                withdrawal_request_data(1556),
                owner_distance_data(-2),
                owner_distance_data(-2),
                owner_distance_data(-5),
            ]
            .pack(),
        )
        .header_dep(deposit_header1.hash())
        .header_dep(deposit_header2.hash())
        .header_dep(deposit_header3.hash())
        .build();

    let create_tx = context.complete_tx(create_tx);
    context
        .verify_tx(&create_tx, MAX_CYCLES)
        .expect("under weak owner locks, owned_owner accepts a three-way crosswired withdrawal batch when DAO index rules still hold");

    let batch_hash = create_tx.hash();
    let owned2 = OutPoint::new(batch_hash.clone(), 1);
    let owner1 = OutPoint::new(batch_hash.clone(), 3);
    let owner2 = OutPoint::new(batch_hash, 4);
    context.create_cell_with_out_point(owned2.clone(), create_tx.outputs().get(1).expect("owned2"), withdrawal_request_data(1555));
    context.create_cell_with_out_point(owner1.clone(), create_tx.outputs().get(3).expect("owner1"), owner_distance_data(-2));
    context.create_cell_with_out_point(owner2.clone(), create_tx.outputs().get(4).expect("owner2"), owner_distance_data(-2));
    link_cell_to_header(&mut context, &owned2, &withdraw_header2);
    context.insert_header(deposit_header2.clone());

    let owned2_output = create_tx.outputs().get(1).expect("owned2 output");
    let claim_capacity = dao_maximum_withdraw_capacity(
        &owned2_output,
        withdrawal_request_data(1555).len(),
        GENESIS_AR as u64,
        SYNTHETIC_WITHDRAW_AR,
    );
    let witness = header_dep_index_witness(1);

    let claim_with_rotated_pair = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(owned2.clone())
                .since(0x2003e802340002f3u64.pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(owner1.clone()).build())
        .output(
            CellOutput::new_builder()
                .capacity(claim_capacity.pack())
                .lock(user1_lock.clone())
                .build(),
        )
        .output_data(Bytes::new().pack())
        .header_dep(withdraw_header2.hash())
        .header_dep(deposit_header2.hash())
        .witness(witness.pack())
        .build();
    let claim_with_rotated_pair = context.complete_tx(claim_with_rotated_pair);
    context
        .verify_tx(&claim_with_rotated_pair, MAX_CYCLES)
        .expect("under weak owner locks, user1 should successfully claim user2's withdrawal request from a three-way crosswired batch");

    let claim_with_intended_pair = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(owned2)
                .since(0x2003e802340002f3u64.pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(owner2).build())
        .output(
            CellOutput::new_builder()
                .capacity(claim_capacity.pack())
                .lock(user2_lock)
                .build(),
        )
        .output_data(Bytes::new().pack())
        .header_dep(withdraw_header2.hash())
        .header_dep(deposit_header2.hash())
        .witness(witness.pack())
        .build();
    let claim_with_intended_pair = context.complete_tx(claim_with_intended_pair);
    let err = context.verify_tx(&claim_with_intended_pair, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_OWNED_OWNER_MISMATCH);
}
