use super::*;

// Scenario: an owner cell uses distance `0`, which cannot point to a distinct partner output.
// Expectation: phase 1 rejects the degenerate self-reference.
#[test]
fn zero_distance_owner_output_is_rejected() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let owned_owner = owned_owner_script(&mut context);
    let dao = dao_script(&mut context);

    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((2_000 * SHANNONS).pack())
            .lock(funding_lock)
            .build(),
        Bytes::new(),
    );
    let owned_output = CellOutput::new_builder()
        .capacity((1_000 * SHANNONS).pack())
        .lock(owned_owner.clone())
        .type_(Some(dao).pack())
        .build();
    let owner_output = CellOutput::new_builder()
        .capacity(200u64.pack())
        .lock(owner_lock)
        .type_(Some(owned_owner).pack())
        .build();

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .outputs(vec![owned_output, owner_output])
        .outputs_data(vec![withdrawal_request_data(1554), owner_distance_data(0)].pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_OWNED_OWNER_MISMATCH);
}

// Scenario: a live owner input uses distance `0` during phase 2.
// Expectation: the melt path rejects the self-reference as a mismatch.
#[test]
fn zero_distance_owner_input_is_rejected() {
    let mut context = Context::default();
    let owned_owner = owned_owner_script(&mut context);
    let dao = dao_script(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let deposit_header = gen_header(1554, SYNTHETIC_DEPOSIT_AR, 35, 1000, 1000);
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    let shared_tx_hash = Byte32::from_slice(&[8u8; 32]).expect("shared tx hash");
    let owned_input = OutPoint::new(shared_tx_hash.clone(), 0);
    let owner_input = OutPoint::new(shared_tx_hash, 1);

    context.create_cell_with_out_point(
        owned_input.clone(),
        CellOutput::new_builder()
            .capacity(123_456_780_000u64.pack())
            .lock(owned_owner.clone())
            .type_(Some(dao).pack())
            .build(),
        withdrawal_request_data(1554),
    );
    context.create_cell_with_out_point(
        owner_input.clone(),
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(owner_lock.clone())
            .type_(Some(owned_owner).pack())
            .build(),
        owner_distance_data(0),
    );
    link_cell_to_header(&mut context, &owned_input, &withdraw_header);
    context.insert_header(deposit_header.clone());
    let witness = header_dep_index_witness(1);

    let tx = TransactionBuilder::default()
        .input(
            CellInput::new_builder()
                .previous_output(owned_input)
                .since(0x2003e800000002f4u64.pack())
                .build(),
        )
        .input(CellInput::new_builder().previous_output(owner_input).build())
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

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_OWNED_OWNER_MISMATCH);
}

// Scenario: the owned cell uses DAO deposit bytes instead of withdrawal-request bytes.
// Expectation: Owned Owner rejects the cell because it is not wrapping a withdrawal request.
#[test]
fn non_withdrawal_owned_cell_is_rejected() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let owned_owner = owned_owner_script(&mut context);
    let dao = dao_script(&mut context);

    let funding_input = context.create_cell(
        CellOutput::new_builder()
            .capacity((2_000 * SHANNONS).pack())
            .lock(funding_lock)
            .build(),
        Bytes::new(),
    );
    let owned_output = CellOutput::new_builder()
        .capacity((1_000 * SHANNONS).pack())
        .lock(owned_owner.clone())
        .type_(Some(dao).pack())
        .build();
    let owner_output = CellOutput::new_builder()
        .capacity(200u64.pack())
        .lock(owner_lock)
        .type_(Some(owned_owner).pack())
        .build();

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .outputs(vec![owned_output, owner_output])
        .outputs_data(vec![dao_deposit_data(), owner_distance_data(-1)].pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_OWNED_OWNER_NOT_WITHDRAW_REQUEST);
}

// A lock-only Owned Owner look-alike can be created with arbitrary type data, but the first spend fails once Owned Owner actually classifies the cell.
#[test]
fn lock_only_owned_owner_non_dao_output_can_be_created_but_not_spent() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let owned_owner = owned_owner_script(&mut context);
    let helper_type = helper_type_script(&mut context);
    let funding_input = context.create_cell(CellOutput::new_builder().capacity(500u64.pack()).lock(funding_lock).build(), Bytes::new());

    let create_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .output(
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(owned_owner.clone())
                .type_(Some(helper_type.clone()).pack())
                .build(),
        )
        .output_data(Bytes::new().pack())
        .build();
    let create_tx = context.complete_tx(create_tx);
    context.verify_tx(&create_tx, MAX_CYCLES).expect("lock-only owned_owner misuse cell can be created because output locks do not execute");

    let forged_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(owned_owner)
            .type_(Some(helper_type).pack())
            .build(),
        Bytes::new(),
    );
    let spend_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(forged_out_point).build())
        .output(CellOutput::new_builder().capacity(200u64.pack()).lock(always_success_lock(&mut context)).build())
        .output_data(Bytes::new().pack())
        .build();
    let spend_tx = context.complete_tx(spend_tx);
    let err = context.verify_tx(&spend_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_OWNED_OWNER_NOT_WITHDRAW_REQUEST);
}

// The same creation-time gap allows a lock-only DAO-shaped look-alike, but the later spend still fails because it is not a withdrawal request pair.
#[test]
fn lock_only_owned_owner_dao_deposit_output_can_be_created_but_not_spent() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let owned_owner = owned_owner_script(&mut context);
    let dao = dao_script(&mut context);
    let funding_input = context.create_cell(CellOutput::new_builder().capacity((2_000 * SHANNONS).pack()).lock(funding_lock).build(), Bytes::new());

    let create_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .output(
            CellOutput::new_builder()
                .capacity((1_000 * SHANNONS).pack())
                .lock(owned_owner.clone())
                .type_(Some(dao.clone()).pack())
                .build(),
        )
        .output_data(dao_deposit_data().pack())
        .build();
    let create_tx = context.complete_tx(create_tx);
    context.verify_tx(&create_tx, MAX_CYCLES).expect("DAO deposit with owned_owner only on the output lock can be created");

    let forged_out_point = context.create_cell(
        CellOutput::new_builder()
            .capacity((1_000 * SHANNONS).pack())
            .lock(owned_owner)
            .type_(Some(dao).pack())
            .build(),
        dao_deposit_data(),
    );
    let spend_tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(forged_out_point).build())
        .output(CellOutput::new_builder().capacity((1_000 * SHANNONS).pack()).lock(always_success_lock(&mut context)).build())
        .output_data(Bytes::new().pack())
        .build();
    let spend_tx = context.complete_tx(spend_tx);
    let err = context.verify_tx(&spend_tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_OWNED_OWNER_NOT_WITHDRAW_REQUEST);
}

// Scenario: the same script is used as both lock and type on one cell.
// Expectation: Owned Owner rejects this explicit script misuse immediately.
#[test]
fn cell_using_owned_owner_as_both_lock_and_type_is_rejected() {
    let mut context = Context::default();
    let funding_lock = always_success_lock(&mut context);
    let owned_owner = owned_owner_script(&mut context);

    let funding_input = context.create_cell(CellOutput::new_builder().capacity(500u64.pack()).lock(funding_lock).build(), Bytes::new());
    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(funding_input).build())
        .output(
            CellOutput::new_builder()
                .capacity(200u64.pack())
                .lock(owned_owner.clone())
                .type_(Some(owned_owner).pack())
                .build(),
        )
        .output_data(Bytes::from(vec![0u8; 4]).pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_OWNED_OWNER_SCRIPT_MISUSE);
}
