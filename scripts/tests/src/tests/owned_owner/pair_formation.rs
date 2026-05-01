use super::*;

// Scenario: a canonical phase 1 withdrawal creates one owned cell and one matching owner cell.
// Expectation: the matched pair verifies successfully.
#[test]
fn valid_output_pair_passes() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

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
    let owned_output = CellOutput::new_builder()
        .capacity(deposit_total_capacity.pack())
        .lock(owned_owner.clone())
        .type_(Some(dao).pack())
        .build();
    let owner_output = CellOutput::new_builder()
        .capacity(200u64.pack())
        .lock(owner_lock)
        .type_(Some(owned_owner).pack())
        .build();

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit_input).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .outputs(vec![owned_output, owner_output])
        .outputs_data(vec![withdrawal_request_data(1554), owner_distance_data(-1)].pack())
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("owned_owner should accept a matched output pair");
}

// Pair formation and orphaned-output validation.

// Scenario: a transaction forges an Owned Owner withdrawal-looking pair without consuming any DAO deposit.
// Expectation: the batch is rejected because there is no real withdrawal to pair against.
#[test]
fn withdrawal_shape_cannot_be_created_without_any_dao_input() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"no-dao-user");
    let funding_lock = named_always_success_lock(&mut context, b"no-dao-funding");
    let owned_owner = owned_owner_script(&mut context);
    let dao = dao_script(&mut context);

    let withdrawal_capacity = 123_456_780_000u64;
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((withdrawal_capacity + 200u64).pack())
            .lock(funding_lock)
            .build(),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(withdrawal_capacity.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(user_lock)
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
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, -19);
}

// Scenario: one withdrawal output is paired with two owner outputs in the same batch.
// Expectation: Owned Owner rejects the ambiguous pairing.
#[test]
fn two_owner_cells_for_one_owned_output_are_rejected() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let funding_lock = named_always_success_lock(&mut context, b"funding");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

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
    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(funding_lock)
            .build(),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit_input).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(deposit_total_capacity.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(owner_lock.clone())
                .type_(Some(owned_owner.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(owner_lock)
                .type_(Some(owned_owner).pack())
                .build(),
        ])
        .outputs_data(
            vec![
                withdrawal_request_data(1554),
                owner_distance_data(-1),
                owner_distance_data(-2),
            ]
            .pack(),
        )
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_OWNED_OWNER_MISMATCH);
}

// Output-lock creation can admit an orphan withdrawal request, but the later claim still fails once Owned Owner executes.
#[test]
fn orphan_withdrawal_request_can_be_created_but_not_claimed() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"user");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * SHANNONS;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
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
            .lock(user_lock.clone())
            .type_(Some(xudt).pack())
            .build(),
        udt_data(u128::from(deposit_amount)),
    );

    let create_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit_input).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .output(
            CellOutput::new_builder()
                .capacity(deposit_total_capacity.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
        )
        .output_data(withdrawal_request_data(1554).pack())
        .header_dep(deposit_header.hash())
        .build();

    let create_tx = context.complete_tx(create_tx);
    context
        .verify_tx(&create_tx, MAX_CYCLES)
        .expect("phase1 accepts an orphan withdrawal request because the owned_owner output lock never executes");

    let orphan_out_point = OutPoint::new(create_tx.hash(), 0);
    context.create_cell_with_out_point(
        orphan_out_point.clone(),
        create_tx.outputs().get(0).expect("orphan owned output"),
        withdrawal_request_data(1554),
    );
    link_cell_to_header(&mut context, &orphan_out_point, &withdraw_header);
    context.insert_header(deposit_header.clone());
    let witness = header_dep_index_witness(1);

    let claim_tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(orphan_out_point)
                .since(0x2003e800000002f4u64.pack())
                .build(),
        )
        .output(
            CellOutput::new_builder()
                .capacity(123_468_106_670u64.pack())
                .lock(user_lock)
                .build(),
        )
        .output_data(Bytes::new().pack())
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_header.hash())
        .witness(witness.pack())
        .build();

    let claim_tx = context.complete_tx(claim_tx);
    let err = context.verify_tx(&claim_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_OWNED_OWNER_MISMATCH);
}

// A type-script orphan is rejected immediately because Owned Owner sees the full output pairing and finds no matching owned cell.
#[test]
fn orphan_owner_output_is_rejected() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let owned_owner = owned_owner_script(&mut context);

    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(500u64.pack())
            .lock(funding_lock)
            .build(),
        Bytes::new(),
    );
    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .output(
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(owner_lock)
                .type_(Some(owned_owner).pack())
                .build(),
        )
        .output_data(owner_distance_data(1).pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_OWNED_OWNER_MISMATCH);
}
