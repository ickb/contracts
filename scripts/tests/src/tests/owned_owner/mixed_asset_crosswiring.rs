use super::*;

// If phase 1 owner locks are weak, a mixed foreign-plus-iCKB batch can crosswire who later controls each valid DAO claim.
#[test]
fn weak_lock_mixed_foreign_and_ickb_batch_can_crosswire_claims() {
    let mut context = Context::default();
    let foreign_owner_lock = named_always_success_lock(&mut context, b"foreign-owner");
    let protocol_owner_lock = named_always_success_lock(&mut context, b"protocol-owner");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let foreign_deposit_number = 1554u64;
    let protocol_deposit_number = 1555u64;
    let foreign_deposit_header = gen_header(foreign_deposit_number, GENESIS_AR as u64, 35, 1000, 1000);
    let protocol_deposit_header = gen_header(protocol_deposit_number, GENESIS_AR as u64, 35, 1000, 1000);
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);

    let foreign_deposit_capacity = 123_456_780_000u64;
    let foreign_deposit_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(foreign_deposit_capacity.pack())
            .lock(foreign_owner_lock.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    link_cell_to_header(&mut context, &foreign_deposit_input, &foreign_deposit_header);

    let protocol_deposit_amount = 1_000 * SHANNONS;
    let protocol_deposit_capacity = deposit_capacity(&ickb_logic, &dao, 8, protocol_deposit_amount);
    let protocol_deposit_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(protocol_deposit_capacity.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    link_cell_to_header(&mut context, &protocol_deposit_input, &protocol_deposit_header);

    let udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(protocol_owner_lock.clone())
            .type_(Some(xudt).pack())
            .build(),
        udt_data(u128::from(protocol_deposit_amount)),
    );

    let create_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(foreign_deposit_input).build())
        .input(CellInput::new_builder().previous_output(protocol_deposit_input).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(foreign_deposit_capacity.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(protocol_deposit_capacity.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(foreign_owner_lock.clone())
                .type_(Some(owned_owner.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(protocol_owner_lock.clone())
                .type_(Some(owned_owner.clone()).pack())
                .build(),
        ])
        .outputs_data(
            vec![
                withdrawal_request_data(foreign_deposit_number),
                withdrawal_request_data(protocol_deposit_number),
                owner_distance_data(-1),
                owner_distance_data(-3),
            ]
            .pack(),
        )
        .header_dep(foreign_deposit_header.hash())
        .header_dep(protocol_deposit_header.hash())
        .build();

    let create_tx = context.complete_tx(create_tx);
    context
        .verify_tx(&create_tx, MAX_CYCLES)
        .expect("owned_owner should accept a weak-lock mixed foreign-plus-iCKB withdrawal batch with crosswired later claim assignments");

    let batch_hash = create_tx.hash();
    let foreign_owned = OutPoint::new(batch_hash.clone(), 0);
    let protocol_owned = OutPoint::new(batch_hash.clone(), 1);
    let foreign_owner = OutPoint::new(batch_hash.clone(), 2);
    let protocol_owner = OutPoint::new(batch_hash, 3);
    context.create_cell_with_out_point(
        foreign_owned.clone(),
        create_tx.outputs().get(0).expect("foreign owned output"),
        withdrawal_request_data(foreign_deposit_number),
    );
    context.create_cell_with_out_point(
        protocol_owned.clone(),
        create_tx.outputs().get(1).expect("protocol owned output"),
        withdrawal_request_data(protocol_deposit_number),
    );
    context.create_cell_with_out_point(
        foreign_owner.clone(),
        create_tx.outputs().get(2).expect("foreign owner output"),
        owner_distance_data(-1),
    );
    context.create_cell_with_out_point(
        protocol_owner.clone(),
        create_tx.outputs().get(3).expect("protocol owner output"),
        owner_distance_data(-3),
    );
    link_cell_to_header(&mut context, &foreign_owned, &withdraw_header);
    link_cell_to_header(&mut context, &protocol_owned, &withdraw_header);
    context.insert_header(foreign_deposit_header.clone());
    context.insert_header(protocol_deposit_header.clone());

    let claim_with_crosswired_foreign_owner = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(protocol_owned.clone())
                .since(0x2003e802340002f3u64.pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(foreign_owner.clone()).build())
        .output(
            CellOutput::new_builder()
                .capacity(
                    dao_maximum_withdraw_capacity(
                        &create_tx.outputs().get(1).expect("protocol owned output"),
                        withdrawal_request_data(protocol_deposit_number).len(),
                        GENESIS_AR as u64,
                        SYNTHETIC_WITHDRAW_AR,
                    )
                    .pack(),
                )
                .lock(foreign_owner_lock)
                .build(),
        )
        .output_data(Bytes::new().pack())
        .header_dep(withdraw_header.hash())
        .header_dep(foreign_deposit_header.hash())
        .header_dep(protocol_deposit_header.hash())
        .witness(header_dep_index_witness(2).pack())
        .build();
    let claim_with_crosswired_foreign_owner = context.complete_tx(claim_with_crosswired_foreign_owner);
    context
        .verify_tx(&claim_with_crosswired_foreign_owner, MAX_CYCLES)
        .expect("under weak owner locks, the foreign owner cell should be able to claim the real iCKB withdrawal once the mixed batch crosswires the later claim assignment");

    let claim_with_intended_protocol_owner = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(protocol_owned)
                .since(0x2003e802340002f3u64.pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(protocol_owner).build())
        .output(
            CellOutput::new_builder()
                .capacity(
                    dao_maximum_withdraw_capacity(
                        &create_tx.outputs().get(1).expect("protocol owned output"),
                        withdrawal_request_data(protocol_deposit_number).len(),
                        GENESIS_AR as u64,
                        SYNTHETIC_WITHDRAW_AR,
                    )
                    .pack(),
                )
                .lock(protocol_owner_lock)
                .build(),
        )
        .output_data(Bytes::new().pack())
        .header_dep(withdraw_header.hash())
        .header_dep(foreign_deposit_header.hash())
        .header_dep(protocol_deposit_header.hash())
        .witness(header_dep_index_witness(2).pack())
        .build();
    let claim_with_intended_protocol_owner = context.complete_tx(claim_with_intended_protocol_owner);
    let err = context
        .verify_tx(&claim_with_intended_protocol_owner, MAX_CYCLES)
        .unwrap_err();
    assert_script_error(err, ERROR_OWNED_OWNER_MISMATCH);
}
