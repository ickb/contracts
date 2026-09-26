use super::*;

// Scenario: a valid withdrawal batch also creates an unrelated non-empty-args lock output.
// Expectation: Owned Owner scans the batch, treats the non-empty args as poison, and rejects it.
#[test]
fn unrelated_non_empty_args_output_lock_poisons_withdrawal() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);
    let poisoned_lock = data1_script(&mut context, "owned_owner", Bytes::from(vec![1]));

    let deposit_amount = 1_000 * CKB;
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
        .input(input(deposit_input))
        .input(input(udt_input))
        .outputs(vec![
            cell(deposit_total_capacity, &owned_owner, Some(&dao)),
            cell(occupied_capacity(&owner_lock, &owned_owner, 4), &owner_lock, Some(&owned_owner)),
            cell(200 * CKB, &poisoned_lock, None),
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
    fail(&context, &tx, ERROR_NOT_EMPTY_ARGS);
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

    let deposit_amount = 1_000 * CKB;
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
        .input(input(deposit_input))
        .input(input(udt_input))
        .input(input(funding_cell(&mut context)))
        .outputs(vec![
            cell(deposit_total_capacity, &owned_owner, Some(&dao)),
            cell(occupied_capacity(&owner_lock, &owned_owner, 4), &owner_lock, Some(&owned_owner)),
            cell(200 * CKB, &foreign_lock, Some(&foreign_type)),
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
        .verify(&tx, MAX_CYCLES)
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

    let deposit_amount = 1_000 * CKB;
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
        .input(input(deposit_input))
        .input(input(udt_input))
        .outputs(vec![
            cell(deposit_total_capacity, &owned_owner, Some(&dao)),
            cell(occupied_capacity(&owner_lock, &owned_owner, 4), &owner_lock, Some(&owned_owner)),
            cell(deposit_total_capacity, &poisoned_lock, Some(&dao)),
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
    fail(&context, &tx, ERROR_NOT_EMPTY_ARGS);
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

    let deposit_amount = 1_000 * CKB;
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
        cell(200 * CKB, &owner_lock, Some(&poisoned_owner_type)),
        owner_distance_data(-1),
    );

    let tx = TransactionBuilder::default()
        .input(input(deposit_input))
        .input(input(udt_input))
        .input(input(poisoned_owner_input))
        .outputs(vec![
            cell(deposit_total_capacity, &owned_owner, Some(&dao)),
            cell(occupied_capacity(&owner_lock, &owned_owner, 4), &owner_lock, Some(&owned_owner)),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), owner_distance_data(-1)].pack())
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_NOT_EMPTY_ARGS);
}
