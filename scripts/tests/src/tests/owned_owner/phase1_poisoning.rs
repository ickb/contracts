use super::*;

// Scenario: a valid withdrawal batch also creates an unrelated non-empty-args lock output.
// Expectation: Owned Owner scans the batch, treats the non-empty args as poison, and rejects it.
#[test]
fn unrelated_non_empty_args_output_lock_poisons_withdrawal() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);
    let poisoned_lock = data1_script(&mut context, "owned_owner", Bytes::from(vec![1]));

    let deposit_amount = 1_000 * SHANNONS;
    let (deposit_total_capacity, deposit_header, deposit_input, udt_input) = create_withdrawal_inputs(
        &mut context,
        &ickb_logic,
        &dao,
        &xudt,
        owner_lock.clone(),
        deposit_amount,
        1554,
    );

    let tx = TransactionBuilder::default()
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
                .lock(owner_lock)
                .type_(Some(owned_owner).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(poisoned_lock)
                .build(),
        ])
        .outputs_data(
            vec![
                withdrawal_request_data(1554),
                owner_distance_data(-1),
                Bytes::new(),
            ]
            .pack(),
        )
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_NOT_EMPTY_ARGS);
}

// Scenario: the batch includes an unrelated typed output that does not use Owned Owner.
// Expectation: the foreign typed output is ignored and the valid withdrawal pair still verifies.
#[test]
fn foreign_typed_output_is_ignored() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);
    let foreign_lock = named_always_success_lock(&mut context, b"foreign-lock");
    let foreign_type = named_always_success_lock(&mut context, b"foreign-type");

    let deposit_amount = 1_000 * SHANNONS;
    let (deposit_total_capacity, deposit_header, deposit_input, udt_input) = create_withdrawal_inputs(
        &mut context,
        &ickb_logic,
        &dao,
        &xudt,
        owner_lock.clone(),
        deposit_amount,
        1554,
    );

    let tx = TransactionBuilder::default()
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
                .lock(owner_lock)
                .type_(Some(owned_owner).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(foreign_lock)
                .type_(Some(foreign_type).pack())
                .build(),
        ])
        .outputs_data(
            vec![
                withdrawal_request_data(1554),
                owner_distance_data(-1),
                Bytes::new(),
            ]
            .pack(),
        )
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("owned_owner should ignore unrelated foreign typed outputs during a valid withdrawal batch");
}

// Scenario: the batch includes a DAO-shaped sibling output that looks like Owned Owner but has non-empty args.
// Expectation: the plausible shape is still poison because the args are non-empty.
#[test]
fn owned_shaped_non_empty_args_output_poisons_withdrawal() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let ickb_logic = ickb_logic_script(&mut context);
    let owned_owner = owned_owner_script(&mut context);
    let poisoned_lock = data1_script(&mut context, "owned_owner", Bytes::from(vec![1]));
    let dao = dao_script(&mut context);
    let xudt = xudt_script(&mut context, &ickb_logic);

    let deposit_amount = 1_000 * SHANNONS;
    let (deposit_total_capacity, deposit_header, deposit_input, udt_input) = create_withdrawal_inputs(
        &mut context,
        &ickb_logic,
        &dao,
        &xudt,
        owner_lock.clone(),
        deposit_amount,
        1554,
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit_input).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(deposit_total_capacity.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(owner_lock)
                .type_(Some(owned_owner.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(deposit_total_capacity.pack())
                .lock(poisoned_lock)
                .type_(Some(dao).pack())
                .build(),
        ])
        .outputs_data(
            vec![
                withdrawal_request_data(1554),
                owner_distance_data(-1),
                withdrawal_request_data(1554),
            ]
            .pack(),
        )
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_NOT_EMPTY_ARGS);
}

// Scenario: a valid withdrawal consumes an owner-side sibling input that uses non-empty args.
// Expectation: Owned Owner rejects the whole batch because poisoned siblings are checked on inputs too.
#[test]
fn non_empty_args_owner_sibling_poisons_withdrawal() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let ickb_logic = ickb_logic_script(&mut context);
    let owned_owner = owned_owner_script(&mut context);
    let poisoned_owner_type = data1_script(&mut context, "owned_owner", Bytes::from(vec![1]));
    let dao = dao_script(&mut context);
    let xudt = xudt_script(&mut context, &ickb_logic);

    let deposit_amount = 1_000 * SHANNONS;
    let (deposit_total_capacity, deposit_header, deposit_input, udt_input) = create_withdrawal_inputs(
        &mut context,
        &ickb_logic,
        &dao,
        &xudt,
        owner_lock.clone(),
        deposit_amount,
        1554,
    );
    let poisoned_owner_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(owner_lock.clone())
            .type_(Some(poisoned_owner_type).pack())
            .build(),
        owner_distance_data(-1),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit_input).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .input(CellInput::new_builder().previous_output(poisoned_owner_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(deposit_total_capacity.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(owner_lock)
                .type_(Some(owned_owner).pack())
                .build(),
        ])
        .outputs_data(
            vec![withdrawal_request_data(1554), owner_distance_data(-1)].pack(),
        )
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_NOT_EMPTY_ARGS);
}
