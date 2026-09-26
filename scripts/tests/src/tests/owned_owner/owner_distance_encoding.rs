use super::*;

// Owner-distance encoding and melt invariants.

// Scenario: the owner distance output is truncated during phase 1 creation.
// Expectation: Owned Owner rejects the malformed distance as an encoding error.
#[test]
fn truncated_owner_distance_output_is_rejected() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * CKB;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);
    let udt_input = create_udt(&mut context, &owner_lock, &xudt, u128::from(deposit_amount));
    let deposit_input = create_deposit(&mut context, deposit_total_capacity, &ickb_logic, &dao);
    link_cell_to_header(&mut context, &deposit_input, &deposit_header);

    let owner_output = cell(occupied_capacity(&owner_lock, &owned_owner, 4), &owner_lock, Some(&owned_owner));
    let owned_output = cell(deposit_total_capacity, &owned_owner, Some(&dao));

    let tx = TransactionBuilder::default()
        .input(input(udt_input))
        .input(input(deposit_input))
        .outputs(vec![owner_output, owned_output])
        .outputs_data(vec![truncated_bytes(owner_distance_data(1), 1), withdrawal_request_data(1554)].pack())
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_ENCODING);
}

// Scenario: the owner distance output is empty instead of four bytes.
// Expectation: phase 1 rejects the empty distance as an encoding error.
#[test]
fn zero_length_owner_distance_output_is_rejected_as_encoding() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * CKB;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);
    let udt_input = create_udt(&mut context, &owner_lock, &xudt, u128::from(deposit_amount));
    let deposit_input = create_deposit(&mut context, deposit_total_capacity, &ickb_logic, &dao);
    link_cell_to_header(&mut context, &deposit_input, &deposit_header);

    let owner_output = cell(occupied_capacity(&owner_lock, &owned_owner, 4), &owner_lock, Some(&owned_owner));
    let owned_output = cell(deposit_total_capacity, &owned_owner, Some(&dao));

    let tx = TransactionBuilder::default()
        .input(input(udt_input))
        .input(input(deposit_input))
        .outputs(vec![owner_output, owned_output])
        .outputs_data(vec![Bytes::new(), withdrawal_request_data(1554)].pack())
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    fail(&context, &tx, ERROR_ENCODING);
}

// Scenario: the owner distance has valid leading bytes plus extra trailing data.
// Expectation: phase 1 ignores the trailing bytes and accepts the pair.
#[test]
fn owner_distance_trailing_bytes_are_ignored() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * CKB;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);
    let udt_input = create_udt(&mut context, &owner_lock, &xudt, u128::from(deposit_amount));
    let deposit_input = create_deposit(&mut context, deposit_total_capacity, &ickb_logic, &dao);
    link_cell_to_header(&mut context, &deposit_input, &deposit_header);

    let owner_output = cell(occupied_capacity(&owner_lock, &owned_owner, 6), &owner_lock, Some(&owned_owner));
    let owned_output = cell(deposit_total_capacity, &owned_owner, Some(&dao));

    let tx = TransactionBuilder::default()
        .input(input(udt_input))
        .input(input(deposit_input))
        .outputs(vec![owner_output, owned_output])
        .outputs_data(
            vec![
                Bytes::from([1u8, 0, 0, 0, 0xaa, 0xbb].to_vec()),
                withdrawal_request_data(1554),
            ]
            .pack(),
        )
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify(&tx, MAX_CYCLES)
        .expect("owned_owner should ignore trailing bytes in owner distance data");
}
