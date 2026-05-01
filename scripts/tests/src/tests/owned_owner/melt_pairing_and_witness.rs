use super::*;

// Scenario: the owner input carries a valid distance plus trailing bytes during phase 2.
// Expectation: melt accepts the input because the leading distance still decodes correctly.
#[test]
fn melt_accepts_owner_distance_trailing_bytes_on_input() {
    let mut context = Context::default();
    let owned_owner = owned_owner_script(&mut context);
    let dao = dao_script(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let deposit_header = gen_header(1554, SYNTHETIC_DEPOSIT_AR, 35, 1000, 1000);
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    let shared_tx_hash = Byte32::from_slice(&[7u8; 32]).expect("shared tx hash");
    let owned_input = OutPoint::new(shared_tx_hash.clone(), 0);
    let owner_input = OutPoint::new(shared_tx_hash, 1);
    let owned_input_output = CellOutput::new_builder()
        .capacity(123_456_780_000u64.pack())
        .lock(owned_owner.clone())
        .type_(Some(dao).pack())
        .build();
    context.create_cell_with_out_point(
        owned_input.clone(),
        owned_input_output.clone(),
        withdrawal_request_data(1554),
    );
    context.create_cell_with_out_point(
        owner_input.clone(),
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(owner_lock.clone())
            .type_(Some(owned_owner).pack())
            .build(),
        owner_distance_data_with_trailing_bytes(-1, &[0xaa, 0xbb]),
    );
    link_cell_to_header(&mut context, &owned_input, &withdraw_header);
    context.insert_header(deposit_header.clone());
    let witness = header_dep_index_witness(1);
    let exact_capacity = dao_maximum_withdraw_capacity(
        &owned_input_output,
        withdrawal_request_data(1554).len(),
        SYNTHETIC_DEPOSIT_AR,
        SYNTHETIC_WITHDRAW_AR,
    );

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
                .capacity(exact_capacity.pack())
                .lock(owner_lock)
                .build(),
        )
        .output_data(Bytes::new().pack())
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_header.hash())
        .witness(witness.pack())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("owned_owner melt accepts trailing bytes after a valid input owner distance");
}

// Scenario: a canonical Owned Owner withdrawal pair is melted with the correct witness and headers.
// Expectation: the normal phase 2 claim succeeds.
#[test]
fn valid_melt_pair_passes() {
    let mut context = Context::default();
    let owned_owner = owned_owner_script(&mut context);
    let dao = dao_script(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let deposit_header = gen_header(1554, SYNTHETIC_DEPOSIT_AR, 35, 1000, 1000);
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    let shared_tx_hash = Byte32::from_slice(&[6u8; 32]).expect("shared tx hash");
    let owned_input = OutPoint::new(shared_tx_hash.clone(), 0);
    let owner_input = OutPoint::new(shared_tx_hash, 1);
    let owned_input_output = CellOutput::new_builder()
        .capacity(123_456_780_000u64.pack())
        .lock(owned_owner.clone())
        .type_(Some(dao).pack())
        .build();
    context.create_cell_with_out_point(
        owned_input.clone(),
        owned_input_output.clone(),
        withdrawal_request_data(1554),
    );
    context.create_cell_with_out_point(
        owner_input.clone(),
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(owner_lock.clone())
            .type_(Some(owned_owner).pack())
            .build(),
        owner_distance_data(-1),
    );
    link_cell_to_header(&mut context, &owned_input, &withdraw_header);
    context.insert_header(deposit_header.clone());
    let witness = header_dep_index_witness(1);
    let exact_capacity = dao_maximum_withdraw_capacity(
        &owned_input_output,
        withdrawal_request_data(1554).len(),
        SYNTHETIC_DEPOSIT_AR,
        SYNTHETIC_WITHDRAW_AR,
    );

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
                .capacity(exact_capacity.pack())
                .lock(owner_lock)
                .build(),
        )
        .output_data(Bytes::new().pack())
        .header_dep(withdraw_header.hash())
        .header_dep(deposit_header.hash())
        .witness(witness.pack())
        .build();

    let tx = context.complete_tx(tx);
    context
        .verify_tx(&tx, MAX_CYCLES)
        .expect("owned_owner should accept a valid melt pair");
}

// Scenario: the witness points to the wrong deposit header dep index.
// Expectation: phase 2 rejects the claim because the owner pair is bound to the wrong header metadata.
#[test]
fn melt_rejects_misbound_deposit_header_index() {
    let mut context = Context::default();
    let owned_owner = owned_owner_script(&mut context);
    let dao = dao_script(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let deposit_header = gen_header(1554, SYNTHETIC_DEPOSIT_AR, 35, 1000, 1000);
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    let shared_tx_hash = Byte32::from_slice(&[7u8; 32]).expect("shared tx hash");
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
        owner_distance_data(-1),
    );
    link_cell_to_header(&mut context, &owned_input, &withdraw_header);
    context.insert_header(deposit_header.clone());

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
        .witness(header_dep_index_witness(0).pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_DAO_INVALID_WITHDRAW_BLOCK);
}

