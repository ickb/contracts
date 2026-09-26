use super::*;

// Scenario: a canonical phase 1 withdrawal creates one owned cell and one matching owner cell.
// Expectation: the matched pair verifies successfully.
#[test]
fn valid_output_pair_passes() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * CKB;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);
    let deposit_input = create_deposit(&mut context, deposit_total_capacity, &ickb_logic, &dao);
    link_cell_to_header(&mut context, &deposit_input, &deposit_header);
    let udt_input = create_udt(&mut context, &owner_lock, &xudt, u128::from(deposit_amount));
    let owned_output = cell(deposit_total_capacity, &owned_owner, Some(&dao));
    let owner_output = cell(occupied_capacity(&owner_lock, &owned_owner, 4), &owner_lock, Some(&owned_owner));

    let tx = TransactionBuilder::default()
        .input(input(deposit_input))
        .input(input(udt_input))
        .outputs(vec![owned_output, owner_output])
        .outputs_data(vec![withdrawal_request_data(1554), owner_distance_data(-1)].pack())
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
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
        cell(withdrawal_capacity + 200u64, &funding_lock, None),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(input(funding_input))
        .outputs(vec![
            cell(withdrawal_capacity, &owned_owner, Some(&dao)),
            cell(occupied_capacity(&user_lock, &owned_owner, 4), &user_lock, Some(&owned_owner)),
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
    fail(&context, &tx, ERROR_DAO_NEWLY_CREATED_CELL);
}

// Scenario: one withdrawal output is paired with two owner outputs in the same batch.
// Expectation: Owned Owner rejects the ambiguous pairing.
#[test]
fn two_owner_cells_for_one_owned_output_are_rejected() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let funding_lock = named_always_success_lock(&mut context, b"funding");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * CKB;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);
    let deposit_input = create_deposit(&mut context, deposit_total_capacity, &ickb_logic, &dao);
    link_cell_to_header(&mut context, &deposit_input, &deposit_header);
    let udt_input = create_udt(&mut context, &owner_lock, &xudt, u128::from(deposit_amount));
    let funding_input = context.create_cell(
        cell(200 * CKB, &funding_lock, None),
        Bytes::new(),
    );

    let tx = TransactionBuilder::default()
        .input(input(deposit_input))
        .input(input(udt_input))
        .input(input(funding_input))
        .outputs(vec![
            cell(deposit_total_capacity, &owned_owner, Some(&dao)),
            cell(occupied_capacity(&owner_lock, &owned_owner, 4), &owner_lock, Some(&owned_owner)),
            cell(occupied_capacity(&owner_lock, &owned_owner, 4), &owner_lock, Some(&owned_owner)),
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
    fail(&context, &tx, ERROR_OWNED_OWNER_MISMATCH);
}

// The reverse of the case above: two owned withdrawal requests and a single owner cell.
// Expectation: Owned Owner rejects the request left without an owner.
#[test]
fn two_owned_outputs_for_one_owner_cell_are_rejected() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let funding_lock = named_always_success_lock(&mut context, b"funding");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * CKB;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);
    let mut deposit = || {
        let out_point = create_deposit(&mut context, deposit_total_capacity, &ickb_logic, &dao);
        link_cell_to_header(&mut context, &out_point, &deposit_header);
        out_point
    };
    let (first_deposit, second_deposit) = (deposit(), deposit());
    let udt_input = create_udt(&mut context, &owner_lock, &xudt, 2 * u128::from(deposit_amount));
    let funding_input = context.create_cell(
        cell(200 * CKB, &funding_lock, None),
        Bytes::new(),
    );

    let owned_request = || {
        cell(deposit_total_capacity, &owned_owner, Some(&dao))
    };
    let tx = TransactionBuilder::default()
        .input(input(first_deposit))
        .input(input(second_deposit))
        .input(input(udt_input))
        .input(input(funding_input))
        .outputs(vec![
            owned_request(),
            owned_request(),
            cell(occupied_capacity(&owner_lock, &owned_owner, 4), &owner_lock, Some(&owned_owner)),
        ])
        .outputs_data(
            vec![
                withdrawal_request_data(1554),
                withdrawal_request_data(1554),
                owner_distance_data(-2),
            ]
            .pack(),
        )
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_OWNED_OWNER_MISMATCH);
}

// Output-lock creation can admit an orphan withdrawal request, but the later claim still fails once Owned Owner executes.
#[test]
fn orphan_withdrawal_request_can_be_created_but_not_claimed() {
    let mut context = Context::default();
    let user_lock = named_always_success_lock(&mut context, b"user");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * CKB;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    let deposit_input = create_deposit(&mut context, deposit_total_capacity, &ickb_logic, &dao);
    link_cell_to_header(&mut context, &deposit_input, &deposit_header);
    let udt_input = create_udt(&mut context, &user_lock, &xudt, u128::from(deposit_amount));

    let create_tx = TransactionBuilder::default()
        .input(input(deposit_input))
        .input(input(udt_input))
        .output(cell(deposit_total_capacity, &owned_owner, Some(&dao)))
        .output_data(withdrawal_request_data(1554).pack())
        .header_dep(deposit_header.hash())
        .build();

    let create_tx = context.complete_tx(create_tx);
    context
        .verify(&create_tx, MAX_CYCLES)
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
        .output(cell(123_468_106_670u64, &user_lock, None))
        .output_data(Bytes::new().pack())
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_header.hash())
        .witness(witness.pack())
        .build();

    let claim_tx = context.complete_tx(claim_tx);
    fail(&context, &claim_tx, ERROR_OWNED_OWNER_MISMATCH);
}

// A type-script orphan is rejected immediately because Owned Owner sees the full output pairing and finds no matching owned cell.
#[test]
fn orphan_owner_output_is_rejected() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let owned_owner = owned_owner_script(&mut context);

    let funding_input = context.create_cell(
        cell(500 * CKB, &funding_lock, None),
        Bytes::new(),
    );
    let tx = TransactionBuilder::default()
        .input(input(funding_input))
        .output(
            cell(occupied_capacity(&owner_lock, &owned_owner, 4), &owner_lock, Some(&owned_owner)),
        )
        .output_data(owner_distance_data(1).pack())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_OWNED_OWNER_MISMATCH);
}
