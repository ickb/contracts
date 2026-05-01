use super::*;

// Live claim rotation and pair-swapping limits.
// A live claim cannot be re-emitted as a fresh Owned Owner pair during phase 2: DAO withdrawal semantics block the rotation before a new live pair exists.
#[test]
fn live_claim_cannot_roll_into_fresh_pair() {
    let mut context = Context::default();
    let owned_owner = owned_owner_script(&mut context);
    let dao = dao_script(&mut context);
    let old_owner_lock = named_always_success_lock(&mut context, b"old-owner");
    let new_owner_lock = named_always_success_lock(&mut context, b"new-owner");
    let deposit_header = gen_header(1554, SYNTHETIC_DEPOSIT_AR, 35, 1000, 1000);
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    let shared_tx_hash = Byte32::from_slice(&[9u8; 32]).expect("shared tx hash");
    let owned_input = OutPoint::new(shared_tx_hash.clone(), 0);
    let owner_input = OutPoint::new(shared_tx_hash, 1);

    context.create_cell_with_out_point(
        owned_input.clone(),
        CellOutput::new_builder()
            .capacity(123_456_780_000u64.pack())
            .lock(owned_owner.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        withdrawal_request_data(1554),
    );
    context.create_cell_with_out_point(
        owner_input.clone(),
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(old_owner_lock)
            .type_(Some(owned_owner.clone()).pack())
            .build(),
        owner_distance_data(-1),
    );
    link_cell_to_header(&mut context, &owned_input, &withdraw_header);
    context.insert_header(deposit_header.clone());
    let witness = header_dep_index_witness(1);

    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(owned_input)
                .since(0x2003e800000002f4u64.pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(owner_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(123_456_780_000u64.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(new_owner_lock)
                .type_(Some(owned_owner).pack())
                .build(),
        ])
        .outputs_data(
            vec![
                withdrawal_request_data(1554),
                owner_distance_data(-1),
            ]
            .pack(),
        )
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_header.hash())
        .witness(witness.pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, -19);
}

// Consuming multiple live pairs still cannot rotate them into a new set of live pairs once the full DAO claim rules are enforced.
#[test]
fn live_claims_cannot_rotate_into_new_pairs() {
    let mut context = Context::default();
    let user1_lock = named_always_success_lock(&mut context, b"user1");
    let user2_lock = named_always_success_lock(&mut context, b"user2");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let amount1 = 1_000 * SHANNONS;
    let amount2 = 1_100 * SHANNONS;
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
                owner_distance_data(-2),
                owner_distance_data(-2),
            ]
            .pack(),
        )
        .header_dep(deposit_header1.hash())
        .header_dep(deposit_header2.hash())
        .build();
    let create_tx = context.complete_tx(create_tx);
    context
        .verify_tx(&create_tx, MAX_CYCLES)
        .expect("initial live owned_owner pairs should verify");

    let batch_hash = create_tx.hash();
    let owned1 = OutPoint::new(batch_hash.clone(), 0);
    let owned2 = OutPoint::new(batch_hash.clone(), 1);
    let owner1 = OutPoint::new(batch_hash.clone(), 2);
    let owner2 = OutPoint::new(batch_hash, 3);
    context.create_cell_with_out_point(
        owned1.clone(),
        create_tx.outputs().get(0).expect("owned1 output"),
        withdrawal_request_data(1554),
    );
    context.create_cell_with_out_point(
        owned2.clone(),
        create_tx.outputs().get(1).expect("owned2 output"),
        withdrawal_request_data(1555),
    );
    context.create_cell_with_out_point(
        owner1.clone(),
        create_tx.outputs().get(2).expect("owner1 output"),
        owner_distance_data(-2),
    );
    context.create_cell_with_out_point(
        owner2.clone(),
        create_tx.outputs().get(3).expect("owner2 output"),
        owner_distance_data(-2),
    );
    link_cell_to_header(&mut context, &owned1, &withdraw_header1);
    link_cell_to_header(&mut context, &owned2, &withdraw_header2);
    context.insert_header(deposit_header1.clone());
    context.insert_header(deposit_header2.clone());

    let witness1 = header_dep_index_witness(2);
    let witness2 = header_dep_index_witness(3);

    let rotate_tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(owned1)
                .since(0x2003e800000002f4u64.pack())
                .build(),
        )
        .input(
            CellInput::new_builder()
                .previous_output(owned2)
                .since(0x2003e802340002f3u64.pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(owner1).build())
        .input(CellInput::new_builder().previous_output(owner2).build())
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
        .header_dep(withdraw_header1.hash())
        .header_dep(withdraw_header2.hash())
        .header_dep(deposit_header1.hash())
        .header_dep(deposit_header2.hash())
        .witness(witness1.pack())
        .witness(witness2.pack())
        .build();

    let rotate_tx = context.complete_tx(rotate_tx);
    let err = context.verify_tx(&rotate_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, -19);
}
