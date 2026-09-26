use super::*;

// Scenario: the owner cell is placed immediately before the owned cell and points forward with distance `1`.
// Expectation: both phase 1 creation and the later phase 2 DAO claim succeed.
#[test]
fn adjacent_positive_distance_pair_can_complete_phase2_claim() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let funding_lock = named_always_success_lock(&mut context, b"funding");
    let filler_lock = named_always_success_lock(&mut context, b"filler");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * CKB;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    let funding_input = context.create_cell(
        cell(200 * CKB, &funding_lock, None),
        Bytes::new(),
    );
    let deposit_input = create_deposit(&mut context, deposit_total_capacity, &ickb_logic, &dao);
    link_cell_to_header(&mut context, &deposit_input, &deposit_header);
    let udt_input = create_udt(&mut context, &owner_lock, &xudt, u128::from(deposit_amount));

    let create_tx = TransactionBuilder::default()
        .input(input(funding_input))
        .input(input(udt_input))
        .input(input(deposit_input))
        .outputs(vec![
            cell(100 * CKB, &filler_lock, None),
            cell(occupied_capacity(&owner_lock, &owned_owner, 4), &owner_lock, Some(&owned_owner)),
            cell(deposit_total_capacity, &owned_owner, Some(&dao)),
        ])
        .outputs_data(
            vec![
                Bytes::new(),
                owner_distance_data(1),
                withdrawal_request_data(1554),
            ]
            .pack(),
        )
        .header_dep(deposit_header.hash())
        .build();

    let create_tx = context.complete_tx(create_tx);
    context
        .verify(&create_tx, MAX_CYCLES)
        .expect("owned_owner should accept an adjacent positive-distance phase1 pair");

    let tx_hash = create_tx.hash();
    let owner_out_point = OutPoint::new(tx_hash.clone(), 1);
    let owned_out_point = OutPoint::new(tx_hash, 2);
    context.create_cell_with_out_point(
        owner_out_point.clone(),
        create_tx.outputs().get(1).expect("owner output"),
        owner_distance_data(1),
    );
    context.create_cell_with_out_point(
        owned_out_point.clone(),
        create_tx.outputs().get(2).expect("owned output"),
        withdrawal_request_data(1554),
    );
    link_cell_to_header(&mut context, &owned_out_point, &withdraw_header);
    context.insert_header(deposit_header.clone());

    let exact_capacity = dao_maximum_withdraw_capacity(
        &create_tx.outputs().get(2).expect("owned output"),
        withdrawal_request_data(1554).len(),
        GENESIS_AR as u64,
        SYNTHETIC_WITHDRAW_AR,
    );

    let witness = header_dep_index_witness(1);
    let claim_tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(owned_out_point)
                .since(0x2003e800000002f4u64.pack())
                .build(),
        )
        .input(input(owner_out_point))
        .output(cell(exact_capacity, &owner_lock, None))
        .output_data(Bytes::new().pack())
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_header.hash())
        .witness(witness.pack())
        .build();

    let claim_tx = context.complete_tx(claim_tx);
    context
        .verify(&claim_tx, MAX_CYCLES)
        .expect("an adjacent positive-distance pair should remain spendable in DAO phase2");
}

// Scenario: phase 1 creates a sparse positive-distance pair with a filler output between owner and owned cells.
// Expectation: phase 1 accepts the layout, and the exact DAO claim capacity remains spendable in phase 2.
#[test]
fn sparse_positive_distance_pair_can_complete_phase2_claim_at_exact_capacity() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let filler_lock = named_always_success_lock(&mut context, b"filler");
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * CKB;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    let funding_input = context.create_cell(
        cell(200 * CKB, &funding_lock, None),
        Bytes::new(),
    );
    let udt_input = create_udt(&mut context, &owner_lock, &xudt, u128::from(deposit_amount));
    let deposit_input = create_deposit(&mut context, deposit_total_capacity, &ickb_logic, &dao);
    link_cell_to_header(&mut context, &deposit_input, &deposit_header);

    let create_tx = TransactionBuilder::default()
        .input(input(funding_input))
        .input(input(udt_input))
        .input(input(deposit_input))
        .outputs(vec![
            cell(occupied_capacity(&owner_lock, &owned_owner, 4), &owner_lock, Some(&owned_owner)),
            cell(100 * CKB, &filler_lock, None),
            cell(deposit_total_capacity, &owned_owner, Some(&dao)),
        ])
        .outputs_data(
            vec![
                owner_distance_data(2),
                Bytes::new(),
                withdrawal_request_data(1554),
            ]
            .pack(),
        )
        .header_dep(deposit_header.hash())
        .build();

    let create_tx = context.complete_tx(create_tx);
    context
        .verify(&create_tx, MAX_CYCLES)
        .expect("owned_owner should accept a sparse positive-distance pair when DAO index rules are still satisfied");

    let tx_hash = create_tx.hash();
    let owner_out_point = OutPoint::new(tx_hash.clone(), 0);
    let owned_out_point = OutPoint::new(tx_hash, 2);
    context.create_cell_with_out_point(
        owner_out_point.clone(),
        create_tx.outputs().get(0).expect("owner output"),
        owner_distance_data(2),
    );
    context.create_cell_with_out_point(
        owned_out_point.clone(),
        create_tx.outputs().get(2).expect("owned output"),
        withdrawal_request_data(1554),
    );
    link_cell_to_header(&mut context, &owned_out_point, &withdraw_header);
    context.insert_header(deposit_header.clone());
    let witness = header_dep_index_witness(1);

    let exact_capacity = dao_maximum_withdraw_capacity(
        &create_tx.outputs().get(2).expect("owned output"),
        withdrawal_request_data(1554).len(),
        GENESIS_AR as u64,
        SYNTHETIC_WITHDRAW_AR,
    );

    let claim_tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(owned_out_point)
                .since(0x2003e800000002f4u64.pack())
                .build(),
        )
        .input(input(owner_out_point))
        .output(cell(exact_capacity, &owner_lock, None))
        .output_data(Bytes::new().pack())
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_header.hash())
        .witness(witness.pack())
        .build();
    let claim_tx = context.complete_tx(claim_tx);
    context
        .verify(&claim_tx, MAX_CYCLES)
        .expect("a sparse positive-distance pair should remain spendable in DAO phase2 at the exact claim capacity");
}

