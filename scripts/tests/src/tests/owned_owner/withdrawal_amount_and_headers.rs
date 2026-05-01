use super::*;

// Withdrawal data-shape and header-dependency checks.

// Scenario: the xUDT amount input contains the correct 16-byte amount plus trailing bytes.
// Expectation: Owned Owner reads the prefix amount and accepts the withdrawal.
#[test]
fn withdrawal_accepts_xudt_input_with_trailing_bytes() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
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

    let mut udt_input_data = udt_data(u128::from(deposit_amount)).to_vec();
    udt_input_data.push(0xaa);
    let udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(udt_input_data.len() as u64).pack())
            .lock(user_lock.clone())
            .type_(Some(xudt).pack())
            .build(),
        Bytes::from(udt_input_data),
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
                .lock(user_lock)
                .type_(Some(owned_owner).pack())
                .build(),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), owner_distance_data(-1)].pack())
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("withdrawal should accept xudt input with trailing bytes");
}

// Scenario: the xUDT input data is shorter than the required 16-byte amount.
// Expectation: amount decoding fails with an encoding error.
#[test]
fn withdrawal_rejects_short_xudt_input_data() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
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
            .capacity(capacity_for_data(8).pack())
            .lock(user_lock.clone())
            .type_(Some(xudt).pack())
            .build(),
        truncated_bytes(udt_data(u128::from(deposit_amount)), 8),
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
                .lock(user_lock)
                .type_(Some(owned_owner).pack())
                .build(),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), owner_distance_data(-1)].pack())
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_ENCODING);
}

// Scenario: the xUDT input data is completely empty.
// Expectation: amount decoding fails with the same encoding error as other truncated inputs.
#[test]
fn withdrawal_rejects_zero_length_xudt_input_data() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
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
            .capacity(capacity_for_data(0).pack())
            .lock(user_lock.clone())
            .type_(Some(xudt).pack())
            .build(),
        Bytes::new(),
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
                .lock(user_lock)
                .type_(Some(owned_owner).pack())
                .build(),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), owner_distance_data(-1)].pack())
        .header_dep(deposit_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_ENCODING);
}

// Scenario: the transaction omits the deposit header dep entirely.
// Expectation: Owned Owner cannot resolve the deposit metadata and rejects the withdrawal.
#[test]
fn withdrawal_without_deposit_header_dep_is_rejected() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
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
            .lock(user_lock.clone())
            .type_(Some(xudt).pack())
            .build(),
        udt_data(u128::from(deposit_amount)),
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
                .lock(user_lock)
                .type_(Some(owned_owner).pack())
                .build(),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), owner_distance_data(-1)].pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_ITEM_MISSING);
}

