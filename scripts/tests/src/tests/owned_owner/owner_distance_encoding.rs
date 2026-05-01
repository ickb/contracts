use super::*;

// Owner-distance encoding and melt invariants.

// Scenario: the owner distance output is truncated during phase 1 creation.
// Expectation: Owned Owner rejects the malformed distance as an encoding error.
#[test]
fn truncated_owner_distance_output_is_rejected() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * SHANNONS;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);
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

    let owner_output = CellOutput::new_builder()
        .capacity(200u64.pack())
        .lock(owner_lock.clone())
        .type_(Some(owned_owner.clone()).pack())
        .build();
    let owned_output = CellOutput::new_builder()
        .capacity(deposit_total_capacity.pack())
        .lock(owned_owner)
        .type_(Some(dao).pack())
        .build();

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .input(CellInput::new_builder().previous_output(deposit_input).build())
        .outputs(vec![owner_output, owned_output])
        .outputs_data(vec![truncated_bytes(owner_distance_data(1), 1), withdrawal_request_data(1554)].pack())
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_ENCODING);
}

// Scenario: the owner distance output is empty instead of four bytes.
// Expectation: phase 1 rejects the empty distance as an encoding error.
#[test]
fn zero_length_owner_distance_output_is_rejected_as_encoding() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * SHANNONS;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);
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

    let owner_output = CellOutput::new_builder()
        .capacity(200u64.pack())
        .lock(owner_lock.clone())
        .type_(Some(owned_owner.clone()).pack())
        .build();
    let owned_output = CellOutput::new_builder()
        .capacity(deposit_total_capacity.pack())
        .lock(owned_owner)
        .type_(Some(dao).pack())
        .build();

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .input(CellInput::new_builder().previous_output(deposit_input).build())
        .outputs(vec![owner_output, owned_output])
        .outputs_data(vec![Bytes::new(), withdrawal_request_data(1554)].pack())
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_ENCODING);
}

// Scenario: the owner distance has valid leading bytes plus extra trailing data.
// Expectation: phase 1 ignores the trailing bytes and accepts the pair.
#[test]
fn owner_distance_trailing_bytes_are_ignored() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * SHANNONS;
    let (deposit_total_capacity, deposit_header) = deposit_total_capacity_and_header(&ickb_logic, &dao, deposit_amount, 1554);
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

    let owner_output = CellOutput::new_builder()
        .capacity(200u64.pack())
        .lock(owner_lock.clone())
        .type_(Some(owned_owner.clone()).pack())
        .build();
    let owned_output = CellOutput::new_builder()
        .capacity(deposit_total_capacity.pack())
        .lock(owned_owner)
        .type_(Some(dao).pack())
        .build();

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .input(CellInput::new_builder().previous_output(deposit_input).build())
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
        .verify_tx(&tx, MAX_CYCLES)
        .expect("owned_owner should ignore trailing bytes in owner distance data");
}
