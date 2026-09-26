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
    // For deposits since block 10,000,000 the node keeps a withdrawal request's lock the same size as
    // its deposit's, so only a foreign deposit whose lock has Owned Owner's size (empty args) can be wrapped.
    let wrappable_lock = always_success_lock(&mut context);
    let foreign_deposit_input = create_deposit(&mut context, foreign_deposit_capacity, &wrappable_lock, &dao);
    link_cell_to_header(&mut context, &foreign_deposit_input, &foreign_deposit_header);

    let protocol_deposit_amount = 1_000 * CKB;
    let protocol_deposit_capacity = deposit_capacity(&ickb_logic, &dao, 8, protocol_deposit_amount);
    let protocol_deposit_input = create_deposit(&mut context, protocol_deposit_capacity, &ickb_logic, &dao);
    link_cell_to_header(&mut context, &protocol_deposit_input, &protocol_deposit_header);

    let udt_input = create_udt(&mut context, &protocol_owner_lock, &xudt, u128::from(protocol_deposit_amount));

    let create_tx = TransactionBuilder::default()
        .input(input(foreign_deposit_input))
        .input(input(protocol_deposit_input))
        .input(input(udt_input))
        .input(input(funding_cell(&mut context)))
        .outputs(vec![
            cell(foreign_deposit_capacity, &owned_owner, Some(&dao)),
            cell(protocol_deposit_capacity, &owned_owner, Some(&dao)),
            cell(occupied_capacity(&foreign_owner_lock, &owned_owner, 4), &foreign_owner_lock, Some(&owned_owner)),
            cell(occupied_capacity(&protocol_owner_lock, &owned_owner, 4), &protocol_owner_lock, Some(&owned_owner)),
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
        .verify(&create_tx, MAX_CYCLES)
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
        .input(input(foreign_owner.clone()))
        .output(
            cell(
                dao_maximum_withdraw_capacity(
                    &create_tx.outputs().get(1).expect("protocol owned output"),
                    withdrawal_request_data(protocol_deposit_number).len(),
                    GENESIS_AR as u64,
                    SYNTHETIC_WITHDRAW_AR,
                ),
                &foreign_owner_lock,
                None,
            ),
        )
        .output_data(Bytes::new().pack())
        .header_dep(withdraw_header.hash())
        .header_dep(foreign_deposit_header.hash())
        .header_dep(protocol_deposit_header.hash())
        .witness(header_dep_index_witness(2).pack())
        .build();
    let claim_with_crosswired_foreign_owner = context.complete_tx(claim_with_crosswired_foreign_owner);
    context
        .verify(&claim_with_crosswired_foreign_owner, MAX_CYCLES)
        .expect("under weak owner locks, the foreign owner cell should be able to claim the real iCKB withdrawal once the mixed batch crosswires the later claim assignment");

    let claim_with_intended_protocol_owner = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(protocol_owned)
                .since(0x2003e802340002f3u64.pack())
                .build(),
        )
        .input(input(protocol_owner))
        .output(
            cell(
                dao_maximum_withdraw_capacity(
                    &create_tx.outputs().get(1).expect("protocol owned output"),
                    withdrawal_request_data(protocol_deposit_number).len(),
                    GENESIS_AR as u64,
                    SYNTHETIC_WITHDRAW_AR,
                ),
                &protocol_owner_lock,
                None,
            ),
        )
        .output_data(Bytes::new().pack())
        .header_dep(withdraw_header.hash())
        .header_dep(foreign_deposit_header.hash())
        .header_dep(protocol_deposit_header.hash())
        .witness(header_dep_index_witness(2).pack())
        .build();
    let claim_with_intended_protocol_owner = context.complete_tx(claim_with_intended_protocol_owner);
    fail(&context, &claim_with_intended_protocol_owner, ERROR_OWNED_OWNER_MISMATCH);
}