// Scenario: two DAO deposits from different headers are withdrawn together with matching header deps.
// Expectation: the batch succeeds because each withdrawal request can resolve its own deposit header.
#[test]
fn withdrawal_with_two_deposits_from_distinct_headers_passes() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let first_amount = 1_000 * SHANNONS;
    let second_amount = 1_200 * SHANNONS;
    let first_total_capacity = deposit_capacity(&ickb_logic, &dao, 8, first_amount);
    let second_total_capacity = deposit_capacity(&ickb_logic, &dao, 8, second_amount);
    let first_header = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);
    let second_header = gen_header(1555, GENESIS_AR as u64, 35, 1000, 1000);
    let first_deposit = context.create_cell(
        CellOutput::new_builder()
            .capacity(first_total_capacity.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    let second_deposit = context.create_cell(
        CellOutput::new_builder()
            .capacity(second_total_capacity.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    link_cell_to_header(&mut context, &first_deposit, &first_header);
    link_cell_to_header(&mut context, &second_deposit, &second_header);
    let udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(user_lock.clone())
            .type_(Some(xudt).pack())
            .build(),
        udt_data(u128::from(first_amount + second_amount)),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(first_deposit).build())
        .input(CellInput::new_builder().previous_output(second_deposit).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(first_total_capacity.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(second_total_capacity.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(user_lock.clone())
                .type_(Some(owned_owner.clone()).pack())
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
                withdrawal_request_data(1555),
                owner_distance_data(-2),
                owner_distance_data(-2),
            ]
            .pack(),
        )
        .header_dep(first_header.hash())
        .header_dep(second_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("withdrawal should support multiple deposits from distinct headers");
}

// Scenario: one of two deposit headers is omitted from a multi-deposit batch.
// Expectation: the entire batch is rejected because one withdrawal request is missing its header dependency.
#[test]
fn withdrawal_with_one_missing_deposit_header_dep_is_rejected() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let first_amount = 1_000 * SHANNONS;
    let second_amount = 1_200 * SHANNONS;
    let first_total_capacity = deposit_capacity(&ickb_logic, &dao, 8, first_amount);
    let second_total_capacity = deposit_capacity(&ickb_logic, &dao, 8, second_amount);
    let first_header = gen_header(1554, GENESIS_AR as u64, 35, 1000, 1000);
    let second_header = gen_header(1555, GENESIS_AR as u64, 35, 1000, 1000);
    let first_deposit = context.create_cell(
        CellOutput::new_builder()
            .capacity(first_total_capacity.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    let second_deposit = context.create_cell(
        CellOutput::new_builder()
            .capacity(second_total_capacity.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    link_cell_to_header(&mut context, &first_deposit, &first_header);
    link_cell_to_header(&mut context, &second_deposit, &second_header);
    let udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(user_lock.clone())
            .type_(Some(xudt).pack())
            .build(),
        udt_data(u128::from(first_amount + second_amount)),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(first_deposit).build())
        .input(CellInput::new_builder().previous_output(second_deposit).build())
        .input(CellInput::new_builder().previous_output(udt_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(first_total_capacity.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(second_total_capacity.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(user_lock.clone())
                .type_(Some(owned_owner.clone()).pack())
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
                withdrawal_request_data(1555),
                owner_distance_data(-2),
                owner_distance_data(-2),
            ]
            .pack(),
        )
        .header_dep(first_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_ITEM_MISSING);
}

// Scenario: the deposit header is present but its accumulated rate is zero.
// Expectation: the contract panics on invalid DAO math instead of accepting malformed metadata.
#[test]
fn withdrawal_with_zero_accumulated_rate_deposit_header_is_rejected() {
    let mut context = Context::default();
    let user_lock = always_success_lock(&mut context);
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let deposit_amount = 1_000 * SHANNONS;
    let deposit_total_capacity = deposit_capacity(&ickb_logic, &dao, 8, deposit_amount);
    let malformed_header = gen_header(1554, 0, 35, 1000, 1000);
    let deposit_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_total_capacity.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    link_cell_to_header(&mut context, &deposit_input, &malformed_header);
    let udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(user_lock.clone())
            .type_(Some(xudt).pack())
            .build(),
        udt_data(u128::from(deposit_amount)),
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
                .lock(user_lock)
                .type_(Some(owned_owner).pack())
                .build(),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), owner_distance_data(-1)].pack())
        .header_dep(malformed_header.hash())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_SCRIPT_PANIC);
}

// Scenario: the withdrawal amount is above the soft-cap boundary.
// Expectation: the naive uncapped burn is rejected, while the discounted soft-capped burn amount succeeds.
#[test]
fn withdrawal_applies_soft_cap_discount_above_boundary() {
    let mut context = Context::default();
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let (ickb_logic, owned_owner, dao, xudt) = ickb_logic_owned_owner_dao_and_xudt_scripts(&mut context);

    let amount = 100_001 * SHANNONS;
    let deposit_total_capacity = deposit_capacity(&ickb_logic, &dao, 8, amount);
    let deposit_header = gen_header(1554, GENESIS_AR, 35, 1000, 1000);
    let deposit_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(deposit_total_capacity.pack())
            .lock(ickb_logic.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        dao_deposit_data(),
    );
    link_cell_to_header(&mut context, &deposit_input, &deposit_header);

    let naive_udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(owner_lock.clone())
            .type_(Some(xudt.clone()).pack())
            .build(),
        udt_data(u128::from(amount)),
    );
    let naive_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit_input.clone()).build())
        .input(CellInput::new_builder().previous_output(naive_udt_input).build())
        .outputs(vec![
            CellOutput::new_builder()
                .capacity(deposit_total_capacity.pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(owner_lock.clone())
                .type_(Some(owned_owner.clone()).pack())
                .build(),
        ])
        .outputs_data(vec![withdrawal_request_data(1554), owner_distance_data(-1)].pack())
        .header_dep(deposit_header.hash())
        .build();
    let naive_tx = context.complete_tx(naive_tx);
    let err = context.verify_tx(&naive_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_AMOUNT_MISMATCH);

    let exact_udt_input = context.create_cell(
        CellOutput::new_builder()
            .capacity(capacity_for_data(16).pack())
            .lock(owner_lock.clone())
            .type_(Some(xudt).pack())
            .build(),
        udt_data(soft_capped_ickb(amount, GENESIS_AR)),
    );
    let exact_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(deposit_input).build())
        .input(CellInput::new_builder().previous_output(exact_udt_input).build())
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
        .outputs_data(vec![withdrawal_request_data(1554), owner_distance_data(-1)].pack())
        .header_dep(deposit_header.hash())
        .build();
    let exact_tx = context.complete_tx(exact_tx);
    context
        .verify_tx(&exact_tx, MAX_CYCLES)
        .expect("withdrawal should require the soft-capped iCKB amount above the 100k boundary");
}
