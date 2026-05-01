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

    let deposit_amount = 1_000 * SHANNONS;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);
    let withdraw_header = gen_header(2_000_610, 10_001_000, 575, 2_000_000, 1100);
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(100u64.pack())
            .lock(funding_lock)
            .build(),
        Bytes::new(),
    );
    let deposit_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_total_capacity.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    link_cell_to_header(&mut context, &deposit_input, &deposit_header);
    let udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(owner_lock.clone())
            .type_(Some(xudt).pack())
            .build(),
        udt_data(u128::from(deposit_amount)),
    );

    let create_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .input(CellInput::new_builder().previous_output(deposit_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(100u64.pack())
                .lock(filler_lock)
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(owner_lock.clone())
                .type_(Some(owned_owner.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(deposit_total_capacity.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
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
        .verify_tx(&create_tx, MAX_CYCLES)
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
        10_001_000,
    );

    let witness = header_dep_index_witness(1);
    let claim_tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(owned_out_point)
                .since(0x2003e800000002f4u64.pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(owner_out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity(exact_capacity.pack())
                .lock(owner_lock)
                .build(),
        )
        .output_data(Bytes::new().pack())
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_header.hash())
        .witness(witness.pack())
        .build();

    let claim_tx = context.complete_tx(claim_tx);
    context
        .verify_tx(&claim_tx, MAX_CYCLES)
        .expect("an adjacent positive-distance pair should remain spendable in DAO phase2");
}

// Scenario: phase 1 creates a sparse positive-distance pair with a filler output between owner and owned cells.
// Expectation: phase 1 accepts the layout, but no valid phase 2 claim capacity exists for it.
#[test]
fn sparse_positive_distance_pair_has_no_valid_phase2_claim_path() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let filler_lock = named_always_success_lock(&mut context, b"filler");
    let funding_lock = always_success_lock(&mut context);
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * SHANNONS;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);
    let withdraw_header = gen_header(2_000_610, 10_001_000, 575, 2_000_000, 1100);
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(100u64.pack())
            .lock(funding_lock)
            .build(),
        Bytes::new(),
    );
    let udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(owner_lock.clone())
            .type_(Some(xudt).pack())
            .build(),
        udt_data(u128::from(deposit_amount)),
    );
    let deposit_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_total_capacity.pack())
            .lock(ickb_logic)
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    link_cell_to_header(&mut context, &deposit_input, &deposit_header);

    let create_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .input(CellInput::new_builder().previous_output(deposit_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(owner_lock.clone())
                .type_(Some(owned_owner.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(100u64.pack())
                .lock(filler_lock)
                .build(),
            CellOutput::new_builder()
                .capacity(deposit_total_capacity.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
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
        .verify_tx(&create_tx, MAX_CYCLES)
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

    let mut discovered_capacity = None;
    for capacity in [123_468_105_686u64, 123_468_105_886u64, 123_468_106_670u64, 123_468_106_870u64]
        .into_iter()
        .chain(123_468_105_200u64..=123_468_107_200u64)
    {
        let claim_tx = TransactionBuilder::default()
            .input(
                CellInput::new_builder()
                    .previous_output(owned_out_point.clone())
                    .since(0x2003e800000002f4u64.pack())
                    .build(),
            )
            .input(CellInput::new_builder().previous_output(owner_out_point.clone()).build())
            .output(
                CellOutput::new_builder()
                    .capacity(capacity.pack())
                    .lock(owner_lock.clone())
                    .build(),
            )
            .output_data(Bytes::new().pack())
            .header_dep(withdraw_header.hash())
            .header_dep(deposit_header.hash())
            .witness(witness.pack())
            .build();
        let claim_tx = context.complete_tx(claim_tx);
        if context.verify_tx(&claim_tx, MAX_CYCLES).is_ok() {
            discovered_capacity = Some(capacity);
            break;
        }
    }

    assert!(
        discovered_capacity.is_none(),
        "sparse positive-distance phase1 pairs should not become spendable in DAO phase2, found capacity {:?}",
        discovered_capacity
    );
}
