use super::*;

// Creation-time output checks do not block outputs that later become unspendable.
// Phase 1 accepts an owner cell whose lock never executed on creation, but the later claim fails once that foreign lock has to run.
#[test]
fn phase1_accepts_unspendable_foreign_owner_lock() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);
    let poisoned_lock = data1_script(&mut context, "limit_order", Bytes::from(vec![1]));

    let deposit_amount = 1_000 * SHANNONS;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);
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
        .input(CellInput::new_builder().previous_output(deposit_input).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(deposit_total_capacity.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(poisoned_lock)
                .type_(Some(owned_owner.clone()).pack())
                .build(),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), owner_distance_data(-1)].pack())
        .header_dep(deposit_header.hash())
        .build();

    let create_tx = context.complete_tx(create_tx);
    context
        .verify_tx(&create_tx, MAX_CYCLES)
        .expect("owned_owner creation accepts an owner cell whose lock never executed on creation");

    let tx_hash = create_tx.hash();
    let owned_out_point = OutPoint::new(tx_hash.clone(), 0);
    let owner_out_point = OutPoint::new(tx_hash, 1);
    context.create_cell_with_out_point(
        owned_out_point.clone(),
        create_tx.outputs().get(0).expect("owned output"),
        withdrawal_request_data(1554),
    );
    context.create_cell_with_out_point(
        owner_out_point.clone(),
        create_tx.outputs().get(1).expect("owner output"),
        owner_distance_data(-1),
    );
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    link_cell_to_header(&mut context, &owned_out_point, &withdraw_header);
    context.insert_header(deposit_header.clone());
    let witness = header_dep_index_witness(1);

    let melt_tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(owned_out_point)
                .since(0x2003e800000002f4u64.pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(owner_out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity(123_468_106_670u64.pack())
                .lock(owner_lock)
                .build(),
        )
        .output_data(Bytes::new().pack())
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_header.hash())
        .witness(witness.pack())
        .build();

    let melt_tx = context.complete_tx(melt_tx);
    let err = context.verify_tx(&melt_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_NOT_EMPTY_ARGS);
}

// Even an empty-args foreign lock can strand the pair later if the eventual owner lock semantics do not match the claim flow.
#[test]
fn phase1_accepts_limit_order_owner_lock_but_claim_strands() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);
    let benign_foreign_lock = limit_order_script(&mut context);

    let deposit_amount = 1_000 * SHANNONS;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);
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
        .input(CellInput::new_builder().previous_output(deposit_input).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(deposit_total_capacity.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(benign_foreign_lock)
                .type_(Some(owned_owner.clone()).pack())
                .build(),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), owner_distance_data(-1)].pack())
        .header_dep(deposit_header.hash())
        .build();

    let create_tx = context.complete_tx(create_tx);
    context
        .verify_tx(&create_tx, MAX_CYCLES)
        .expect("owned_owner creation accepts an empty-args limit_order owner lock at creation");

    let tx_hash = create_tx.hash();
    let owned_out_point = OutPoint::new(tx_hash.clone(), 0);
    let owner_out_point = OutPoint::new(tx_hash, 1);
    context.create_cell_with_out_point(
        owned_out_point.clone(),
        create_tx.outputs().get(0).expect("owned output"),
        withdrawal_request_data(1554),
    );
    context.create_cell_with_out_point(
        owner_out_point.clone(),
        create_tx.outputs().get(1).expect("owner output"),
        owner_distance_data(-1),
    );
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    link_cell_to_header(&mut context, &owned_out_point, &withdraw_header);
    context.insert_header(deposit_header.clone());
    let witness = header_dep_index_witness(1);

    let melt_tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(owned_out_point)
                .since(0x2003e800000002f4u64.pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(owner_out_point).build())
        .output(
            CellOutput::new_builder()
                .capacity(123_468_106_670u64.pack())
                .lock(owner_lock)
                .build(),
        )
        .output_data(Bytes::new().pack())
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_header.hash())
        .witness(witness.pack())
        .build();

    let melt_tx = context.complete_tx(melt_tx);
    let err = context.verify_tx(&melt_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_ENCODING);
}

// Scenario: phase 1 tries to emit an owner cell whose lock and type are both limit-order driven.
// Expectation: Owned Owner blocks this stronger form immediately instead of letting it reach phase 2.
#[test]
fn limit_order_backed_owner_is_blocked_in_phase1() {
    let mut context = Context::default();
    let (_attacker_privkey, attacker_lock, _secp_data_dep) = secp_lock(&mut context);
    let ickb_logic = ickb_logic_script(&mut context);
    let owned_owner = owned_owner_script(&mut context);
    let limit_order = limit_order_script(&mut context);
    let dao = dao_script(&mut context);
    let xudt = xudt_script(&mut context, &ickb_logic);
    let burn_lock = named_always_success_lock(&mut context, b"owner");

    let deposit_amount = 1_000 * SHANNONS;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);
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
            .lock(burn_lock)
            .type_(Some(xudt).pack())
            .build(),
        udt_data(u128::from(deposit_amount)),
    );

    let master_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(attacker_lock.clone())
            .type_(Some(limit_order.clone()).pack())
            .build(),
        Bytes::new(),
    );

    let owner_order_data = order_data_match(u128::from(u32::MAX), &master_out_point, (1, 1));
    let create_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit_input).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(deposit_total_capacity.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(deposit_capacity(&limit_order, &owned_owner, owner_order_data.len(), 1_500 * SHANNONS).pack())
                .lock(limit_order.clone())
                .type_(Some(owned_owner.clone()).pack())
                .build(),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), owner_order_data.clone()].pack())
        .header_dep(deposit_header.hash())
        .build();

    let create_tx = context.complete_tx(create_tx);
    let err = context.verify_tx(&create_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_DAO_INCORRECT_CAPACITY);
}