// Scenario: phase 1 consumes two noncontiguous inputs protected by the same secp lock, then phase 2 spends the signed owner cell.
// Expectation: both signed transactions verify and the sparse Owned Owner pair completes its DAO claim.
#[test]
fn secp_protected_sparse_pair_can_complete_signed_phase2_claim() {
    let mut context = Context::default();
    let (privkey, owner_lock, secp_data_dep) = secp_lock(&mut context);
    let filler_lock = named_always_success_lock(&mut context, b"filler");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * CKB;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    let funding_input = context.create_cell(
        cell(100 * CKB, &owner_lock, None),
        Bytes::new(),
    );
    let filler_input = context.create_cell(
        cell(100 * CKB, &filler_lock, None),
        Bytes::new(),
    );
    let deposit_input = create_deposit(&mut context, deposit_total_capacity, &ickb_logic, &dao);
    link_cell_to_header(&mut context, &deposit_input, &deposit_header);
    let udt_input = create_udt(&mut context, &owner_lock, &xudt, u128::from(deposit_amount));

    let create_tx = TransactionBuilder::default()
        .input(input(funding_input))
        .input(input(filler_input))
        .input(input(deposit_input))
        .input(input(udt_input))
        .outputs(vec![
            cell(occupied_capacity(&owner_lock, &owned_owner, 4), &owner_lock, Some(&owned_owner)),
            cell(100 * CKB, &filler_lock, None),
            cell(deposit_total_capacity, &owned_owner, Some(&dao)),
        ])
        .outputs_data(
            vec![
                owner_distance_data(2),
                Bytes::new(),
                withdrawal_request_data(1554),
            ]
            .pack(),
        )
        .header_dep(deposit_header.hash())
        .witness(empty_witness().pack())
        .witness(Bytes::new().pack())
        .witness(Bytes::new().pack())
        .witness(Bytes::new().pack())
        .cell_dep(secp_data_dep.clone())
        .build();
    let create_tx = sign_tx_by_input_indices(context.complete_tx(create_tx), &privkey, &[0, 3]);
    context
        .verify(&create_tx, MAX_CYCLES)
        .expect("signed phase1 creation with a noncontiguous secp group should verify");

    let owner_out_point = seed_verified_output(&mut context, &create_tx, 0, owner_distance_data(2));
    let owned_out_point = seed_verified_output(&mut context, &create_tx, 2, withdrawal_request_data(1554));
    link_cell_to_header(&mut context, &owned_out_point, &withdraw_header);
    context.insert_header(deposit_header.clone());

    let exact_capacity = dao_maximum_withdraw_capacity(
        &create_tx.outputs().get(2).expect("owned output"),
        withdrawal_request_data(1554).len(),
        GENESIS_AR as u64,
        SYNTHETIC_WITHDRAW_AR,
    );
    let claim_tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(owned_out_point)
                .since(0x2003e800000002f4u64.pack())
                .build(),
        )
        .input(input(owner_out_point))
        .output(cell(exact_capacity, &owner_lock, None))
        .output_data(Bytes::new().pack())
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_header.hash())
        .witness(header_dep_index_witness(1).pack())
        .witness(empty_witness().pack())
        .cell_dep(secp_data_dep)
        .build();
    let claim_tx = sign_tx_by_input_group(context.complete_tx(claim_tx), &privkey, 1, 1);
    context
        .verify(&claim_tx, MAX_CYCLES)
        .expect("signed secp-protected owner should complete the phase2 DAO claim");
}