// Scenario: the witness input_type is too short to hold a u64 header index.
// Expectation: witness decoding fails before the claim can proceed.
#[test]
fn melt_rejects_short_header_dep_index_witness() {
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
            .type_(Some(dao.clone()).pack())
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
        owner_distance_data(-1),
    );
    link_cell_to_header(&mut context, &owned_input, &withdraw_header);
    context.insert_header(deposit_header.clone());

    let witness = witness_with_input_type(truncated_bytes(Bytes::from(1u64.to_le_bytes().to_vec()), 4));
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
    assert_script_error(err, -11);
}

// Scenario: the witness places the header index in output_type instead of input_type.
// Expectation: phase 2 rejects the misplaced witness encoding.
#[test]
fn melt_rejects_header_dep_index_witness_in_output_type() {
    let mut context = Context::default();
    let owned_owner = owned_owner_script(&mut context);
    let dao = dao_script(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let deposit_header = gen_header(1554, SYNTHETIC_DEPOSIT_AR, 35, 1000, 1000);
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    let shared_tx_hash = Byte32::from_slice(&[81u8; 32]).expect("shared tx hash");
    let owned_input = OutPoint::new(shared_tx_hash.clone(), 0);
    let owner_input = OutPoint::new(shared_tx_hash, 1);

    context.create_cell_with_out_point(
        owned_input.clone(),
        CellOutput::new_builder()
            .capacity(123_456_780_000u64.pack())
            .lock(owned_owner.clone())
            .type_(Some(dao.clone()).pack())
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
        owner_distance_data(-1),
    );
    link_cell_to_header(&mut context, &owned_input, &withdraw_header);
    context.insert_header(deposit_header.clone());

    let witness = witness_with_output_type(Bytes::from(1u64.to_le_bytes().to_vec()));
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
    assert_script_error(err, -11);
}

// Scenario: the owner input distance is truncated during phase 2.
// Expectation: melt rejects the malformed owner cell as an encoding error.
#[test]
fn melt_rejects_truncated_owner_distance_on_input() {
    let mut context = Context::default();
    let owned_owner = owned_owner_script(&mut context);
    let dao = dao_script(&mut context);
    let owner_lock = named_always_success_lock(&mut context, b"owner");
    let deposit_header = gen_header(1554, SYNTHETIC_DEPOSIT_AR, 35, 1000, 1000);
    let withdraw_header = gen_header(2_000_610, SYNTHETIC_WITHDRAW_AR, 575, 2_000_000, 1100);
    let shared_tx_hash = Byte32::from_slice(&[9u8; 32]).expect("shared tx hash");
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
        truncated_bytes(owner_distance_data(-1), 1),
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
    assert_script_error(err, ERROR_ENCODING);
}

// Scenario: two separate Owned Owner pairs are consumed with the owner cells deliberately swapped.
// Expectation: phase 2 rejects the cross-pair spend as a mismatch.
#[test]
fn pairs_cannot_be_swapped_during_melt() {
    let mut context = Context::default();
    let owned_owner = owned_owner_script(&mut context);
    let dao = dao_script(&mut context);
    let user1_lock = named_always_success_lock(&mut context, b"user1");
    let user2_lock = named_always_success_lock(&mut context, b"user2");

    let owned1 = context.create_cell(
        CellOutput::new_builder()
            .capacity(1000u64.pack())
            .lock(owned_owner.clone())
            .type_(Some(dao.clone()).pack())
            .build(),
        withdrawal_request_data(1554),
    );
    let owner1 = context.create_cell(
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(user1_lock.clone())
            .type_(Some(owned_owner.clone()).pack())
            .build(),
        owner_distance_data(1),
    );
    let owned2 = context.create_cell(
        CellOutput::new_builder()
            .capacity(1000u64.pack())
            .lock(owned_owner.clone())
            .type_(Some(dao).pack())
            .build(),
        withdrawal_request_data(1555),
    );
    let owner2 = context.create_cell(
        CellOutput::new_builder()
            .capacity(200u64.pack())
            .lock(user2_lock.clone())
            .type_(Some(owned_owner).pack())
            .build(),
        owner_distance_data(-1),
    );

    let tx = TransactionBuilder::default()
        .input(CellInput::new_builder().previous_output(owned1).build())
        .input(CellInput::new_builder().previous_output(owner2).build())
        .input(CellInput::new_builder().previous_output(owned2).build())
        .input(CellInput::new_builder().previous_output(owner1).build())
        .outputs(vec![
            CellOutput::new_builder().capacity(600u64.pack()).lock(user1_lock).build(),
            CellOutput::new_builder().capacity(600u64.pack()).lock(user2_lock).build(),
        ])
        .outputs_data(vec![Bytes::new(), Bytes::new()].pack())
        .build();

    let tx = context.complete_tx(tx);
    let err = context.verify_tx(&tx, MAX_CYCLES).unwrap_err();
    assert_script_error(err, ERROR_OWNED_OWNER_MISMATCH);
}
